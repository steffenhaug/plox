use self::Node::*;
use crate::gpu::text::{SharedText, TextElement, TextRenderer, TextRendererState, Transform};
use crate::spline::Rect;
use std::sync::Arc;

pub enum Node {
    // Just plain old text. (Leaf node)
    Text(SharedText),
    // Sequence (multiple typeset text elements after one another)
    Seq(Vec<TypesetText>),
    // An integral symbol with optional limits.
    Integral {
        lo_limit: Option<Arc<TypesetText>>,
        hi_limit: Option<Arc<TypesetText>>,
        body: TextElement,
    },
}

/// A Typeset text element is essentially a scene graph.
pub struct TypesetText {
    pub content: Node,
    pub bbox: Rect,
    // Transform relative to the parent node.
    pub transform: Transform,
}

impl TypesetText {
    pub unsafe fn rasterize(&self, renderer: &TextRenderer, state: &TextRendererState) {
        match &self.content {
            Text(arc) => {
                let text = arc.read().unwrap();
                text.rasterize(renderer, state, &self.transform);
            }
            _ => unimplemented!(),
        }
    }
}
