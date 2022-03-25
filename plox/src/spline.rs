//! Cubic Bezier splines
//!
use std::f32::consts::PI;

/// Check if two numbers a,b are approximately equal.
fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-8
}

/// A (control) point on a Bézier curve.
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// A cubic Bézier curve consists of four control points.
#[derive(Debug)]
pub struct Cubic(pub Point, pub Point, pub Point, pub Point);

impl Cubic {
    pub fn at(&self, t: f32) -> Point {
        let q1 = lerp(self.0, self.1, t);
        let q2 = lerp(self.1, self.2, t);
        let q3 = lerp(self.2, self.3, t);

        let q4 = lerp(q1, q2, t);
        let q5 = lerp(q2, q3, t);

        lerp(q4, q5, t)
    }
}

/// A cubic spline is a sequence of cubic Bézier curves.
/// If this is used as the contour of some set, it should be a full
/// loop, i. e. do not omit the last line back to the start point even
/// if it is a straight line.
#[derive(Debug)]
pub struct Spline(Vec<Cubic>);

impl Spline {
    pub(crate) fn builder() -> Builder {
        Builder {
            spline: vec![],
            position: Point { x: 0.0, y: 0.0 },
            start: None,
        }
    }

    pub fn strokes(&self) -> impl Iterator<Item = &Cubic> {
        self.0.iter()
    }

    pub fn winding_number(&self, p: Point) -> i32 {
        unimplemented!()
    }
}

/// Linearly interpolate between points p, q and interpolation vartiable t.
pub fn lerp(p: Point, q: Point, t: f32) -> Point {
    let x = (1.0 - t) * p.x + t * q.x;
    let y = (1.0 - t) * p.y + t * q.y;
    Point { x, y }
}

/// Solve P(x) = 0 for some polynomial P = k3 x^3 + k2 x^2 + k1 x + k0.
pub fn solve(k0: f32, k1: f32, k2: f32, k3: f32) -> Vec<f32> {
    let c = k0;
    let b = -3.0 * k0 + 3.0 * k1;
    let a = 3.0 * k0 - 6.0 * k1 + 3.0 * k2;
    let d = -k0 + 3.0 * k1 - 3.0 * k2 + k3;

    if approx(0.0, d) {
        // Quadratic solution.
        //
        // We can calculate the answer much faster, but also, dividing by
        // the leading coefficient could be undefined or very numerically
        // unstable (zero or close to).
        let discriminant = f32::sqrt(b * b - 4.0 * a * c);
        return vec![
            (discriminant - b) / (2.0 * a),
            (-discriminant - b) / (2.0 * a),
        ];
    }

    // Cubic solution is required.

    // Calculate the depressed cubic P(s) = x^3 + px + q.
    let c = c / d;
    let b = b / d;
    let a = a / d;

    let p = (3.0 * b - a.powi(2)) / 3.0;
    let q = (2.0 * a.powi(3) - 9.0 * a * b + 27.0 * c) / 27.0;

    // Discriminant Δ.
    let delta = (q).powi(2) / 4.0 + p.powi(3) / 27.0;

    if delta < 0.0 {
        let r = f32::sqrt(-p.powi(3) / 27.0);
        let phi = f32::atan2(-delta.sqrt(), -q / 2.0);

        // Three real solutions.
        return vec![
            2.0 * r.cbrt() * f32::cos(phi / 3.0),
            2.0 * r.cbrt() * f32::cos((phi + 2.0 * PI) / 3.0),
            2.0 * r.cbrt() * f32::cos((phi + 4.0 * PI) / 3.0),
        ];
    }

    if delta == 0.0 {
        let u = -(q / 2.0).cbrt();
        let v = -(q / 2.0).cbrt();

        // Two real solutions.
        return vec![u + v - a / 3.0, -0.5 * (u + v) - a / 3.0];
    }

    if delta > 0.0 {
        let u = (-(q / 2.0) + delta.sqrt()).cbrt();
        let v = (-(q / 2.0) - delta.sqrt()).cbrt();

        // One real solution.
        return vec![u + v - a / 3.0];
    }

    unreachable!()
}

pub(crate) struct Builder {
    spline: Vec<Cubic>,
    position: Point,
    start: Option<Point>,
}

impl Builder {
    pub(crate) fn build(self) -> Spline {
        Spline(self.spline)
    }
}

/// Using this, the TTF-library can construct a spline from a glyph.
impl ttf_parser::OutlineBuilder for Builder {
    fn move_to(&mut self, x: f32, y: f32) {
        // If we are moving after drawing a boundary, loop back to the start.
        // This ensures we have a closed loop.
        if let Some(start) = self.start {
            self.line_to(start.x, start.y);
        }

        // Go to the requested position.
        self.position = Point { x, y };

        // Mark the start of a new boundary.
        self.start = Some(self.position);
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
        // Loop back to the start of the boundary. Some fonts "compress" by
        // omitting the line back to the start point if it is a simple straight
        // line.
        if let Some(start) = self.start {
            self.line_to(start.x, start.y);
        }
    }
}
