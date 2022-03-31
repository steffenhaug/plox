//! Text shaping!
use crate::spline::Rect;
use rustybuzz::{self as buzz, Face, GlyphInfo, GlyphPosition, UnicodeBuffer};
use ttf_parser as ttf;

#[derive(Debug)]
pub struct Glyph {
    pub glyph_id: u32,
    pub bbox: Rect,
    pub x: f32,
    pub y: f32,
}

pub fn shape<S>(text: S, face: &Face) -> Vec<Glyph>
where
    S: AsRef<str>,
{
    // TODO: We can save this allocation if we re-use the buffer.
    let mut unicode_buffer = UnicodeBuffer::new();
    unicode_buffer.push_str(text.as_ref());

    let before = std::time::Instant::now();
    let glyph_buffer = buzz::shape(face, &[], unicode_buffer);
    let after = std::time::Instant::now();
    println!("Shaping time: {}ms", (after - before).as_millis());

    let mut x = 0.0;
    let mut y = 0.0;
    let mut glyphs = vec![];
    let em = face.units_per_em() as f32;

    for i in 0..glyph_buffer.len() {
        let GlyphInfo { glyph_id, .. } = glyph_buffer.glyph_infos()[i];

        // Advances: How much to advance *after* drawing the glyph
        let GlyphPosition {
            x_advance,
            y_advance,
            ..
        } = glyph_buffer.glyph_positions()[i];

        // Kerning information in units of 1em.
        let x_advance = x_advance as f32 / em;
        let y_advance = y_advance as f32 / em;

        let bbox = face
            .glyph_bounding_box(ttf::GlyphId(glyph_id as u16))
            // I don't think it is actually safe to do this. fix later.
            .unwrap();

        // Glyph bounding box in units of 1em.
        let x0 = f32::min(bbox.x_min as f32, bbox.x_max as f32) / em;
        let x1 = f32::max(bbox.x_min as f32, bbox.x_max as f32) / em;
        let y0 = f32::min(bbox.y_min as f32, bbox.y_max as f32) / em;
        let y1 = f32::max(bbox.y_min as f32, bbox.y_max as f32) / em;
        let bbox = Rect { x0, x1, y0, y1 };

        glyphs.push(Glyph { glyph_id, bbox, x, y });

        // Advance the cursor in preparation for the next glyph.
        x += x_advance;
        y += y_advance;
    }

    glyphs
}
