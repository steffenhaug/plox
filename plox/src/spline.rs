//! Bezier splines
//!
use crate::approx;
use crate::polynomial::Poly;
use std::ops;

/// A (control) point on a Bézier curve. TODO: Replace with glm::Vec2
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

#[derive(Debug, Clone, Copy)]
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
            beziers: Vec::with_capacity(64),
            position: Point { x: 0.0, y: 0.0 },
            start: None,
            scale: 1.0,
            y0: f32::INFINITY,
            y1: -f32::INFINITY,
            x0: f32::INFINITY,
            x1: -f32::INFINITY,
        }
    }
}

pub struct Builder {
    beziers: Vec<Quadratic>,
    position: Point,
    start: Option<Point>,
    // Cursor: All the other points are relative to this.
    scale: f32,
    // Bounding box.
    y0: f32,
    y1: f32,
    x0: f32,
    x1: f32,
}

impl Builder {
    pub fn build(self) -> Spline {
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

    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    fn expand_bbox(&mut self, x: f32, y: f32) {
        self.y0 = f32::min(self.y0, y);
        self.y1 = f32::max(self.y1, y);
        self.x0 = f32::min(self.x0, x);
        self.x1 = f32::max(self.x1, x);
    }
}

/// Using this, the TTF-library can construct a spline from a glyph.
impl ttf_parser::OutlineBuilder for Builder {
    fn move_to(&mut self, x: f32, y: f32) {
        let x = x * self.scale;
        let y = y * self.scale;
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
        let x = x * self.scale;
        let y = y * self.scale;
        self.expand_bbox(x, y);
        // A straight line cubic can be made with two control points on said line.
        let target = Point { x, y };
        let m = lerp(self.position, target, 0.33);
        self.beziers.push(
            // Translate the Bézier curve relative to the cursor.
            Quadratic(self.position, m, target),
        );
        self.position = target;
    }

    /// Insert a Bézier curve to (x, y)
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let x = x * self.scale;
        let y = y * self.scale;
        let x1 = x1 * self.scale;
        let y1 = y1 * self.scale;
        self.expand_bbox(x1, y1);
        self.expand_bbox(x, y);
        self.beziers.push(
            // Translate the Bézier curve relative to the cursor.
            Quadratic(
                self.position,
                Point { x: x1, y: y1 }, // Control point
                Point { x, y },
            ),
        );

        self.position = Point { x, y };
    }

    /// Insert a Bézier curve to (x, y) via the control points (x1, y1), (x2, y2).
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let x = x * self.scale;
        let y = y * self.scale;
        let x1 = x1 * self.scale;
        let y1 = y1 * self.scale;
        let x2 = x2 * self.scale;
        let y2 = y2 * self.scale;

        self.expand_bbox(x1, y1);
        self.expand_bbox(x2, y2);
        self.expand_bbox(x, y);

        let cubic = Cubic {
            p0: glm::vec2(self.position.x, self.position.y),
            p1: glm::vec2(x1, y1),
            p2: glm::vec2(x2, y2),
            p3: glm::vec2(x, y),
        };

        // Take some on-curve sample points.
        let samples = [
            cubic.r(0.0),
            cubic.r(1.0 / 6.0),
            cubic.r(1.0 / 3.0),
            cubic.r(3.0 / 6.0),
            cubic.r(2.0 / 3.0),
            cubic.r(5.0 / 6.0),
            cubic.r(1.0),
        ];

        // Approximate the cubic by three quadratics.
        let q1 = Quadratic(
            /* 0 1 2 */
            Point {
                x: samples[0].x,
                y: samples[0].y,
            },
            Point {
                x: 2.0 * samples[1].x - 0.5 * (samples[0].x + samples[2].x),
                y: 2.0 * samples[1].y - 0.5 * (samples[0].y + samples[2].y),
            },
            Point {
                x: samples[2].x,
                y: samples[2].y,
            },
        );

        let q2 = Quadratic(
            /* 2 3 4 */
            Point {
                x: samples[2].x,
                y: samples[2].y,
            },
            Point {
                x: 2.0 * samples[3].x - 0.5 * (samples[2].x + samples[4].x),
                y: 2.0 * samples[3].y - 0.5 * (samples[2].y + samples[4].y),
            },
            Point {
                x: samples[4].x,
                y: samples[4].y,
            },
        );

