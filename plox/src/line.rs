use crate::gpu::shader::{Shader, UniformFloat, UniformMat4};
use crate::gpu::{self, Ibo, Transform, Vao, Vbo};
use crate::tesselate::tesselate;
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
    pub line_shader: Shader,
    u_mvp: UniformMat4,
    u_width: UniformFloat,
}

pub struct LineElement {
    vao: Vao<2>,
    vbo: Vbo,
    ibo: Ibo,
    n_segments: u32,
    width: f32,
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

impl LineElement {
    pub unsafe fn rasterize(
        &self,
        renderer: &LineRenderer,
        transform: &Transform,
        line_shader: &Shader,
    ) {
        self.vao.bind();
        self.ibo.bind();

        let Transform {
            translation: (x, y),
            ..
        } = *transform;

        let (.., vp_w, vp_h) = gpu::gl_viewport();
        let proj = glm::ortho(0.0, vp_w as f32, 0.0, vp_h as f32, 0.0, 100.0);
        let model = glm::translation(&glm::vec3(x.floor(), y.floor(), 0.0));

        line_shader.bind();

        renderer.u_mvp.data(&(proj * model));
        renderer.u_width.data(self.width);

        gl::DrawElements(
            gl::TRIANGLES,
            self.n_segments as i32 * 6,
            gl::UNSIGNED_INT,
            std::ptr::null(),
        );
    }

    pub unsafe fn line(from: Vec2, to: Vec2, width: f32) -> Self {
        Self::new([Segment { p1: from, p2: to }].iter(), width)
    }

    pub unsafe fn new<'a, S>(segments: S, width: f32) -> Self
    where
        S: Iterator<Item = &'a Segment>,
    {
        let (vs, uvs, idx) = tesselate(segments, width);

        let vao = Vao::gen();
        vao.enable_attrib_arrays();

        let vbo = Vbo::gen();
        vbo.data(&vs);
        vao.attrib_ptr(0, 2, gl::FLOAT);

        let vbo = Vbo::gen();
        vbo.data(&uvs);
        vao.attrib_ptr(1, 2, gl::FLOAT);

        let ibo = Ibo::gen();
        ibo.data(&idx);

        LineElement {
            vao,
            vbo,
            ibo,
            n_segments: vs.len() as u32 / 2 - 1,
            width,
        }
    }
}

impl LineRenderer {
    pub unsafe fn new() -> Self {
        let shader = Shader::line();
        let u_mvp = shader.uniform("mvp");
        let u_width = shader.uniform("width");

        LineRenderer {
            line_shader: shader,
            u_mvp,
            u_width,
        }
    }
}
