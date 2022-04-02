//! A font Atlas.
use crate::spline::{Quadratic, Rect, Spline};
use rayon::prelude::*;
use rustybuzz::Face;
use ttf_parser as ttf;

pub struct Atlas {
    pub outlines: Vec<Quadratic>,
    pub bboxes: Vec<Rect>,
    pub lut: Vec<(usize, usize)>,
}

impl Atlas {
    /// Create a new font atlas from a given font face.
    /// This is a relatively expensive operation!
    pub fn new(face: &Face) -> Atlas {
        // We store the arcs in a coordinate system of 1em to get consistency between fonts.
        let em = face.units_per_em() as f32;
        let n = face.number_of_glyphs();

        let mut outlines = Vec::with_capacity(10_000);
        let mut bboxes = Vec::with_capacity(1_000);
        let mut lut = Vec::with_capacity(1_000);

        let splines: Vec<Spline> = (0..n)
            // ~3x speedup on my 4-core machine.
            .into_par_iter()
            .map(|id| {
                let mut builder = Spline::builder().scale(1.0 / em);
                face.outline_glyph(ttf::GlyphId(id), &mut builder);
                let spline = builder.build();
                spline
            })
            .collect();

        for spline in splines {
            // The range [start, end) in the atlas.
            let start = outlines.len();
            let end = start + spline.len();

            outlines.extend(spline.strokes());
            bboxes.push(*spline.bbox());
            lut.push((start, end));
        }

        Atlas {
            outlines,
            bboxes,
            lut,
        }
    }
}
