//! Bezier splines
//!
use crate::approx;
use crate::polynomial::Poly;
use std::ops;

/// A (control) point on a Bézier curve.
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// Linearly interpolate between points p, q and interpolation vartiable t.
pub fn lerp(p: Point, q: Point, t: f32) -> Point {
    let x = (1.0 - t) * p.x + t * q.x;
    let y = (1.0 - t) * p.y + t * q.y;
    Point { x, y }
}

/// A quadratic Bézier curve consists of three control points.
#[derive(Debug, Clone, Copy)]
pub struct Quadratic(pub Point, pub Point, pub Point);

/// A quadratic spline is a sequence of quadratic Bézier curves.
/// If this is used as the contour of some set, it should be a full
/// loop, i. e. do not omit the last line back to the start point even
/// if it is a straight line (Some .otf fonts do this).
#[derive(Debug)]
pub struct Spline {
    beziers: Vec<Quadratic>,
    bbox: Rect,
}

#[derive(Debug)]
pub struct Rect {
    pub x0: f32,
    pub x1: f32,
    pub y0: f32,
    pub y1: f32,
}

impl Quadratic {
    /// Evaluates the Bézier curve at a point t.
    pub fn at(&self, t: f32) -> Point {
        let q1 = lerp(self.0, self.1, t);
        let q2 = lerp(self.1, self.2, t);
        lerp(q1, q2, t)
    }

    /// Returns the polynomial B_y(t) of degree 4.
    pub fn y(&self) -> Poly<3> {
        let c = 1.0 * self.0.y;
        let b = -2.0 * self.0.y + 2.0 * self.1.y;
        let a = 1.0 * self.0.y - 2.0 * self.1.y + 1.0 * self.2.y;
        // ax² + bx + c
        Poly([c, b, a])
    }

    /// Returns the polynomial dB_y/dt of degree 3.
    pub fn dy(&self) -> Poly<2> {
        let b = -2.0 * self.0.y + 2.0 * self.1.y;
        let a = 1.0 * self.0.y - 2.0 * self.1.y + 1.0 * self.2.y;
        // d/dx ax² + bx + c = 2ax + b
        Poly([b, 2.0 * a])
    }
}

impl Spline {
    /// Iterator over the strokes of the spline. I. e. references
    /// to the underlying quadratic Bézier curves.
    pub fn strokes(&self) -> impl Iterator<Item = &Quadratic> {
        self.beziers.iter()
    }

    pub fn len(&self) -> usize {
        self.beziers.len()
    }

    pub fn bbox(&self) -> &Rect {
        &self.bbox
    }

    pub fn scale(self, s: f32) -> Spline {
        Spline {
            // I think this reallocates, which isn't good.
            beziers: self.beziers.into_iter().map(|bez| s * bez).collect(),
            bbox: Rect {
                x0: s * self.bbox.x0,
                x1: s * self.bbox.x1,
                y0: s * self.bbox.y0,
                y1: s * self.bbox.y1,
            },
        }
    }

    /// Creates a "translating" function that spits out translated versions
    /// of the Bézier curves in the spline. Useful for mapping over a spline.
    pub fn translate(x: f32, y: f32) -> impl Fn(&Quadratic) -> Quadratic {
        let dp = Point { x, y };
        move |bez| *bez + dp
    }

    // Eric Lengyels Winding Number Algorithm.
    //   https://jcgt.org/published/0006/02/02/paper.pdf
    pub fn winding_number(&self, p: Point) -> i32 {
        let mut w = 0;

        for bez in self.strokes().map(Spline::translate(0.0, -p.y)) {
            // Get the Bézier curves control points.
            let (y0, y1, y2) = (bez.0.y, bez.1.y, bez.2.y);

            // Calculate the jump.
            let jmp = if y0 > 0.0 { 8 } else { 0 }
                + if y1 > 0.0 { 4 } else { 0 }
                + if y2 > 0.0 { 2 } else { 0 };

            // Calculate the Bézier curves equivalence class.
            let class = 0x2E74 >> jmp;

            // Solve B_y(t) = 0. The equivalence class determines whether
            // to count these solutions towards the winding number or not.
            let (t1, t2) = bez.y().solve();

            // Low bit high => Use B(t1)
            if (class & 0b01 != 0) && bez.at(t1).x >= p.x {
                w += 1;
            }

            // High bit high => Use B(t2)
            if (class & 0b10 != 0) && bez.at(t2).x >= p.x {
                w -= 1;
            }
        }

        return w;
    }

