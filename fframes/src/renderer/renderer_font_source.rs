use crate::usvgr::fontdb::{self, Family, Query, Weight};
use crate::{FontFace, FontStretch, FontStyle};
use std::path::Path;
use std::sync::Arc;

pub(crate) struct RendererFont<'a> {
    pub(crate) index: u32,
    // We can not use the ttf_parser::Face directly because it can be a file
    // which in theory not a big deal because "parse" here mostly not doing any data transformations
    pub(crate) data: Arc<dyn AsRef<[u8]> + Send + Sync + 'a>,
}

impl std::fmt::Debug for RendererFont<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RendererFont")
            .field("index", &self.index)
            .finish()
    }
}

impl<'a> crate::FontFace<'a> for RendererFont<'a> {
    fn is_monospaced(&self) -> Option<bool> {
        let face = crate::ttf_parser::Face::parse(self.data.as_ref().as_ref(), self.index).ok()?;
        Some(face.is_monospaced())
    }

    fn resolve_char_width(&self, font_size: usize, char: char) -> Option<usize> {
        let face = crate::ttf_parser::Face::parse(self.data.as_ref().as_ref(), self.index).ok()?;
        let glyph_id = face.glyph_index(char)?;

        Some(
            font_size * face.tables().hmtx?.advance(glyph_id)? as usize
                / face.units_per_em() as usize,
        )
    }

    fn shaped_width(&self, font_size: usize, text: &str) -> Option<usize> {
        let face = rustybuzz::Face::from_slice(self.data.as_ref().as_ref(), self.index)?;
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(text);
        let glyphs = rustybuzz::shape(&face, &[], buffer);
        let units: i64 = glyphs
            .glyph_positions()
            .iter()
            .map(|p| i64::from(p.x_advance))
            .sum();
        let width = units.max(0) as f64 * font_size as f64 / f64::from(face.units_per_em());
        Some(width.round() as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::RendererFont;
    use crate::FontFace;
    use std::sync::Arc;

    #[test]
    fn shaped_width_applies_kerning() {
        let data: &'static [u8] =
            include_bytes!("../../../examples/hello-world/media/DMSans-Medium.ttf");
        let font = RendererFont {
            index: 0,
            data: Arc::new(data),
        };
        let advances: usize = "AVAVAV"
            .chars()
            .map(|c| font.resolve_char_width(1000, c).unwrap())
            .sum();
        let shaped = font.shaped_width(1000, "AVAVAV").unwrap();
        assert!(
            shaped < advances,
            "kerned {shaped} should be narrower than {advances}"
        );
        // no kerning pairs: the shaped width is the advance sum
        let plain: usize = "IIII"
            .chars()
            .map(|c| font.resolve_char_width(1000, c).unwrap())
            .sum();
        assert!(font.shaped_width(1000, "IIII").unwrap().abs_diff(plain) <= 1);
    }
}

#[derive(Debug)]
pub struct RendererFontSource {
    pub(crate) fontdb: fontdb::Database,
}

impl RendererFontSource {
    pub fn as_db_ref(&self) -> &fontdb::Database {
        &self.fontdb
    }

    /// Read the fonts from the folder
    pub fn read_folder(&mut self, path: impl AsRef<Path>) {
        self.fontdb.load_fonts_dir(path);
    }

    /// Loads the fonts installed on the system (see `RenderOptions::load_system_fonts`).
    pub fn load_system_fonts(&mut self) {
        self.fontdb.load_system_fonts();
    }
}

impl<'a> crate::FontSource<'a> for RendererFontSource {
    fn resolve_font(
        &'a self,
        font_name: &str,
        font_weight: u16,
        font_style: FontStyle,
        font_stretch: FontStretch,
    ) -> Option<Box<dyn FontFace<'a> + 'a>> {
        let font_id = self.fontdb.query(&Query {
            families: &[Family::Name(font_name)],
            weight: Weight(font_weight),
            style: match font_style {
                FontStyle::Normal => fontdb::Style::Normal,
                FontStyle::Italic => fontdb::Style::Italic,
                FontStyle::Oblique => fontdb::Style::Oblique,
            },
            stretch: match font_stretch {
                FontStretch::ExtraCondensed => fontdb::Stretch::ExtraCondensed,
                FontStretch::UltraCondensed => fontdb::Stretch::UltraCondensed,
                FontStretch::Condensed => fontdb::Stretch::Condensed,
                FontStretch::SemiCondensed => fontdb::Stretch::SemiCondensed,
                FontStretch::Normal => fontdb::Stretch::Normal,
                FontStretch::SemiExpanded => fontdb::Stretch::SemiExpanded,
                FontStretch::Expanded => fontdb::Stretch::Expanded,
                FontStretch::ExtraExpanded => fontdb::Stretch::ExtraExpanded,
                FontStretch::UltraExpanded => fontdb::Stretch::UltraExpanded,
            },
        })?;

        let (source, index) = self.fontdb.face_source(font_id)?;
        let font_data = match source {
            fontdb::Source::Binary(data) | fontdb::Source::SharedFile(_, data) => data.clone(),
            fontdb::Source::File(path) => Arc::new(std::fs::read(path).ok()?),
        };

        Some(Box::new(RendererFont {
            index,
            data: font_data,
        }))
    }

    fn add_font(&mut self, _filename: String, font_data: Arc<dyn AsRef<[u8]> + Sync + Send>) {
        self.fontdb
            .load_font_source(fontdb::Source::Binary(font_data.clone()));
    }
}
