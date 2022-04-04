//! A font Atlas.
use crate::spline::{Point, Quadratic, Rect, Spline};
use rayon::prelude::*;
use rustybuzz::Face;
use ttf_parser as ttf;

pub struct Atlas<'a> {
    pub outlines: Vec<Quadratic>,
    pub bboxes: Vec<Rect>,
    pub lut: Vec<(usize, usize)>,
    pub face: &'a Face<'a>,
}

pub struct Outline {
    pub ctrl_pts: Vec<(f32, f32)>,
    pub bbox: Rect,
}

impl<'a> Atlas<'a> {
    /// Create a new font atlas from a given font face.
    /// This is a relatively expensive operation!
    pub fn new(face: &'a Face) -> Atlas<'a> {
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
            face,
        }
    }

    pub fn outline(&self, text: &str) -> Outline {
        // this is a bit ugly atm. needs rework.
        let glyphs = crate::shaping::shape(text, self.face);

        let mut vertices: Vec<(f32, f32)> = Vec::with_capacity(10_000);

        let mut y0 = f32::INFINITY;
        let mut y1 = -f32::INFINITY;
        let mut x0 = f32::INFINITY;
        let mut x1 = -f32::INFINITY;

        for g in &glyphs {
            let (beg, end) = self.lut[g.glyph_id];
            let x = g.x; // offsets of the glyph
            let y = g.y;
            let bbox = g.bbox;

            // compute the accumulated bounding box
            x0 = f32::min(x0, bbox.x0 + x);
            x1 = f32::max(x1, bbox.x1 + x);
            y0 = f32::min(y0, bbox.y0 + y);
            y1 = f32::max(y1, bbox.y1 + y);

            for i in beg..end {
                let curve = self.outlines[i];
                vertices.push((curve.0 + Point { x, y }).into());
                vertices.push((curve.1 + Point { x, y }).into());
                vertices.push((curve.2 + Point { x, y }).into());
            }
        }

        Outline {
            ctrl_pts: vertices,
            bbox: Rect { x0, x1, y0, y1 },
        }
    }
}