    pub fn builder() -> Builder {
        Builder {
            beziers: vec![],
            position: Point { x: 0.0, y: 0.0 },
            start: None,
            cursor: Point { x: 0.0, y: 0.0 },
            // The bbox is not _completely_ tight: It has to preserve
            // kerning information, that's why the lower bounds start
            // at zero, and not infinity.
            y0: 0.0,
            y1: -f32::INFINITY,
            x0: 0.0,
            x1: -f32::INFINITY,
        }
    }
}

pub struct Builder {
    beziers: Vec<Quadratic>,
    position: Point,
    start: Option<Point>,
    // Cursor: All the other points are relative to this.
    cursor: Point,
    // Bounding box.
    y0: f32,
    y1: f32,
    x0: f32,
    x1: f32,
}

impl Builder {
    pub(crate) fn build(self) -> Spline {
        Spline {
            beziers: self.beziers,
            bbox: Rect {
                x0: self.x0,
                x1: self.x1,
                y0: self.y0,
                y1: self.y1,
            },
        }
    }

    /// Advances the cursor.
    pub(crate) fn advance(&mut self, x: f32, y: f32) {
        self.cursor.x += x;
        self.cursor.y += y;
    }

    fn expand_bbox(&mut self, x: f32, y: f32) {
        let x = x + self.cursor.x;
        let y = y + self.cursor.y;
        self.y0 = f32::min(self.y0, y);
        self.y1 = f32::max(self.y1, y);
        self.x0 = f32::min(self.x0, x);
        self.x1 = f32::max(self.x1, x);
    }
}

/// Using this, the TTF-library can construct a spline from a glyph.
impl ttf_parser::OutlineBuilder for Builder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.expand_bbox(x, y);
        // If we are moving after drawing a boundary, loop back to the start.
        // This ensures we have a closed loop.
        if let Some(start) = self.start {
            if !approx(0.0, Point::d(&start, &self.position)) {
                self.line_to(start.x, start.y);
            }
        }

        // Go to the requested position.
        self.position = Point { x, y };

        // Mark the start of a new boundary.
        self.start = Some(self.position);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.expand_bbox(x, y);
        // A straight line cubic can be made with two control points on said line.
        let target = Point { x, y };
        let m = lerp(self.position, target, 0.33);
        self.beziers.push(
            // Translate the Bézier curve relative to the cursor.
            Quadratic(self.position, m, target) + self.cursor,
        );
        self.position = target;
    }

    /// Insert a Bézier curve to (x, y)
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.expand_bbox(x1, y1);
        self.expand_bbox(x, y);
        self.beziers.push(
            // Translate the Bézier curve relative to the cursor.
            Quadratic(
                self.position,
                Point { x: x1, y: y1 }, // Control point
                Point { x, y },
            ) + self.cursor,
        );

        self.position = Point { x, y };
    }

    /// Insert a Bézier curve to (x, y) via the control points (x1, y1), (x2, y2).
    fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {
        // No accurate cubic->quadratic approximation yet.
        unimplemented!()
    }

    fn close(&mut self) {
        // Loop back to the start of the boundary. Some fonts "compress" by
        // omitting the line back to the start point if it is a simple straight
        // line.
        if let Some(start) = self.start {
            if !approx(0.0, Point::d(&start, &self.position)) {
                self.line_to(start.x, start.y);
            }
        }
    }
}

impl Point {
    /// Distance to another point.
    pub fn d(&self, p: &Point) -> f32 {
        let dx = f32::abs(p.x - self.x);
        let dy = f32::abs(p.y - self.y);
        f32::sqrt(dx.powi(2) + dy.powi(2))
    }
}

impl<const N: usize> std::fmt::Display for Poly<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (pow, coef) in self.0.iter().enumerate() {
            if pow != 0 {
                write!(f, " + ")?;
            }
            write!(f, "{}x^{}", coef, pow)?;
        }
        Ok(())
    }
}

// Arithmetic overloads.

impl ops::Add<Point> for Point {
    type Output = Point;

    fn add(self, rhs: Point) -> Self::Output {
        Point {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl ops::Add<Point> for Quadratic {
    type Output = Quadratic;

    fn add(self, rhs: Point) -> Self::Output {
        Quadratic(rhs + self.0, rhs + self.1, rhs + self.2)
    }
}

impl ops::Mul<Point> for f32 {
    type Output = Point;

    fn mul(self, rhs: Point) -> Self::Output {
        Point {
            x: self * rhs.x,
            y: self * rhs.y,
        }
    }
}

impl ops::Mul<Quadratic> for f32 {
    type Output = Quadratic;

    fn mul(self, rhs: Quadratic) -> Self::Output {
        Quadratic(self * rhs.0, self * rhs.1, self * rhs.2)
    }
}
