use crate::gpu::shader::{Shader, UniformMat4};
use glm::Vec2;

#[derive(Debug, Clone, Copy)]
pub struct Segment {
    pub p1: Vec2,
    pub p2: Vec2,
}

pub struct LinearSpline {
    lines: Vec<Segment>,
}

pub struct LineRenderer {
    line_shader: Shader,
    u_mvp: UniformMat4,
}

impl Segment {
    pub fn spline(segments: &[Vec2]) -> LinearSpline {
        LinearSpline {
            lines: segments
                .windows(2)
                .map(|ps| Segment {
                    p1: ps[0],
                    p2: ps[1],
                })
                .collect(),
        }
    }

    pub fn vec(&self) -> glm::Vec2 {
        self.p2 - self.p1
    }
}

impl LinearSpline {
    pub fn segments(&self) -> impl Iterator<Item = &Segment> {
        self.lines.iter()
    }

    pub fn close(self) -> Self {
        unimplemented!()
    }

    pub fn to(mut self, p: Vec2) -> Self {
        let segment = Segment {
            p1: self.lines[self.lines.len() - 1].p2,
            p2: p,
        };

        self.lines.push(segment);

        self
    }
}
