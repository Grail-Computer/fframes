use ffmpeg_next::sys::*;
use fframes::{FFramesContext, ResolvedAudioMap};
use std::ffi::CString;

use crate::{
    encoder::{Encoder, EncoderFrame},
    ffmpeg_action,
    renderer_error::{AVError, AVResult},
    stream::Stream,
    stream::StreamVariant,
};

unsafe fn open_file_stream(
    filename: &str,
    input_format_ctx: &mut *mut AVFormatContext,
    codec_type: AVMediaType,
) -> AVResult<*mut AVStream> {
    let input_file = CString::new(filename).unwrap();

    ffmpeg_action!(
        avformat_open_input(
            input_format_ctx,
            input_file.as_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        ),
        AVError::CantOpenFile(filename.to_owned())
    );

    ffmpeg_action!(
        avformat_find_stream_info(*input_format_ctx, std::ptr::null_mut()),
        AVError::CantOpenFile(filename.to_owned())
    );

    let streams = std::slice::from_raw_parts_mut(
        (*(*input_format_ctx)).streams,
        (*(*input_format_ctx)).nb_streams as usize,
    );

    let mut input_stream = std::ptr::null_mut();
    for stream in streams {
        let codec = (*stream.to_owned()).codecpar;

        if (*codec).codec_type == codec_type {
            input_stream = *stream;
            break;
        }
    }

    if input_stream.is_null() {
        Err(AVError::MissingVideoStreamInFile(filename.to_owned()))
    } else {
        Ok(input_stream)
    }
}

unsafe fn copy_codec_params(
    codec: *mut AVCodecContext,
    input_format_ctx: *mut AVFormatContext,
    input_video_stream: *mut AVStream,
    output_video_stream: *mut AVStream,
) {
    (*codec).bit_rate = (*input_format_ctx).bit_rate;
    (*codec).codec_id = (*(*input_video_stream).codecpar).codec_id;

    // getting AVCodecContext
    let avc = avcodec_find_decoder((*codec).codec_id);
    let avcc = avcodec_alloc_context3(avc);

    (*codec).codec_type = (*(*input_video_stream).codecpar).codec_type;

    (*codec).time_base = (*input_video_stream).time_base;
    (*output_video_stream).time_base = (*codec).time_base;

    (*codec).width = (*(*input_video_stream).codecpar).width;
    (*codec).height = (*(*input_video_stream).codecpar).height;
    (*codec).pix_fmt = (*avcc).pix_fmt;

    (*codec).flags = (*avcc).flags;
    (*codec).flags |= AV_CODEC_FLAG_GLOBAL_HEADER as i32;

    (*codec).me_range = (*avcc).me_range;
    (*codec).max_qdiff = (*avcc).max_qdiff;
    (*codec).gop_size = (*avcc).gop_size; // maybe hardcode to 12?

    (*codec).qmin = (*avcc).qmin;
    (*codec).qmax = (*avcc).qmax;
    (*codec).qcompress = (*avcc).qcompress;

    (*codec).extradata = (*avcc).extradata;
    (*codec).extradata_size = (*avcc).extradata_size;
    avcodec_parameters_from_context((*output_video_stream).codecpar, codec);
}