        let q3 = Quadratic(
            /* 4 5 6 */
            Point {
                x: samples[4].x,
                y: samples[4].y,
            },
            Point {
                x: 2.0 * samples[5].x - 0.5 * (samples[4].x + samples[6].x),
                y: 2.0 * samples[5].y - 0.5 * (samples[4].y + samples[6].y),
            },
            Point {
                x: samples[6].x,
                y: samples[6].y,
            },
        );
        self.beziers.extend([q1, q2, q3]);
        self.position = Point { x, y };
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

impl Rect {
    pub fn width(&self) -> f32 {
        self.x1 - self.x0
    }

    pub fn height(&self) -> f32 {
        self.y1 - self.y0
    }

    /// Calculate a new Rect which is the tight bounding box around two rectangles.
    pub fn extend(&self, rect: Rect) -> Rect {
        let x0 = f32::min(self.x0, rect.x0);
        let x1 = f32::max(self.x1, rect.x1);
        let y0 = f32::min(self.y0, rect.y0);
        let y1 = f32::max(self.y1, rect.y1);
        Rect { x0, y0, x1, y1 }
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

impl std::convert::From<Point> for (f32, f32) {
    fn from(p: Point) -> (f32, f32) {
        (p.x, p.y)
    }
}

//
// Cubic stuff
//

pub struct Cubic {
    pub p0: glm::Vec2,
    pub p1: glm::Vec2,
    pub p2: glm::Vec2,
    pub p3: glm::Vec2,
}

impl Cubic {
    pub fn pts(p0: glm::Vec2, p1: glm::Vec2, p2: glm::Vec2, p3: glm::Vec2) -> Self {
        Cubic { p0, p1, p2, p3 }
    }

    pub fn y(&self) -> Poly<4> {
        let c = 1.0 * self.p0.y;
        let b = -3.0 * self.p0.y + 3.0 * self.p1.y;
        let a = 3.0 * self.p0.y - 6.0 * self.p1.y + 3.0 * self.p2.y;
        let d = -1.0 * self.p0.y + 3.0 * self.p1.y - 3.0 * self.p2.y + 1.0 * self.p3.y;
        Poly([c, b, a, d])
    }

    pub fn x(&self) -> Poly<4> {
        let c = 1.0 * self.p0.x;
        let b = -3.0 * self.p0.x + 3.0 * self.p1.x;
        let a = 3.0 * self.p0.x - 6.0 * self.p1.x + 3.0 * self.p2.x;
        let d = -1.0 * self.p0.x + 3.0 * self.p1.x - 3.0 * self.p2.x + 1.0 * self.p3.x;
        Poly([c, b, a, d])
    }

    pub fn r(&self, t: f32) -> glm::Vec2 {
        glm::vec2(self.x().at(t), self.y().at(t))
    }

    pub fn curvature(&self, t: f32) -> f32 {
        let dx = self.x().d();
        let ddx = self.x().dd();
        let dy = self.y().d();
        let ddy = self.y().dd();

        let kappa = (dx.at(t) * ddy.at(t) - ddx.at(t) * dy.at(t))
            / (dx.at(t).powi(2) + dy.at(t).powi(2)).powf(3.0 / 2.0);

        kappa
    }

    pub fn sample(&self) -> Vec<glm::Vec2> {
        let mut samples = Vec::with_capacity(128);
        samples.push(self.r(0.0));
        Self::sample_interval(self, &mut samples, glm::vec2(0.0, 1.0));
        samples.push(self.r(1.0));

        samples
    }

    fn sample_interval(curve: &Cubic, samples: &mut Vec<glm::Vec2>, interval: glm::Vec2) {
        const EPS: f32 = 3.0;
        let t0 = interval.x;
        let t1 = interval.y;
        let t = 0.5 * (t0 + t1);

        // Segment length is a bad, but easy and fast error measure.
        // Some alternatives are outlined in the report.
        let l = (curve.r(t) - curve.r(t0)).norm();

        if l < EPS {
            samples.extend([curve.r(t)]);
        } else {
            Self::sample_interval(curve, samples, glm::vec2(t0, t));
            Self::sample_interval(curve, samples, glm::vec2(t, t1));
        }
    }
}
