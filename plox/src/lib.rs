pub mod spline;
pub use spline::{Point, Cubic, Spline};

pub fn load() -> Spline {
    // Pretty stupid to bloat the binary with this, but it works.
    let bytes = include_bytes!("../res/lm/latinmodern-math.otf");
    let face = rustybuzz::Face::from_slice(bytes, 0).unwrap();
    let mut buf = rustybuzz::UnicodeBuffer::new();
    buf.push_str("\u{03BE}");

    let glyph_buffer = rustybuzz::shape(&face, &[], buf);
    dbg!(&glyph_buffer);

    let mut h = Spline::builder();
    face.outline_glyph(
        ttf_parser::GlyphId(glyph_buffer.glyph_infos()[0].glyph_id as u16),
        &mut h,
    );
    let h = h.build();
    dbg!(&h);
    h
}

