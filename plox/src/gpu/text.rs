//! # Text Renderer implementation.
use crate::atlas::{Atlas, Outline};
use crate::gpu::typeset::Typeset;
use crate::gpu::{shader::*, MultisampleTexture, Render, Vao, Vbo};
use crate::spline::{Point, Rect};
use std::sync::{Arc, RwLock};

pub struct TextRenderer {
    // Text elements. (Scene graph)
    content: Vec<Typeset>,
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

        // Scale is how many pixels tall the text is. (1em in pixels)
        // Translation is position in pixel coordinates.
        let Transform {
            scale,
            translation: (x, y),
        } = *transform;

        // Bounding box coordinates in pixels.
        let bbox = self.bbox;
        let (x0, x1) = ((scale * bbox.x0).floor(), (scale * bbox.x1).ceil());
        let (y0, y1) = ((scale * bbox.y0).floor(), (scale * bbox.y1).ceil());

        // The width and height (again, in pixels) of the quad.
        let w = x1 - x0;
        let h = y1 - y0;

        // Issue: In theory the bbox might extend past the texture.
        // This can be "fixed" by using a massive texture. This will invoke fragment processing of
        // off-screen fragments, but it is actually not as costly as you would think, since
        // the fragment shader is cheap, and the vertex processing (where the magic happens) has
        // to be done anyways.

        let tw = 4096.0;
        let th = 4096.0;

        // Projects the text element onto the (4K) texture.
        let texture_projection = glm::ortho(x0, x0 + tw, y0, y0 + th, 0.0, 100.0);
        let texture_scale = glm::scaling(&glm::vec3(scale, scale, 0.0));
        let texture_mvp = texture_projection * texture_scale;

        //
        // Rasterize α-texture.
        //

        // Look at a correctly sized box in the texture.
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
        gl::Disable(gl::COLOR_LOGIC_OP);
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        let (win_w, win_h) = state.win_dims;
        gl::Viewport(0, 0, win_w as i32, win_h as i32);

        //
        // Draw a quad sampling the α-texture.
        //

        renderer.sample.bind();

        // Submit data about the quad.
        renderer.u_bbox.data(x0, y0, x1, y1);
        renderer.u_tex_dims.data(w as i32, h as i32);

        // Window projection.
        let window_projection = glm::ortho(0.0, win_w as f32, 0.0, win_h as f32, 0.0, 100.0);
        let model_matrix = glm::translation(&glm::vec3(x.floor(), y.floor(), 0.0));
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

    /// Create a single "pseudo-glyph" out of stacked symbols.
    /// For example: Stack \u{2320} and \u{2321} to create a tall integral.
    pub unsafe fn stack(top: &str, bot: &str, atlas: &Atlas) -> Self {
        let top = atlas.outline(&top);
        let mut bot = atlas.outline(&bot);
        let dy = bot.bbox.y1;
        bot.ctrl_pts.extend(
            top.ctrl_pts
                .into_iter()
                // Move the top glyph ctrl pts above the bottom glyph.
                .map(|(x, y)| (x, y + dy)),
        );

        let bbox = bot.bbox.extend(Rect {
            x0: top.bbox.x0,
            x1: top.bbox.x1,
            y0: top.bbox.y0 + dy,
            // We just need a _tiny_ bit more space so the addition plays nicely with rounding and
            // anti-aliasing.
            y1: top.bbox.y1 + dy + 1e-3,
        });

        let vao = Vao::<1>::gen();
        vao.enable_attrib_arrays();

        let vbo = Vbo::gen();
        vbo.data(&bot.ctrl_pts);

        vao.attrib_ptr(0, 2, gl::FLOAT);

        TextElement {
            bbox,
            vao,
            vbo,
            n: bot.ctrl_pts.len() as u32,
        }
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
            text.rasterize(self, state, &Transform::identity());
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
            u_bbox,
            tex,
            fbuf,
            content: vec![],
        }
    }

    pub fn submit(&mut self, text: Typeset) {
        self.content.push(text);
    }
}

// Really, this could just be a glm::mat4
#[derive(Debug)]
pub struct Transform {
    pub scale: f32,
    pub translation: (f32, f32),
}

impl Transform {
    pub fn identity() -> Self {
        Transform {
            scale: 1.0,
            translation: (0.0, 0.0),
        }
    }

    /// Compose two transforms.
    pub fn compose(&self, rhs: &Transform) -> Self {
        let dx = self.scale * rhs.translation.0 + self.translation.0;
        let dy = self.scale * rhs.translation.1 + self.translation.1;
        Transform {
            scale: self.scale * rhs.scale,
            translation: (dx, dy),
        }
    }

    pub fn translate(&self, dx: f32, dy: f32) -> Transform {
        Transform {
            scale: self.scale,
            translation: (self.translation.0 + dx, self.translation.1 + dy),
        }
    }

    pub fn scale(&self, s: f32) -> Transform {
        Transform {
            scale: s * self.scale,
            translation: (self.translation.0, self.translation.1),
        }
    }
}