pub unsafe fn concat_video_files_with_audio(
    files: &[String],
    output: &str,
    audio_map: Option<&ResolvedAudioMap>,
    ctx: &FFramesContext,
) -> Result<(), AVError> {
    let mut input_format_ctx: *mut AVFormatContext = std::ptr::null_mut();
    let mut output_format_ctx: *mut AVFormatContext = std::ptr::null_mut();

    let input_video_stream = open_file_stream(
        &files[0],
        &mut input_format_ctx,
        AVMediaType::AVMEDIA_TYPE_VIDEO,
    )?;
    let output_file = CString::new(output).unwrap();

    avformat_alloc_output_context2(
        &mut output_format_ctx,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        output_file.as_ptr(),
    );

    let output_video_stream = avformat_new_stream(output_format_ctx, std::ptr::null_mut());
    // getting AVCodecContext
    let avc = avcodec_find_decoder((*(*output_video_stream).codecpar).codec_id);
    let codec = avcodec_alloc_context3(avc);

    let audio_stream = Stream::make_audio(
        44100,
        output_format_ctx,
        "aac",
        (*output_format_ctx).audio_codec_id,
    )?;

    let mut encoder = Encoder {
        video_stream: Stream {
            st: output_video_stream,
            enc: codec,
            variant: StreamVariant::Video,
        },
        audio_stream: Some(audio_stream),
        oc: output_format_ctx,
        b_frames_count: 0,
    };

    copy_codec_params(
        codec,
        input_format_ctx,
        input_video_stream,
        output_video_stream,
    );

    if (*(*output_format_ctx).oformat).flags & AVFMT_GLOBALHEADER != 0 {
        let ofo = (*output_format_ctx).oformat.cast_mut();
        (*ofo).flags |= AV_CODEC_FLAG_GLOBAL_HEADER as i32;
        (*output_format_ctx).oformat = ofo;
    }

    avio_open(
        &mut (*output_format_ctx).pb,
        output_file.as_ptr(),
        AVIO_FLAG_WRITE,
    );

    avformat_close_input(&mut input_format_ctx);
    avformat_write_header(output_format_ctx, std::ptr::null_mut());

    av_dump_format(output_format_ctx, 0, output_file.as_ptr(), 1);

    let mut last_pts = 0;
    let mut last_dts = 0;
    let mut start_time = 0;

    let mut packet = av_packet_alloc();

    for (i, file) in files.iter().enumerate() {
        let mut input_format_ctx = std::ptr::null_mut();

        let input_video_stream =
            open_file_stream(file, &mut input_format_ctx, AVMediaType::AVMEDIA_TYPE_VIDEO)?;

        loop {
            let res = av_read_frame(input_format_ctx, packet);
            if res < 0 {
                break;
            }

            (*packet).flags |= AV_PKT_FLAG_KEY;

            // This calculates the delta in pts based on the duration when this file must be appeared
            let delta = av_rescale_q(start_time, AV_TIME_BASE_Q, (*output_video_stream).time_base);

            (*packet).pts += delta;
            (*packet).dts += delta;

            if i != 0 && (*packet).dts <= last_dts {
                // This can happen if first frames dts is negative
                // just make +1 and hope 🤞 it won't broke in the final video
                (*packet).dts = last_dts + 1
            }
            if i != 0 && (*packet).pts <= last_pts {
                // This can happen if first frames pts is negative
                // just make +1 and hope 🤞 it won't broke in the final video
                (*packet).pts = last_pts + 1
            }

            last_dts = (*packet).dts;
            last_pts = (*packet).pts;

            av_packet_rescale_ts(
                packet,
                (*input_video_stream).time_base,
                (*output_video_stream).time_base,
            );
            av_interleaved_write_frame(output_format_ctx, packet);
        }

        start_time += (*input_format_ctx).duration + 1024;
        avformat_close_input(&mut input_format_ctx);
    }

    fill_audio_stream(&mut encoder, audio_map, ctx)?;

    avcodec_send_frame(codec, std::ptr::null_mut());
    avcodec_send_frame(audio_stream.enc, std::ptr::null_mut());

    av_write_trailer(output_format_ctx);

    avcodec_close(codec);
    avcodec_close(audio_stream.enc);
    audio_stream.free();

    avio_close((*output_format_ctx).pb);

    Ok(())
}

pub unsafe fn fill_audio_stream(
    encoder: &mut Encoder,
    audio_map: Option<&ResolvedAudioMap>,
    ctx: &FFramesContext,
) -> Result<(), AVError> {
    if let (Some(audio_map), Some(audio_stream)) = (audio_map, encoder.audio_stream) {
        let mut audio_frame = EncoderFrame::make(
            &encoder
                .audio_stream
                .ok_or_else(|| AVError::Internal("Missing audio_stream".to_owned()))?,
        );

        let audio_stream_duration = audio_map.calc_stream_duration_in_samples();

        let mut audio_frame_pts = 0usize;
        let frame_size = (*audio_stream.enc).frame_size as usize;

        while audio_frame_pts <= audio_stream_duration {
            let audio_data =
                ctx.get_mixed_audio_data_in_fltp(audio_map, audio_frame_pts, frame_size);

            audio_frame.fill_from_audio_data(audio_frame_pts as i64, audio_data);
            encoder.send_frame(&audio_stream, audio_frame)?;

            audio_frame_pts += frame_size;
        }
    }

    Ok(())
}
