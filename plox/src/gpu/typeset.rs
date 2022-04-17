//! # Typesetting
//!
//! Typesetting essentially amounts to taking some text, create a VAO for it, and
//! assign its place in the scene graph so it gets rendered with the correct transform.
use self::Node::*;
use crate::atlas::Atlas;
use crate::gpu::text::{SharedText, TextElement, TextRenderer, TextShader};
use crate::gpu::Transform;
use crate::spline::Rect;
use std::sync::{Arc, RwLock};

enum Node {
    // Just plain old text. (Leaf node)
    Text(SharedText),
    // Sequence (multiple typeset text elements after one another)
    Seq(Vec<Typeset>),
    // An integral symbol with optional limits.
    Limits {
        lo_limit: Option<Arc<Typeset>>,
        hi_limit: Option<Arc<Typeset>>,
        text: Arc<Typeset>,
    },
    // Still to do:
    // - sub/superscript
    // - fractions
    // - radicals
    // - vectors (can be done using bold notation)
}

/// A Typeset text element is essentially a scene graph.
pub struct Typeset {
    content: Node,
    pub bbox: Rect,
    // Transform relative to the parent node.
    pub transform: Box<dyn Fn() -> Transform>,
}

impl Typeset {
    pub fn transform(mut self, f: impl 'static + Fn() -> Transform) -> Self {
        self.transform = Box::new(f);
        self
    }

    pub unsafe fn text(txt: &str, atlas: &Atlas) -> Self {
        let content = TextElement::new(txt, atlas);
        let bbox = content.bbox;
        Typeset {
            content: Node::Text(Arc::new(RwLock::new(content))),
            bbox,
            transform: Box::new(|| Transform::identity()),
        }
    }

    pub unsafe fn stack(bot: &str, top: &str, atlas: &Atlas) -> Typeset {
        let content = TextElement::stack(bot, top, atlas);
        let bbox = content.bbox;

        Typeset {
            content: Node::Text(Arc::new(RwLock::new(content))),
            bbox,
            transform: Box::new(|| Transform::identity()),
        }
    }

    pub unsafe fn elem(arc: Arc<RwLock<TextElement>>) -> Typeset {
        let bbox = arc.read().unwrap().bbox;
        Typeset {
            content: Node::Text(arc),
            bbox,
            transform: Box::new(|| Transform::identity()),
        }
    }

    pub unsafe fn seq(content: Vec<Typeset>) -> Typeset {
        let bbox = content
            .iter()
            // Take the bbox of each item.
            .map(|t| t.bbox)
            // And extend into one large tight containing box.
            .reduce(|t, s| t.extend(s))
            // If we got an empty seq, it has zero-sized bounds.
            .unwrap_or(Rect {
                x0: 0.0,
                x1: 0.0,
                y0: 0.0,
                y1: 0.0,
            });

        Typeset {
            content: Node::Seq(content),
            bbox,
            transform: Box::new(|| Transform::identity()),
        }
    }

    pub unsafe fn limits(from: Option<Typeset>, to: Option<Typeset>, around: Typeset) -> Typeset {
        let a_tr = (around.transform)();
        let bbox = Rect {
            x0: around.bbox.x0 * a_tr.scale,
            x1: around.bbox.x1 * a_tr.scale,
            y0: around.bbox.y0 * a_tr.scale,
            y1: around.bbox.y1 * a_tr.scale,
        };

        Typeset {
            content: Node::Limits {
                hi_limit: to.map(Arc::new),
                lo_limit: from.map(Arc::new),
                text: Arc::new(around),
            },
            bbox,
            transform: Box::new(|| Transform::identity()),
        }
    }

    pub unsafe fn integral(from: Option<Typeset>, to: Option<Typeset>, atlas: &Atlas) -> Self {
        // Adjust the kerning of the limits.
        // TODO: Actually  find out how latex adjusts this instead of these random values

        let from = from.map(|t| {
            t.transform(Box::new(|| Transform {
                scale: 1.0,
                translation: (-0.4, -0.15),
            }))
        });

        let to = to.map(|t| {
            t.transform(Box::new(|| Transform {
                scale: 1.0,
                translation: (0.3, 0.1),
            }))
        });

        // Create a text node with an integral symbol.
        let int = Typeset::stack("\u{2320}", "\u{2321}", atlas);

        // Typeset the integral symbol with the kerned limits.
        Typeset::limits(from, to, int).transform(Box::new(|| Transform {
            // Adjust the integral to match LaTeX approximately
            scale: 0.80,
            translation: (0.0, -0.7883),
        }))
    }

    pub unsafe fn traverse_scenegraph(
        &self,
        renderer: &TextRenderer,
        transform_so_far: &Transform,
        text_shader: &TextShader,
    ) {
        let transform = transform_so_far.compose(&(self.transform)());
        match &self.content {
            // For a simple text box, simply rasterize it.
            Text(arc) => {
                let text = arc.read().unwrap();
                text.rasterize(renderer, &transform, text_shader);
            }
            // For a sequence of text object defined in their own coordinate system,
            // we need to iterate and drawthem offset corrctly relative to the leftmost
            // text element.
            Seq(texts) => {
                let mut dx = 0.0;
                for text in texts {
                    // Apply transform to the left side (essentially preserces kerning between
                    // elements)
                    let text_tr = (text.transform)();
                    dx += transform.scale * text_tr.scale * text.bbox.x0;
                    let transform = transform.translate(dx.round(), 0.0);
                    text.traverse_scenegraph(renderer, &transform, text_shader);
                    // Apply transformation past the text element.
                    dx += transform.scale * text_tr.scale * text.bbox.x1;
                }
            }
            // For an object with limits, we need to calculate affine transforms for the
            // top- and bottom limit respectively, and recursively rasterize them in their
            // correct coordinate system.
            Limits {
                lo_limit: lo_opt,
                hi_limit: hi_opt,
                text: int,
            } => {
                int.traverse_scenegraph(renderer, &transform, text_shader);

                // Limit scale-down factor.
                let ds = 0.7;

                // this stuff is probably more elegant to do as compisition

                if let Some(lim) = lo_opt {
                    // Center the limit under the text
                    let dx = transform.scale
                        * (self.bbox.x0 + self.bbox.width() / 2.0 - ds * lim.bbox.width() / 2.0);

                    // Position it directly under
                    let dy = transform.scale * (self.bbox.y0 - ds * lim.bbox.height());

                    let transform = transform
                        // Translate in the parents reference.
                        .translate(dx, dy)
                        // And then scale down.
                        .scale(ds);

                    lim.traverse_scenegraph(renderer, &transform, text_shader);
                }

                if let Some(lim) = hi_opt {
                    // Center the limit over the text
                    let dx = transform.scale
                        * (self.bbox.x0 + self.bbox.width() / 2.0 - ds * lim.bbox.width() / 2.0);

                    // Position directly above
                    let dy = transform.scale * (self.bbox.y1 - ds * lim.bbox.y0);

                    let transform = transform
                        // Translate in the parents reference.
                        .translate(dx, dy)
                        // And then scale down.
                        .scale(ds);

                    lim.traverse_scenegraph(renderer, &transform, text_shader);
                }
            }
        }
    }
}
