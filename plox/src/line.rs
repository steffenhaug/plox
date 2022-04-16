use crate::gpu::shader::{Shader, UniformFloat, UniformMat4};
use crate::gpu::{self, Ibo, Vao, Vbo};
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
    pub default_line_shader: LineShader,
}

pub struct LineShader {
    shader: Shader,
    u_mvp: UniformMat4,
    u_width: UniformFloat,
}

impl From<Shader> for LineShader {
    fn from(shader: Shader) -> Self {
        unsafe {
            let u_mvp = shader.uniform("mvp");
            let u_width = shader.uniform("width");

            LineShader {
                shader,
                u_mvp,
                u_width
            }
        }
    }
}

pub struct LineElement {
    vao: Vao<2>,
    pos_vbo: Vbo,
    uv_vbo: Vbo,
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
        trans: glm::Vec2,
        line_shader: &LineShader,
    ) {
        self.vao.bind();
        self.ibo.bind();

        let (x, y) = (trans.x, trans.y);

        let (.., vp_w, vp_h) = gpu::gl_viewport();
        let proj = glm::ortho(0.0, vp_w as f32, 0.0, vp_h as f32, 0.0, 100.0);
        let model = glm::translation(&glm::vec3(x.floor(), y.floor(), 0.0));

        line_shader.shader.bind();

        renderer.default_line_shader.u_mvp.data(&(proj * model));
        renderer.default_line_shader.u_width.data(self.width);

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

    pub unsafe fn update_line(&mut self, from: Vec2, to: Vec2, width: f32) {
        self.update([Segment { p1: from, p2: to }].iter(), width);
    }

    pub unsafe fn update<'a, S>(&mut self, segments: S, width: f32)
    where
        S: Iterator<Item = &'a Segment>,
    {
        let (vs, uvs, idx) = tesselate(segments, width);
        self.pos_vbo.data(&vs);
        self.uv_vbo.data(&uvs);
        self.ibo.data(&idx);
        self.n_segments = vs.len() as u32 / 2 - 1;
    }

    pub unsafe fn new<'a, S>(segments: S, width: f32) -> Self
    where
        S: Iterator<Item = &'a Segment>,
    {
        let vao = Vao::gen();
        vao.enable_attrib_arrays();

        let pos_vbo = Vbo::gen();
        pos_vbo.bind();
        vao.attrib_ptr(0, 2, gl::FLOAT);

        let uv_vbo = Vbo::gen();
        uv_vbo.bind();
        vao.attrib_ptr(1, 2, gl::FLOAT);

        let ibo = Ibo::gen();

        let mut li = LineElement {
            vao,
            pos_vbo,
            uv_vbo,
            ibo,
            n_segments: 0,
            width,
        };

        li.update(segments, width);

        li
    }
}

impl LineRenderer {
    pub unsafe fn new() -> Self {
        let shader = Shader::line();
        LineRenderer {
            default_line_shader: shader.into(),
        }
    }
}
