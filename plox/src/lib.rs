//! Plox
//! This library contains all Bézier curve- and font-related functionality.
//! Essentially, everything you need to turn strings into Bézier curve buffers,
//! and everything you need to manipulate said curves.
//!
//! There is no OpenGL stuff here, because i want to have the possibility to
//! switch to Vulkan if i ever seriously intend to maintain this for real.
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
