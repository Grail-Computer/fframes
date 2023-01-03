use fframes::{
    animation::{self, AnimationRuntime},
    prettify_spectrum, AudioMap, AudioTimestamp, Svgr,
};
pub use fframes::{
    audio_data, audio_window_functions, fframes_context, frame::Frame, subtitles, video::Video,
};
use lazy_static::lazy_static;
use svgr_macro::{self, svgr};

const SPRING: animation::Easing = animation::Easing::Spring2(1.85, 130., 16.);
const BAR_SIZE: usize = 30;
const BAR_PADDING: usize = 20;
const SPECTRUM_WIDTH: usize = 16 * (BAR_SIZE + BAR_PADDING) - BAR_PADDING;

lazy_static! {
    static ref SPRING_RUNTIME: AnimationRuntime = AnimationRuntime::from_easing(&SPRING);
}

#[derive(Debug)]
pub struct GooseVideo {
    pub audio_track: &'static str,
}

pub enum TextAlign { 
    Left, 
    Center, 
    Right,
}

struct BreakLinesOpts<'a> {
    approx_char_width: usize,
    width: usize,
    line_height: f32,
    x: &'a str,
    y: &'a str,
    align: TextAlign
}

fn break_lines_wip(
    value: &str,
    BreakLinesOpts {
        approx_char_width,
        width,
        line_height,
        x,
        y,
        align
    }: BreakLinesOpts,
) -> Svgr {
    let mut structure = vec![(String::new(), 0usize)];
    
    value.split_whitespace().for_each(|word| {
        let word_width = word.len() * approx_char_width; 
        let (last_line, last_line_width) = structure.last_mut().unwrap();

        if *last_line_width + approx_char_width + word_width > width {
            structure.push((String::from(word), word_width));
        } else {
            if *last_line_width != 0 { 
                last_line.push(' ');
                *last_line_width += approx_char_width;
            }

            last_line.push_str(word);

            *last_line_width += word_width;
        }
    });

    structure
        .into_iter()
        .enumerate()
        .map(|(index, (line, line_width))| { 
            let dx = match align {
                TextAlign::Left => 0,
                TextAlign::Center => (width - line_width) / 2,
                TextAlign::Right => width - line_width
            };

            svgr!(<tspan x={x} y={y} dx={dx} dy={format!("{}em", index as f32 * line_height)}>{line}</tspan>)
        })
        .collect()
}

impl Video for GooseVideo {
    const FPS: usize = 60;
    const WIDTH: usize = 1080;
    const HEIGHT: usize = 1920;
    const DURATION: fframes::Duration = fframes::Duration::FromAudio("thought.mp3"); 

    fn audio(&self) -> AudioMap {
        use AudioTimestamp::{Eof, Second};

        AudioMap::from([("thought.mp3", (Second(0), Eof))])
    }

    fn render_frame(&self, mut frame: Frame, ctx: &fframes_context::FFramesContext) -> Svgr {
        let subtitles = ctx.get_subtitles("thought.vtt");
        let audio_visualization = frame.visualize_audio_frame(audio_data::VisualizeFrameInput {
            audio: ctx.get_audio_data(self.audio_track),
            sample_size: audio_data::SampleSize::S32,
            ctx,
            smooth_level: 4,
            window: None,
        });

        let audio_visualization = prettify_spectrum(audio_visualization.as_slice());

        svgr!(
            <svg width="1080" height="1920" fill="none" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
                <g clip-path="url(#a)">
                    <path fill="#000" d="M0 0h1080v1920H0z"/>
                    <g style="mix-blend-mode:hard-light" filter="url(#b)">
                    <ellipse cx="449.658" cy="652.599" rx="311.126" ry="316.462" transform="rotate(70 449.658 652.599)" fill="#B011E8"/>
                    </g>
                 <g filter="url(#c)"><ellipse rx="208.812" ry="211.997" transform="matrix(.00844 .99996 -.99984 .01802 686.793 1443.34)" fill="#B310FF"/></g><g style="mix-blend-mode:lighten" opacity=".5" filter="url(#d)"><ellipse rx="176.773" ry="179.55" transform="matrix(.00844 .99996 -.99984 .01802 406.049 927.243)" fill="#F90"/></g><g style="mix-blend-mode:hard-light" filter="url(#e)"><ellipse cx="565.155" cy="1157.97" rx="379.483" ry="380.775" transform="rotate(70 565.155 1157.97)" fill="#454ACF"/></g></g>
                 <defs>
                   <filter id="b" x="-66.271" y="140.76" width="1031.86" height="1023.68" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB"><feFlood flood-opacity="0" result="BackgroundImageFix"/><feBlend in="SourceGraphic" in2="BackgroundImageFix" result="shape"/><feGaussianBlur stdDeviation="100" result="effect1_foregroundBlur_3_2"/></filter><filter id="c" x="194.823" y="954.499" width="983.941" height="977.681" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB"><feFlood flood-opacity="0" result="BackgroundImageFix"/><feBlend in="SourceGraphic" in2="BackgroundImageFix" result="shape"/><feGaussianBlur stdDeviation="140" result="effect1_foregroundBlur_3_2"/></filter><filter id="d" x="26.522" y="550.447" width="759.054" height="753.593" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB"><feFlood flood-opacity="0" result="BackgroundImageFix"/><feBlend in="SourceGraphic" in2="BackgroundImageFix" result="shape"/><feGaussianBlur stdDeviation="100" result="effect1_foregroundBlur_3_2"/></filter><filter id="e" x="-15.573" y="578.228" width="1161.46" height="1159.48" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB"><feFlood flood-opacity="0" result="BackgroundImageFix"/><feBlend in="SourceGraphic" in2="BackgroundImageFix" result="shape"/><feGaussianBlur stdDeviation="100" result="effect1_foregroundBlur_3_2"/></filter><clipPath id="a"><path fill="#fff" d="M0 0h1080v1920H0z"/></clipPath>
                 </defs>
                 {
                     audio_visualization.iter().enumerate().map(|(i, value)| {
                         let height= (value * 7.).clamp(30., 400.);
                         svgr!(
                             <rect
                                 x={i * (BAR_SIZE + BAR_PADDING) + (Self::WIDTH - SPECTRUM_WIDTH) / 2}
                                 y={300. - height / 2. }
                                 width={BAR_SIZE}
                                 rx="15"
                                 fill="white"
                                 height={height}
                             />
                         )
                     })
                     .collect::<Vec<_>>()
                 }

                <text
                  font-size="100"
                  fill="white"
                  font-family="JetBrains Mono"
                >
                  {break_lines_wip(subtitles.get_phrase_for_frame(&frame).unwrap_or_default(), BreakLinesOpts { 
                    approx_char_width: 61, 
                    width: 1000, 
                    line_height: 1.2, 
                    x: "40", 
                    y: "32%",
                    align: TextAlign::Center
                  })}
                </text>

                <image
                  width="950"
                  height="950"
                  xlink:href={ctx.get_image_link("goose2.png")}
                  y={1920 - 950}
                  x={1080 / 2 - 400}
                />
                
        </svg>
        )
    }
}
