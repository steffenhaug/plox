//! Polynomials.
//!
//! Implements what we need to create and manipulate polynomials
//! of different degrees in the context of Bézier curves.
use crate::approx;
use std::f32::consts::PI;

/// Polynomial of degree N over the set f32.
#[derive(Debug)]
pub struct Poly<const N: usize>(pub [f32; N]);

impl<const N: usize> Poly<N> {
    /// Evaluate P at t.
    pub fn at(&self, t: f32) -> f32 {
        self.0
            .iter()
            // A coefficients index corresponds to its power.
            .enumerate()
            .map(|(pow, coeff)| coeff * t.powi(pow as i32))
            .sum()
    }
}

impl Poly<4> {
    pub fn solve(&self) -> Vec<f32> {
        let coeffs = self.0;
        solve_cubic(coeffs[0], coeffs[1], coeffs[2], coeffs[3])
    }

    pub fn d(&self) -> Poly<3> {
        let a = self.0[3];
        let b = self.0[2];
        let c = self.0[1];
        Poly([c, 2.0 * b, 3.0 * a])
    }

    pub fn dd(&self) -> Poly<2> {
        let a = self.0[3];
        let b = self.0[2];
        Poly([2.0 * b, 6.0 * a])
    }
}

impl Poly<3> {
    pub fn solve(&self) -> (f32, f32) {
        let coeffs = self.0;
        solve_quadratic(coeffs[0], coeffs[1], coeffs[2])
    }
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
