//! Bezier splines
//!
use std::f32::consts::PI;

/// Check if two numbers a,b are approximately equal.
/// "Apprixmately" has a _very_ liberal definition in this case.
fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

/// A (control) point on a Bézier curve.
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn d(&self, p: Point) -> f32 {
        let dx = f32::abs(p.x - self.x);
        let dy = f32::abs(p.y - self.y);
        f32::sqrt(dx.powi(2) + dy.powi(2))
    }
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
    pub fn solve(&self) -> Vec<f32> {
        let coeffs = self.0;
        solve_cubic(coeffs[0], coeffs[1], coeffs[2], coeffs[3])
    }
}

impl Poly<3> {
    pub fn solve(&self) -> (f32, f32) {
        let coeffs = self.0;
        solve_quadratic(coeffs[0], coeffs[1], coeffs[2])
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

    pub fn lower(&self) -> Quadratic {
        // This is just an unrolled matrix multiplication, nothing scary.

        // P0
        let x0 = 19.0 * self.0.x + 3.0 * self.1.x - 3.0 * self.2.x + 1.0 * self.3.x;
        let y0 = 19.0 * self.0.y + 3.0 * self.1.y - 3.0 * self.2.y + 1.0 * self.3.y;
        let p0 = Point {
            x: x0 / 20.0,
            y: y0 / 20.0,
        };

        // P1
        let x1 = -5.0 * self.0.x + 15.0 * self.1.x + 15.0 * self.2.x - 5.0 * self.3.x;
        let y1 = -5.0 * self.0.y + 15.0 * self.1.y + 15.0 * self.2.y - 5.0 * self.3.y;
        let p1 = Point {
            x: x1 / 20.0,
            y: y1 / 20.0,
        };

        // P2
        let x2 = 1.0 * self.0.x - 3.0 * self.1.x + 3.0 * self.2.x + 19.0 * self.3.x;
        let y2 = 1.0 * self.0.y - 3.0 * self.1.y + 3.0 * self.2.y + 19.0 * self.3.y;
        let p2 = Point {
            x: x2 / 20.0,
            y: y2 / 20.0,
        };

        Quadratic(p0, p1, p2)
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

    pub fn scale(&self, s: f32) -> Cubic {
        Cubic(
            Point {
                x: self.0.x * s,
                y: self.0.y * s,
            },
            Point {
                x: self.1.x * s,
                y: self.1.y * s,
            },
            Point {
                x: self.2.x * s,
                y: self.2.y * s,
            },
            Point {
                x: self.3.x * s,
                y: self.3.y * s,
            },
        )
    }

    /// Returns the polynomial B_y(t) of degree 4.
    pub fn y(&self) -> Poly<4> {
        let c = 1.0 * self.0.y;
        let b = -3.0 * self.0.y + 3.0 * self.1.y;
        let a = 3.0 * self.0.y - 6.0 * self.1.y + 3.0 * self.2.y;
        let d = -1.0 * self.0.y + 3.0 * self.1.y - 3.0 * self.2.y + 1.0 * self.3.y;
        Poly([c, b, a, d])
    }

    /// Returns the polynomial dB_y/dt of degree 3.
    pub fn dy(&self) -> Poly<3> {
        let b = -3.0 * self.0.y + 3.0 * self.1.y;
        let a = 3.0 * self.0.y - 6.0 * self.1.y + 3.0 * self.2.y;
        let d = -1.0 * self.0.y + 3.0 * self.1.y - 3.0 * self.2.y + 1.0 * self.3.y;
        Poly([b, 2.0 * a, 3.0 * d])
    }
}

/// A quadratic Bézier curve consists of three control points.
#[derive(Debug, Clone, Copy)]
pub struct Quadratic(pub Point, pub Point, pub Point);

impl Quadratic {
    /// Evaluates the Bézier curve at a point t.
    pub fn at(&self, t: f32) -> Point {
        let q1 = lerp(self.0, self.1, t);
        let q2 = lerp(self.1, self.2, t);
        lerp(q1, q2, t)
    }

    /// Returns the polynomial B_y(t) of degree 4.
    pub fn y(&self) -> Poly<3> {
        let c =  1.0 * self.0.y;
        let b = -2.0 * self.0.y + 2.0 * self.1.y;
        let a =  1.0 * self.0.y - 2.0 * self.1.y + 1.0 * self.2.y;
        // ax² + bx + c
        Poly([c, b, a])
    }

    /// Returns the polynomial dB_y/dt of degree 3.
    pub fn dy(&self) -> Poly<2> {
        let b = -2.0 * self.0.y + 2.0 * self.1.y;
        let a =  1.0 * self.0.y - 2.0 * self.1.y + 1.0 * self.2.y;
        // d/dx ax² + bx + c = 2ax + b
        Poly([b, 2.0 * a])
    }

    /// Get the Bézier curve translated by some vector (dx, dy).
    /// Affine transformations on Bézier curves can be applied to
    /// the control points.
    pub fn translate(&self, dx: f32, dy: f32) -> Quadratic {
        Quadratic(
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
        )
    }

    pub fn scale(&self, s: f32) -> Quadratic {
        Quadratic(
            Point {
                x: self.0.x * s,
                y: self.0.y * s,
            },
            Point {
                x: self.1.x * s,
                y: self.1.y * s,
            },
            Point {
                x: self.2.x * s,
                y: self.2.y * s,
            },
        )
    }
}

/// A quadratic spline is a sequence of quadratic Bézier curves.
/// If this is used as the contour of some set, it should be a full
/// loop, i. e. do not omit the last line back to the start point even
/// if it is a straight line.
#[derive(Debug)]
pub struct Spline(Vec<Quadratic>);

impl Spline {
    pub(crate) fn builder() -> Builder {
        Builder {
            spline: vec![],
            position: Point { x: 0.0, y: 0.0 },
            start: None,
            cursor: Point { x: 0.0, y: 0.0 },
        }
    }

    pub fn strokes(&self) -> impl Iterator<Item = &Quadratic> {
        self.0.iter()
    }

    pub fn scale(self, s: f32) -> Spline {
        Spline(self.0.into_iter().map(|bez| bez.scale(s)).collect())
    }

    pub fn winding_number(&self, p: Point) -> i32 {
        let mut w = 0;

        for bez in self.strokes().map(|b| b.translate(0.0, -p.y)) {
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
}

/// Linearly interpolate between points p, q and interpolation vartiable t.
pub fn lerp(p: Point, q: Point, t: f32) -> Point {
    let x = (1.0 - t) * p.x + t * q.x;
    let y = (1.0 - t) * p.y + t * q.y;
    Point { x, y }
}

/// Solve P(x) = 0 for some (linear) polynomial P = mx + b
pub fn solve_linear(c: f32, m: f32) -> f32 {
    // Danger: This can be NaN. Not sure if that is a problem.
    -c / m
}

/// Solve P(x) = 0 for some polynomial P = ax² + bx + c
/// Will return (NaN, NaN) if Δ < 0, and the same root twice
/// if Δ = 0 (root with multiplicity two case).
pub fn solve_quadratic(c: f32, b: f32, a: f32) -> (f32, f32) {
    // If a or b is small, dividing by them is dangerous, so we need
    // to handle these low-order polynomials specially.
    if approx(0.0, a) {
        // a ~ 0 => P = bx + c. (linear)
        let t = solve_linear(c, b);
        return (t, t);
    }

    // Discriminant Δ.
    let delta = b * b - 4.0 * a * c;
    // Δ = 0 => one root (with multiplicity two)
    // Δ > 0 => two distinct roots
    // Δ < 0 => imaginary roots (will be NaN, but never used)
    (
        (-b + f32::sqrt(delta)) / (2.0 * a),
        (-b - f32::sqrt(delta)) / (2.0 * a),
    )
}

/// Solve P(x) = 0 for some polynomial P = dx³ + ax² + bx + c.
pub fn solve_cubic(c: f32, b: f32, a: f32, d: f32) -> Vec<f32> {
    if approx(0.0, d) {
        if approx(0.0, a) {
            if approx(0.0, b) {
                // Constant equation; either zero or infinitely many solutions.
                // For our purpose, this corresponds to the ray following a horizontal
                // segment of a glyph, and we might as well define that to not be an
                // intersection.
                return vec![];
            }

            // Linear equation.
            return vec![-c / b];
        }

        // Quadratic equation.
        let delta = b * b - 4.0 * a * c;

        // Δ = 0 => one root
        // Δ > 0 => two distinct roots
        // Δ < 0 => imaginary roots (which we ignore)
        if delta > 0.0 {
            return vec![
                (-b - f32::sqrt(delta)) / (2.0 * a),
                (-b + f32::sqrt(delta)) / (2.0 * a),
            ];
        }

        if delta == 0.0 {
            return vec![-b / (2.0 * a)];
        }

        return vec![];
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
    spline: Vec<Quadratic>,
    position: Point,
    start: Option<Point>,
    // Cursor: All the other points are relative to this.
    cursor: Point,
}

impl Builder {
    pub(crate) fn build(self) -> Spline {
        Spline(self.spline)
    }

    /// Advances the cursor.
    pub(crate) fn advance(&mut self, x: f32, y: f32) {
        self.cursor.x += x;
        self.cursor.y += y;
    }
}

/// Using this, the TTF-library can construct a spline from a glyph.
impl ttf_parser::OutlineBuilder for Builder {
    fn move_to(&mut self, x: f32, y: f32) {
        // If we are moving after drawing a boundary, loop back to the start.
        // This ensures we have a closed loop.
        if let Some(start) = self.start {
            if !approx(0.0, Point::d(&start, self.position)) {
                self.line_to(start.x, start.y);
            }
        }

        // Go to the requested position.
        self.position = Point { x, y };

        // Mark the start of a new boundary.
        self.start = Some(self.position);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        // A straight line cubic can be made with two control points on said line.
        let target = Point { x, y };
        let m = lerp(self.position, target, 0.33);
        self.spline.push(
            Quadratic(self.position, m, target)
                // Translate the Bézier curve relative to the cursor.
                .translate(self.cursor.x, self.cursor.y)
        );
        self.position = target;
    }

    /// Insert a Bézier curve to (x, y)
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.spline.push(
            Quadratic(
                self.position,
                Point { x: x1, y: y1 }, // Control point
                Point { x, y },
            )
            // Translate the Bézier curve relative to the cursor.
            .translate(self.cursor.x, self.cursor.y),
        );

        self.position = Point { x, y };
    }

    /// Insert a Bézier curve to (x, y) via the control points (x1, y1), (x2, y2).
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.spline.push(
            Cubic(
                self.position,
                Point { x: x1, y: y1 }, // Control point 1
                Point { x: x2, y: y2 }, // Control point 2
                Point { x, y },
            )
            // Translate the Bézier curve relative to the cursor.
            .translate(self.cursor.x, self.cursor.y)
            // Approximate by a quadratic Bézier curve.
            .lower(),
        );
        self.position = Point { x, y };
    }

    fn close(&mut self) {
        // Loop back to the start of the boundary. Some fonts "compress" by
        // omitting the line back to the start point if it is a simple straight
        // line.
        if let Some(start) = self.start {
            if !approx(0.0, Point::d(&start, self.position)) {
                self.line_to(start.x, start.y);
            }
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
