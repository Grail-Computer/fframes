#include <stdio.h>
#include <libavutil/opt.h>
#include <libavutil/timestamp.h>

const char *av_error_to_string(int error_code)
{
  return av_err2str(error_code);
}

#include <stdio.h>
#include "libavcodec/avcodec.h"
#include "libavformat/avformat.h"

void make_stereo_layout_channel(AVCodecContext *c, AVCodec *codec)
{

  c->channels = av_get_channel_layout_nb_channels(c->channel_layout);
  c->channel_layout = AV_CH_LAYOUT_MONO;
  if (codec->channel_layouts)
  {
    int i;
    c->channel_layout = codec->channel_layouts[0];
    for (i = 0; codec->channel_layouts[i]; i++)
    {
      if (codec->channel_layouts[i] == AV_CH_LAYOUT_MONO)
        c->channel_layout = AV_CH_LAYOUT_MONO;
    }
  }

  c->channels = av_get_channel_layout_nb_channels(c->channel_layout);
}

AVFormatContext *i_fmt_ctx;
AVStream *i_video_stream;
AVFormatContext *o_fmt_ctx;
AVStream *o_video_stream;

int concat_files(const char *output)
{

  const char *files[10];
  files[0] = "some-0.mp4";
  files[1] = "some-1.mp4";
  files[2] = "some-2.mp4";
  files[3] = "some-3.mp4";
  files[4] = "some-4.mp4";
  files[5] = "some-5.mp4";
  files[6] = "some-6.mp4";
  files[7] = "some-7.mp4";
  files[8] = "some-8.mp4";
  files[9] = "some-9.mp4";

  printf("%s", files[0]);

  /* should set to NULL so that avformat_open_input() allocate a new one */
  i_fmt_ctx = NULL;
  if (avformat_open_input(&i_fmt_ctx, files[0], NULL, NULL) != 0)
  {
    fprintf(stderr, "could not open input file\n");
    return -1;
  }

  if (avformat_find_stream_info(i_fmt_ctx, NULL) < 0)
  {
    fprintf(stderr, "could not find stream info\n");
    return -1;
  }

  // av_dump_format(i_fmt_ctx, 0, argv[1], 0);

  /* find first video stream */
  for (unsigned i = 0; i < i_fmt_ctx->nb_streams; i++)
    if (i_fmt_ctx->streams[i]->codecpar->codec_type == AVMEDIA_TYPE_VIDEO)
    {
      i_video_stream = i_fmt_ctx->streams[i];
      break;
    }
  if (i_video_stream == NULL)
  {
    fprintf(stderr, "didn't find any video stream\n");
    return -1;
  }

  avformat_alloc_output_context2(&o_fmt_ctx, NULL, NULL, output);

  /*
   * since all input files are supposed to be identical (framerate, dimension, color format, ...)
   * we can safely set output codec values from first input file
   */
  o_video_stream = avformat_new_stream(o_fmt_ctx, 0);
  {
    AVCodecContext *c;
    c = o_video_stream->codecpar;
    c->bit_rate = i_fmt_ctx->bit_rate;
    c->codec_id = i_video_stream->codecpar->codec_id;
    c->codec_type = i_video_stream->codecpar->codec_type;
    c->time_base = i_video_stream->time_base;
    o_video_stream->time_base = c->time_base;

    c->width = i_video_stream->codecpar->width;
    c->height = i_video_stream->codecpar->height;
    AVCodec *avc = avcodec_find_decoder(i_video_stream->codecpar->codec_id);
    AVCodecContext *avcc = avcodec_alloc_context3(avc);
    c->pix_fmt = avcc->pix_fmt;

    c->flags = avcc->flags;
    c->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;

    c->me_range = avcc->me_range;
    c->max_qdiff = avcc->max_qdiff;
    c->gop_size = 12;

    c->qmin = avcc->qmin;
    c->qmax = avcc->qmax;

    c->qcompress = avcc->qcompress;

    c->extradata = i_video_stream->codecpar->extradata;
    c->extradata_size = i_video_stream->codecpar->extradata_size;

    avcodec_parameters_from_context(o_video_stream->codecpar, c);
  }

  if (o_fmt_ctx->oformat->flags & AVFMT_GLOBALHEADER != 0)
  {
    AVOutputFormat* fl = o_fmt_ctx->oformat;
    fl->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
    o_fmt_ctx->oformat = fl;
  }

  // o_video_stream->codecpar = i_video_stream->codecpar;
  avio_open(&o_fmt_ctx->pb, output, AVIO_FLAG_WRITE);

  /* yes! this is redundant */
  avformat_close_input(&i_fmt_ctx);
  avformat_write_header(o_fmt_ctx, NULL);

  int last_pts = 0;
  int last_dts = 0;
  int64_t start_time = 0;
  int files_len = sizeof(files) / sizeof(files[0]);

  for (int i = 0; i < files_len; i++)
  {
    i_fmt_ctx = NULL;

    if (avformat_open_input(&i_fmt_ctx, files[i], NULL, NULL) != 0)
    {
      fprintf(stderr, "could not open input file\n");
      return -1;
    }

    if (avformat_find_stream_info(i_fmt_ctx, NULL) < 0)
    {
      fprintf(stderr, "could not find stream info\n");
      return -1;
    }
    av_dump_format(i_fmt_ctx, 0, files[i], 0);
    /* we only use first video stream of each input file */
    i_video_stream = NULL;
    for (unsigned s = 0; s < i_fmt_ctx->nb_streams; s++)
      if (i_fmt_ctx->streams[s]->codecpar->codec_type == AVMEDIA_TYPE_VIDEO)
      {
        i_video_stream = i_fmt_ctx->streams[s];
        break;
      }

    if (i_video_stream == NULL)
    {
      fprintf(stderr, "didn't find any video stream\n");
      return -1;
    }

    int64_t pts, dts, delta;

    while (1)
    {
      AVPacket i_pkt;
      av_init_packet(&i_pkt);
      i_pkt.size = 0;
      i_pkt.data = NULL;
      if (av_read_frame(i_fmt_ctx, &i_pkt) < 0)
        break;

      i_pkt.flags |= AV_PKT_FLAG_KEY;

      // This calculates the delta in pts based on the duration when this file must be appeared
      delta = av_rescale_q(start_time,
                           AV_TIME_BASE_Q,
                           o_video_stream->time_base);
      i_pkt.pts += delta;
      i_pkt.dts += delta;

      if (i != 0 && i_pkt.dts <= last_dts)
      {
        // just do what ffmpeg is doing when we meet same pts (happens if first frames dts is negative)
        // just make +1 and hope it won't broke the final video
        i_pkt.dts = last_dts + 1;
      }

      last_pts = i_pkt.pts;
      last_dts = i_pkt.dts;

      av_packet_rescale_ts(&i_pkt, i_video_stream->time_base, o_video_stream->time_base);
      av_interleaved_write_frame(o_fmt_ctx, &i_pkt);
    }

    start_time += i_fmt_ctx->duration;

    avformat_close_input(&i_fmt_ctx);
  }

  av_write_trailer(o_fmt_ctx);

  AVCodec *avc = avcodec_find_decoder(o_fmt_ctx->streams[0]->codecpar->codec_id);
  AVCodecContext *avcc = avcodec_alloc_context3(avc);
  avcodec_close(avcc);
  av_freep(avcc);
  av_freep(&o_fmt_ctx->streams[0]);

  avio_close(o_fmt_ctx->pb);
  av_free(o_fmt_ctx);
  avformat_free_context(o_fmt_ctx);

  return 0;
}

void fill_yuv_image(AVFrame *pict, int frame_index,
                    int width, int height)
{
  int x, y, i;

  i = frame_index;

  /* Y */
  for (y = 0; y < height; y++)
    for (x = 0; x < width; x++)
      pict->data[0][y * pict->linesize[0] + x] = x + y + i * 3;

  /* Cb and Cr */
  for (y = 0; y < height / 2; y++)
  {
    for (x = 0; x < width / 2; x++)
    {
      pict->data[1][y * pict->linesize[1] + x] = 128 + y + i * 2;
      pict->data[2][y * pict->linesize[2] + x] = 64 + x + i * 5;
    }
  }
}

void log_packet(AVStream *stream, AVPacket *pkt)
{
  printf("pts:%s pts_time:%s dts:%s dts_time:%s duration:%s duration_time:%s stream_index:%d\n",
         av_ts2str(pkt->pts), av_ts2timestr(pkt->pts, &stream->time_base),
         av_ts2str(pkt->dts), av_ts2timestr(pkt->dts, &stream->time_base),
         av_ts2str(pkt->duration), av_ts2timestr(pkt->duration, &stream->time_base),
         pkt->stream_index);
}
