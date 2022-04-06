use self::Node::*;
use crate::atlas::Atlas;
use crate::gpu::text::{SharedText, TextElement, TextRenderer, TextRendererState, Transform};
use crate::spline::Rect;
use std::sync::{Arc, RwLock};

pub enum Node {
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
}

/// A Typeset text element is essentially a scene graph.
pub struct Typeset {
    pub content: Node,
    pub bbox: Rect,
    // Transform relative to the parent node.
    pub transform: Transform,
}

impl Typeset {
    pub fn transform(mut self, t: Transform) -> Self {
        self.transform = t;
        self
    }

    pub fn raise(mut self, dy: f32) -> Self {
        self.transform.translation.1 += dy;
        self
    }

    pub fn scale(mut self, s: f32) -> Self {
        self.transform.scale *= s;
        self
    }

    pub unsafe fn text(txt: &str, atlas: &Atlas) -> Self {
        let content = TextElement::new(txt, atlas);
        let bbox = content.bbox;
        Typeset {
            content: Node::Text(Arc::new(RwLock::new(content))),
            bbox,
            transform: Transform::identity(),
        }
    }

    pub unsafe fn stack(bot: &str, top: &str, atlas: &Atlas) -> Typeset {
        let content = TextElement::stack(bot, top, atlas);
        let bbox = content.bbox;

        Typeset {
            content: Node::Text(Arc::new(RwLock::new(content))),
            bbox,
            transform: Transform::identity(),
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
            transform: Transform::identity(),
        }
    }

    pub unsafe fn limits(from: Option<Typeset>, to: Option<Typeset>, around: Typeset) -> Typeset {
        let bbox = Rect {
            x0: around.bbox.x0 * around.transform.scale,
            x1: around.bbox.x1 * around.transform.scale,
            y0: around.bbox.y0 * around.transform.scale,
            y1: around.bbox.y1 * around.transform.scale,
        };

        Typeset {
            content: Node::Limits {
                hi_limit: to.map(Arc::new),
                lo_limit: from.map(Arc::new),
                text: Arc::new(around),
            },
            bbox,
            transform: Transform::identity(),
        }
    }

    pub unsafe fn integral(from: Option<Typeset>, to: Option<Typeset>, atlas: &Atlas) -> Self {
        // Adjust the kerning of the limits.
        // TODO: Actually  find out how latex adjusts this instead of these random values

        let from = from.map(|t| {
            t.transform(Transform {
                scale: 1.0,
                translation: (-0.4, -0.15),
            })
        });

        let to = to.map(|t| {
            t.transform(Transform {
                scale: 1.0,
                translation: (0.3, 0.1),
            })
        });

        // Create a text node with an integral symbol.
        let int = Typeset::stack("\u{2320}", "\u{2321}", atlas);

        // Typeset the integral symbol with the kerned limits.
        Typeset::limits(from, to, int)
            // Adjust the integral to match LaTeX approximately
            .raise(-0.7883)
            .scale(0.80)
    }

    pub unsafe fn rasterize(
        &self,
        renderer: &TextRenderer,
        state: &TextRendererState,
        transform_so_far: &Transform,
    ) {
        let transform = transform_so_far.compose(&self.transform);
        match &self.content {
            // For a simple text box, simply rasterize it.
            Text(arc) => {
                let text = arc.read().unwrap();
                text.rasterize(renderer, state, &transform);
            }
            // For a sequence of text object defined in their own coordinate system,
            // we need to iterate and drawthem offset corrctly relative to the leftmost
            // text element.
            Seq(texts) => {
                let mut dx = 0.0;
                for text in texts {
                    // Apply transform to the left side (essentially preserces kerning between
                    // elements)
                    dx += transform.scale * text.transform.scale * text.bbox.x0;
                    let transform = transform.translate(dx, 0.0);
                    text.rasterize(renderer, state, &transform);
                    // Apply transformation past the text element.
                    dx += transform.scale * text.transform.scale * text.bbox.x1;
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
                int.rasterize(renderer, state, &transform);

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

                    lim.rasterize(renderer, state, &transform);
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

                    lim.rasterize(renderer, state, &transform);
                }
            }
        }
    }
}
