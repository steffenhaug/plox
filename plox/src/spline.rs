//! Cubic Bezier splines
//!

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
    pub(crate) fn builder() -> Builder {
        Builder {
            spline: vec![],
            position: Point { x: 0.0, y: 0.0 },
        }
    }

    pub fn strokes(&self) -> impl Iterator<Item = &Cubic> {
        self.0.iter()
    }
}

/// Linearly interpolate between points p, q and interpolation vartiable t.
pub fn lerp(p: Point, q: Point, t: f32) -> Point {
    let x = (1.0 - t) * p.x + t * q.x;
    let y = (1.0 - t) * p.y + t * q.y;
    Point { x, y }
}

pub(crate) struct Builder {
    spline: Vec<Cubic>,
    position: Point,
}

impl Builder {
    pub(crate) fn build(self) -> Spline {
        Spline(self.spline)
    }
}

/// Using this, the TTF-library can construct a spline from a glyph.
impl ttf_parser::OutlineBuilder for Builder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.position = Point { x, y };
    }

    fn line_to(&mut self, x: f32, y: f32) {
        // A straight line cubic can be made with two control points on said line.
        let target = Point { x, y };
        let m1 = lerp(self.position, target, 0.33);
        let m2 = lerp(self.position, target, 0.66);
        self.spline.push(Cubic(self.position, m1, m2, target));
        self.position = target;
    }

    fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) {
        // .otf fonts only use lines and cubic curves, no splines.
        // It is almost trivial to create an identivcal cubic spline
        // from a quadratic one, so this is easily fixed later.
        panic!("Only OpenType fonts supported for now");
    }

    /// Insert a Bézier curve to (x, y) via the control points (x1, y1), (x2, y2).
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.spline.push(Cubic(
            self.position,
            Point { x: x1, y: y1 }, // Control point 1
            Point { x: x2, y: y2 }, // Control point 2
            Point { x, y },
        ));
        self.position = Point { x, y };
    }

    fn close(&mut self) {
        /* do nothing */
    }
}
