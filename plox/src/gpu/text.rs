//! # Text Renderer implementation.
use crate::atlas::{Atlas, Outline};
use crate::gpu::{shader::*, MultisampleTexture, Render, Vao, Vbo};
use crate::gpu::typeset::TypesetText;
use crate::spline::Rect;
use std::sync::{Arc, RwLock};

pub struct TextRenderer {
    // Text elements. (Scene graph)
    content: Vec<TypesetText>,
    // α-texture
    tex: MultisampleTexture,
    fbuf: u32,
    // Shaders to draw the fill and outline of the α-texture.
    fill: Shader,
    fill_mvp: UniformMat4,
    outline: Shader,
    outline_mvp: UniformMat4,
    // Shader to blit the α-texture.
    sample: Shader,
    sample_mvp: UniformMat4,
    u_tex_dims: UniformVec2i,
    u_coverage: UniformVec2,
    u_bbox: UniformVec4,
}

/// Buffer oject + transform.
pub struct TextElement {
    // Transform can compute a transform based on the application state.
    // This is a bit spahetti, but it makes animation possible.
    pub bbox: Rect,
    vao: Vao<1>,
    vbo: Vbo,
    n: u32,
}

/// The Arc-type allows animating by mutating the scene graph externally.
pub type SharedText = Arc<RwLock<TextElement>>;

impl TextElement {
    pub unsafe fn rasterize(
        &self,
        // The renderer to draw with (Shaders for α-texture)
        renderer: &TextRenderer,
        // The state the renderer was invoked with.
        state: &TextRendererState,
        transform: &Transform,
    ) {
        self.vao.bind();

        //
        // Some preliminary coordinate transform calculations.
        //

        // Scale is how many pixels tall the text is.
        // Translation is position in pixel coordinates.
        let Transform {
            scale,
            translation: (x, y),
        } = *transform;

        // Coordinates in pixels.
        let bbox = self.bbox;
        let (x0, x1) = ((scale * bbox.x0).floor(), (scale * bbox.x1).ceil());
        let (y0, y1) = ((scale * bbox.y0).floor(), (scale * bbox.y1).ceil());

        // The width and height (in pixels) of the quad.
        let w = x1 - x0;
        let h = y1 - y0;

        // This may extend past the window. We want to clamp it so OpenGL can
        // clip letters that are outside the texture.
        let tw = f32::min(w, 800.0);
        let th = f32::min(h, 800.0);

        // Projects the text element onto the texture.
        let texture_projection = glm::ortho(x0, x0 + tw, y0, y0 + th, 0.0, 100.0);
        let texture_scale = glm::scaling(&glm::vec3(scale, scale, 0.0));
        let texture_mvp = texture_projection * texture_scale;

        //
        // Rasterize α-texture.
        //

        // Look at a correctly sized box in the texture
        gl::BindFramebuffer(gl::FRAMEBUFFER, renderer.fbuf);
        gl::Viewport(0, 0, tw as i32, th as i32);

        // Start with 100% transparent texture.
        gl::ClearColor(0.0, 0.0, 0.0, 0.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);

        // Enable XOR flipping. (Explanation in report)
        gl::Enable(gl::COLOR_LOGIC_OP);
        gl::LogicOp(gl::XOR);

        // Draw the fill of the glyphs.
        renderer.fill.bind();
        let u_mvp = &renderer.fill_mvp;
        u_mvp.data(&texture_mvp);
        gl::DrawArrays(gl::TRIANGLES, 0, self.n as i32);

        // Finish the outline of the glyphs.
        renderer.outline.bind();
        let u_mvp = &renderer.outline_mvp;
        u_mvp.data(&texture_mvp);
        gl::DrawArrays(gl::TRIANGLES, 0, self.n as i32);

        // Render to the window again.
        let (win_w, win_h) = state.win_dims;
        gl::Disable(gl::COLOR_LOGIC_OP);
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        gl::Viewport(0, 0, win_w as i32, win_h as i32);

        //
        // Draw a quad sampling the α-texture.
        //

        renderer.sample.bind();

        // Submit data about the quad.
        renderer.u_bbox.data(x0, y0, x1, y1);
        renderer.u_coverage.data(w / tw, h / th);
        renderer.u_tex_dims.data(tw as i32, th as i32);

        // Window projection.
        let window_projection = glm::ortho(0.0, win_w as f32, 0.0, win_h as f32, 0.0, 100.0);
        let model_matrix = glm::translation(&glm::vec3(x, y, 0.0));
        let window_mvp = window_projection * model_matrix;
        let u_mvp = &renderer.sample_mvp;
        u_mvp.data(&window_mvp);

        // Bind the texture we just rasterized to.
        renderer.tex.bind();

        // Draw it on a quad.
        gl::DrawArrays(gl::TRIANGLES, 0, 6);
    }

    pub unsafe fn update(&mut self, input: &str, atlas: &Atlas) {
        let Outline { ctrl_pts, bbox } = atlas.outline(input);
        self.vbo.data(&ctrl_pts);
        self.bbox = bbox;
        self.n = ctrl_pts.len() as u32;
    }

    /// Sets up all the buffers to prepare for pushing data.
    pub unsafe fn new(input: &str, atlas: &Atlas) -> Self {
        let Outline { ctrl_pts, bbox } = atlas.outline(input);

        let vao = Vao::<1>::gen();
        vao.enable_attrib_arrays();

        let vbo = Vbo::gen();
        vbo.data(&ctrl_pts);

        vao.attrib_ptr(0, 2, gl::FLOAT);

        TextElement {
            bbox,
            vao,
            vbo,
            n: ctrl_pts.len() as u32,
        }
    }
}

pub struct TextRendererState {
    // The current dimensions of the window.
    pub win_dims: (u32, u32),
}

impl Render for TextRenderer {
    type State = TextRendererState;
    unsafe fn invoke(&self, state: &Self::State) {
        for text in &self.content {
            text.rasterize(self, state);
        }
    }
}

impl TextRenderer {
    pub unsafe fn new() -> Self {
        //
        // Compile shaders.
        //
        let fill = Shader::fill();
        let fill_mvp = fill.uniform("mvp");

        let outline = Shader::outline();
        let outline_mvp = fill.uniform("mvp");

        let sample = Shader::sample();
        let u_tex_dims = sample.uniform("tex_dims");
        let u_coverage = sample.uniform("coverage");
        let u_bbox = sample.uniform("bbox");
        let sample_mvp = sample.uniform("mvp");

        //
        // Set up α-texture. (See report for what this does)
        //
        let tex = MultisampleTexture::alpha(4096, 4096);

        let mut fbuf = 0;
        gl::GenFramebuffers(1, &mut fbuf);
        gl::BindFramebuffer(gl::FRAMEBUFFER, fbuf);

        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D_MULTISAMPLE,
            tex.tex,
            0,
        );

        if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
            panic!("α-texture framebuffer incomplete");
        }

        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

        TextRenderer {
            fill,
            fill_mvp,
            outline,
            outline_mvp,
            sample,
            sample_mvp,
            u_tex_dims,
            u_coverage,
            u_bbox,
            tex,
            fbuf,
            content: vec![],
        }
    }

    pub fn submit(&mut self, text: TypesetText) {
        self.content.push(text);
    }
}

// Really, this could just be a glm::mat4
pub struct Transform {
    pub scale: f32,
    pub translation: (f32, f32),
}

impl Transform {
    fn identity() -> Box<dyn Fn() -> Transform> {
        Box::new(|| Transform {
            scale: 1.0,
            translation: (0.0, 0.0),
        })
    }
}
