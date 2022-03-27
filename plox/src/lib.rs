pub mod spline;
pub mod shaping;
pub mod font;

pub use spline::{Point, Cubic, Spline};

pub fn load() -> Spline {
    shaping::shape("α\u{2192}\u{03D1}\u{03B6}")
}

