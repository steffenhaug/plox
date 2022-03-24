//! Cubic Bezier splines

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug)]
pub struct Cubic(pub Point, pub Point, pub Point, pub Point);

#[derive(Debug)]
pub struct Spline(Vec<Cubic>);

impl Spline {
    fn builder() -> Builder {
        Builder {
            spline: vec![],
            position: Point { x: 0.0, y: 0.0 },
        }
    }

    pub fn strokes(&self) -> impl Iterator<Item=&Cubic> {
        self.0.iter()
    }
}

struct Builder {
    spline: Vec<Cubic>,
    position: Point,
}

impl Builder {
    fn build(self) -> Spline {
        Spline(self.spline)
    }
}

impl ttf_parser::OutlineBuilder for Builder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.position = Point { x, y };
    }

    fn line_to(&mut self, x: f32, y: f32) {
        // A line can be made with two control points exactly in between.
        let cx = 0.5 * (self.position.x + x);
        let cy = 0.5 * (self.position.y + y);
        self.spline.push(Cubic(
            self.position,
            Point { x: cx, y: cy },
            Point { x: cx, y: cy },
            Point { x, y },
        ));
        self.position = Point { x, y };
    }

    fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) {
        // .otf fonts only use lines and cubic curves, no splines.
        panic!("Only OpenType fonts supported for now");
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.spline.push(Cubic(
            self.position,
            Point { x: x1, y: y1 },
            Point { x: x2, y: y2 },
            Point { x, y },
        ));
        self.position = Point { x, y };
    }

    fn close(&mut self) {
        /* do nothing */
    }
}

pub fn load() -> Spline {
    // Pretty stupid to bloat the binary with this, but it works.
    let bytes = include_bytes!("../res/lm/latinmodern-math.otf");
    let face = rustybuzz::Face::from_slice(bytes, 0).unwrap();
    let mut buf = rustybuzz::UnicodeBuffer::new();
    buf.push_str("\u{03C8}");

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

