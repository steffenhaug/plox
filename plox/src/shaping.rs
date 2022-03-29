//! Text shaping!
use crate::font;
use crate::spline::Spline;
use rustybuzz::{self as buzz, GlyphInfo, GlyphPosition, UnicodeBuffer};
use ttf_parser::GlyphId;

pub fn shape<S>(text: S) -> Spline
where
    S: AsRef<str>,
{
    // TODO: We can save this allocation if we re-use the buffer.
    let mut unicode_buffer = UnicodeBuffer::new();
    unicode_buffer.push_str(text.as_ref());
    let glyph_buffer = buzz::shape(&font::LM_MATH, &[], unicode_buffer);

    let mut spline = Spline::builder();

    // For all glyphs in the buffer,
    let n = glyph_buffer.len();
    for i in 0..n {
        // Advances: How much to advance after drawing the glyph
        // Offset: How much to offset before drawing the glyph.
        //   https://harfbuzz.github.io/a-simple-shaping-example.html
        // The above link has a code example of how to use this info.
        // Most "normal" characters have offset=0, so I'll ignore for
        // now.
        let GlyphInfo { glyph_id, .. } = glyph_buffer.glyph_infos()[i];
        let glyph = GlyphId(glyph_id as u16); // Guaranteed to fit.

        let GlyphPosition {
            x_advance,
            y_advance,
            ..
        } = glyph_buffer.glyph_positions()[i];

        // Now, given a glyph and its (x,y)-advance, we can draw it:
        font::LM_MATH.outline_glyph(glyph, &mut spline);
        spline.advance(x_advance as f32, y_advance as f32);
    }

    let s = spline.build();
    dbg!(s.0.len());
    s
}
