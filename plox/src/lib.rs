pub mod spline;
pub mod shaping;
pub mod font;
pub mod polynomial;

pub use polynomial::Poly;
pub use spline::{Point, Cubic, Quadratic, Spline};

/// Check if two numbers a,b are approximately equal.
/// "Apprixmately" has a _very_ liberal definition in this case.
fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}


pub fn load() -> Spline {
    shaping::shape("α\u{2192}\u{03D1}\u{03B6}")
}

