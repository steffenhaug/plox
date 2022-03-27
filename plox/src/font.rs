//! Font loading.
//! At the time of writing this module just contains lazy-static
//! definitions of LaTeX font faces baked into the binary. This
//! is not elegant since it bloats the binary, but the fonts are
//! actually quite small (couple hundred K) and Rust-binaries are
//! already huge (> 50M at time of writing), so I'd say its a good
//! tradeoff for not having to deal with fonts missing and most
//! importantly, trying to figure out where the fuck the fonts are
//! on Windows.
use lazy_static::lazy_static;
use rustybuzz::Face;

lazy_static! {
    pub static ref LM_MATH: Face<'static> = {
        let bytes = include_bytes!("../res/lm/latinmodern-math.ttf");
        // Unwrap OK. Can't really fail with the bytes baked in the binary.
        rustybuzz::Face::from_slice(bytes, 0).unwrap()
    };
}
