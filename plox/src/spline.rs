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

/// Polynomial of degree N over the set f32.
#[derive(Debug)]
pub struct Poly<const N: usize>(pub [f32; N]);

impl<const N: usize> Poly<N> {
    pub fn at(&self, t: f32) -> f32 {
        self.0
            .iter()
            // Enumerate the coefficients.
            .enumerate()
            // This is why they are stored in "reverse" order.
            .map(|(pow, coeff)| coeff * t.powi(pow as i32))
            .sum()
    }
}

impl Poly<4> {
    fn solve(&self) -> Vec<f32> {
        let coeffs = self.0;
        solve_cubic(coeffs[0], coeffs[1], coeffs[2], coeffs[3])
    }
}

/// A cubic Bézier curve consists of four control points.
#[derive(Debug, Clone, Copy)]
pub struct Cubic(pub Point, pub Point, pub Point, pub Point);

impl Cubic {
    /// Evaluates the Bézier curve at a point t.
    pub fn at(&self, t: f32) -> Point {
        let q1 = lerp(self.0, self.1, t);
        let q2 = lerp(self.1, self.2, t);
        let q3 = lerp(self.2, self.3, t);

        let q4 = lerp(q1, q2, t);
        let q5 = lerp(q2, q3, t);

        lerp(q4, q5, t)
    }

    /// Get the Bézier curve translated by some vector (dx, dy).
    /// Affine transformations on Bézier curves can be applied to
    /// the control points.
    pub fn translate(&self, dx: f32, dy: f32) -> Cubic {
        Cubic(
            Point {
                x: self.0.x + dx,
                y: self.0.y + dy,
            },
            Point {
                x: self.1.x + dx,
                y: self.1.y + dy,
            },
            Point {
                x: self.2.x + dx,
                y: self.2.y + dy,
            },
            Point {
                x: self.3.x + dx,
                y: self.3.y + dy,
            },
        )
    }

    /// Returns the polynomial B_y(t) of degree 4.
    pub fn y(&self) -> Poly<4> {
        let c =  1.0 * self.0.y;
        let b = -3.0 * self.0.y + 3.0 * self.1.y;
        let a =  3.0 * self.0.y - 6.0 * self.1.y + 3.0 * self.2.y;
        let d = -1.0 * self.0.y + 3.0 * self.1.y - 3.0 * self.2.y + 1.0 * self.3.y;
        Poly([c, b, a, d])
    }

    /// Returns the polynomial dB_y/dt of degree 3.
    pub fn dy(&self) -> Poly<3> {
        let b = -3.0 * self.0.y + 3.0 * self.1.y;
        let a =  3.0 * self.0.y - 6.0 * self.1.y + 3.0 * self.2.y;
        let d = -1.0 * self.0.y + 3.0 * self.1.y - 3.0 * self.2.y + 1.0 * self.3.y;
        Poly([b, 2.0 * a, 3.0 * d])
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
        // Zero-cost madness!
        self.strokes()
            // Shift the stroke so p.y = 0
            .map(|bez| bez.translate(0.0, -p.y))
            // Get the B_y(t) polynomials solution.
            // Note: We keep the Bézier curve around to evaluate the solution.
            .map(|bez| (bez.y().solve(), bez))
            // Transpose from ([sol ...], bez) to [(sol, bez) ...]
            .flat_map(|(solns, bez)| solns.into_iter().zip(std::iter::repeat(bez)))
            // Evaluate the solution:
            .map(|(sol, bez)| (sol, bez.at(sol), bez.dy()))
            // Get only the solutions to the right of p.
            .filter(|(_, sol_p, _)| sol_p.x > p.x)
            // Get only the solutions that actually correspond to lines on the
            // Bézier curve. Strict inequality is crucial, or there will be a
            // interval where a ray could hit two connecting curves.
            .filter(|(t, _, _)| *t > 0.0 && *t < 1.0)
            // Calculate the contribution to the winding number:
            // If dB_y/dt is positive, curve is going upwards, i. e. we
            // are _leaving_ the set contained in the boundary.
            .map(|(t, _, dy)| {
                if approx(0.0, dy.at(t)) {
                    0
                } else if dy.at(t) > 0.0 {
                    -1
                } else {
                    1
                }
            })
            .sum()
    }
}

/// Linearly interpolate between points p, q and interpolation vartiable t.
pub fn lerp(p: Point, q: Point, t: f32) -> Point {
    let x = (1.0 - t) * p.x + t * q.x;
    let y = (1.0 - t) * p.y + t * q.y;
    Point { x, y }
}

/// Solve P(x) = 0 for some polynomial P = dx³ + ax² + bx + c.
pub fn solve_cubic(c: f32, b: f32, a: f32, d: f32) -> Vec<f32> {
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

    // Calculate the depressed cubic P(s) = x³ + px + q.
    let c = c / d;
    let b = b / d;
    let a = a / d;

    let p = (3.0 * b - a.powi(2)) / 3.0;
    let q = (2.0 * a.powi(3) - 9.0 * a * b + 27.0 * c) / 27.0;

    // Discriminant Δ.
    let delta = (q).powi(2) / 4.0 + p.powi(3) / 27.0;

    if delta == 0.0 {
        // Δ = 0 => Two real solutions.
        let u = -(q / 2.0).cbrt();
        let v = -(q / 2.0).cbrt();
        return vec![u + v - a / 3.0, -0.5 * (u + v) - a / 3.0];
    }

    if delta > 0.0 {
        // Δ > 0 => One real solution.
        let u = (-(q / 2.0) + delta.sqrt()).cbrt();
        let v = (-(q / 2.0) - delta.sqrt()).cbrt();
        return vec![u + v - a / 3.0];
    }

    // Δ < 0 => Three real solutions!
    let r = f32::sqrt(-p.powi(3) / 27.0);
    let phi = f32::atan2(f32::sqrt(-delta), -q / 2.0);

    // i think i can do this without resorting to trig.

    return vec![
        2.0 * r.cbrt() * f32::cos(phi / 3.0) - a / 3.0,
        2.0 * r.cbrt() * f32::cos((phi + 2.0 * PI) / 3.0) - a / 3.0,
        2.0 * r.cbrt() * f32::cos((phi + 4.0 * PI) / 3.0) - a / 3.0,
    ];
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
