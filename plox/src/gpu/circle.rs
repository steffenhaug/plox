use crate::gpu::{self, shader::*, Transform, Vao};

pub struct CircleRenderer {
    phantom_vao: Vao<0>,
    circle_shader: Shader,
    u_width: UniformFloat,
    u_radius: UniformFloat,
    u_arc: UniformVec2,
    u_mvp: UniformMat4,
}

pub struct CircleElement {
    width: f32,
    radius: f32,
    arc: (f32, f32),
}

impl CircleElement {
    pub fn new(r: f32) -> CircleElement {
        CircleElement {
            width: 1.0,
            radius: r,
            arc: (-f32::INFINITY, f32::INFINITY)
        }
    }

    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }

    pub fn arc(mut self, t1: f32, t2: f32) -> Self {
        self.arc = (t1, t2);
        self
    }

    pub unsafe fn rasterize(&self, renderer: &CircleRenderer, transform: &Transform) {
        // Compute the circles transform.
        let Transform {
            translation: (x, y),
            ..
        } = *transform;
        let (.., vp_w, vp_h) = gpu::gl_viewport();
        let window_projection = glm::ortho(0.0, vp_w as f32, 0.0, vp_h as f32, 0.0, 100.0);
        let model_matrix = glm::translation(&glm::vec3(x.floor(), y.floor(), 0.0));
        let window_mvp = window_projection * model_matrix;

        // Bind the empty VAO and the circle shader.
        renderer.phantom_vao.bind();
        renderer.circle_shader.bind();

        // Set up uniforms.
        renderer.u_width.data(self.width);
        renderer.u_radius.data(self.radius);
        renderer.u_arc.data(self.arc.0, self.arc.1);
        renderer.u_mvp.data(&window_mvp);

        // Draw a quad.
        gl::DrawArrays(gl::TRIANGLES, 0, 6);
    }
}

impl CircleRenderer {
    pub unsafe fn new() -> Self {
        let shader = Shader::circle();
        let u_radius = shader.uniform("radius");
        let u_width = shader.uniform("width");
        let u_mvp = shader.uniform("mvp");
        let u_arc = shader.uniform("arc");

        CircleRenderer {
            phantom_vao: Vao::gen(),
            circle_shader: shader,
            u_width,
            u_radius,
            u_arc,
            u_mvp,
        }
    }
}
