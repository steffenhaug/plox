//! # Text Renderer implementation.
use crate::atlas::{Atlas, Outline};
use crate::font;
use crate::gpu::{
    shader::{Shader, UniformMat4, UniformVec2, UniformVec2i, UniformVec4},
    MultisampleTexture, Render, Vao, Vbo,
};
use crate::shaping::Glyph;
use crate::spline::Rect;

pub struct TextRenderer {
    // Text elements. (Scene graph)
    content: Vec<TextElement>,
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

/// We need some info about the window to generate orthographic projection matrix.
pub struct TextRendererState {
    // The current dimensions of the window.
    pub win_dims: (u32, u32),
    // The mouse cursor position in the window if the mouse is in the window.
    pub mouse: Option<(f32, f32)>,
}

// Really, this could just be a glm::mat4
pub struct Transform {
    scale: f32,
    translation: (f32, f32),
}

impl Transform {
    fn identity<S>() -> Box<dyn Fn(&S) -> Transform> {
        Box::new(|_| Transform {
            scale: 1.0,
            translation: (0.0, 0.0),
        })
    }
}

/// Buffer oject + transform.
pub struct TextElement {
    // Transform can compute a transform based on the application state.
    // This is a bit spahetti, but it makes animation possible.
    transform: Box<dyn Fn(&TextRendererState) -> Transform>,
    bbox: Rect,
    vao: Vao<1>,
    _vbos: Vbo,
    n: u32,
}

impl TextElement {
    pub fn with_transform(mut self, f: impl 'static + Fn(&TextRendererState) -> Transform) -> Self {
        self.transform = Box::new(f);
        self
    }

    /// Sets up all the buffers to prepare for pushing data.
    pub unsafe fn new() -> Self {
        unimplemented!()
    }

    /// Push data to the GPU.
    pub unsafe fn data(&mut self, glyhps: &Vec<Glyph>) {
        unimplemented!()
    }
}

impl Render for TextRenderer {
    type State = TextRendererState;
    unsafe fn invoke(&self, state: &Self::State) {
        let text = &self.content[0];
        text.vao.bind();

        // Scale is how many pixels tall the text is.
        // Translation is position in pixel coordinates.
        let Transform {
            scale,
            translation: (x, y),
        } = (text.transform)(state);

        // Coordinates in pixels.
        let bbox = text.bbox;
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

        // Look at a correctly sized box in the texture
        gl::Viewport(0, 0, tw as i32, th as i32);

        //
        // Draw the glyphs alpha channel to a texture.
        //

        gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbuf);

        // Start with 100% transparent texture.
        gl::ClearColor(0.0, 0.0, 0.0, 0.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);

        // Enable XOR flipping. (Explanation in report)
        gl::Enable(gl::COLOR_LOGIC_OP);
        gl::LogicOp(gl::XOR);

        // Draw the fill of the glyphs.
        self.fill.bind();
        let u_mvp = &self.fill_mvp;
        u_mvp.data(&texture_mvp);
        gl::DrawArrays(gl::TRIANGLES, 0, text.n as i32);

        // Finish the outlines of the glyphs.
        self.outline.bind();
        let u_mvp = &self.outline_mvp;
        u_mvp.data(&texture_mvp);
        gl::DrawArrays(gl::TRIANGLES, 0, text.n as i32);

        // Disable the XOR flipping, and unbind the texture.
        gl::Disable(gl::COLOR_LOGIC_OP);
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

        // Set the viewport back to the window dimensions. Now we are drawing on the window again.
        let (win_w, win_h) = state.win_dims;
        gl::Viewport(0, 0, win_w as _, win_h as _);

        //
        // Draw a quad with the texture on it.
        //

        self.sample.bind();
        let u_mvp = &self.sample_mvp;

        // Submit the corners of the quad (in pixel coordinates).
        self.u_bbox.data(x0, y0, x1, y1);
        self.u_coverage.data(w / tw, h / th);

        // Set the dimensions of the texture.
        self.u_tex_dims.data(tw as i32, th as i32);

        // Window projection.
        let window_projection = glm::ortho(0.0, win_w as f32, 0.0, win_h as f32, 0.0, 100.0);
        let model_matrix = glm::translation(&glm::vec3(x, y, 0.0));
        let window_mvp = window_projection * model_matrix;
        u_mvp.data(&window_mvp);

        // Bind the texture we just rasterized to.
        gl::BindTexture(gl::TEXTURE_2D_MULTISAMPLE, self.tex.tex);

        // Draw it on a quad.
        gl::DrawArrays(gl::TRIANGLES, 0, 6);
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
        let mut fbuf = 0;
        gl::GenFramebuffers(1, &mut fbuf);
        gl::BindFramebuffer(gl::FRAMEBUFFER, fbuf);

        let mut tex = 0;
        gl::GenTextures(1, &mut tex);
        gl::BindTexture(gl::TEXTURE_2D_MULTISAMPLE, tex);

        gl::TexImage2DMultisample(
            gl::TEXTURE_2D_MULTISAMPLE,
            16,
            gl::R8 as _,
            800,
            800,
            gl::FALSE,
        );

        gl::BindTexture(gl::TEXTURE_2D_MULTISAMPLE, 0);

        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D_MULTISAMPLE,
            tex,
            0,
        );

        if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
            panic!("α-texture framebuffer incomplete");
        }

        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

        //
        // Outline some test text.
        //
        let atlas = Atlas::new(&font::LM_MATH);
        let input = "\u{2207}\u{03B1} = \u{222B}\u{1D453}d\u{03BC}";
        let Outline { ctrl_pts, bbox } = atlas.outline(input);

        let vao = Vao::<1>::gen();
        vao.enable_attrib_arrays();

        let vb_w = Vbo::gen();
        vb_w.data(&ctrl_pts);
        vao.attrib_ptr(0, 2, gl::FLOAT);

        let text = TextElement {
            transform: Box::new(|_| Transform {
                scale: 125.0,
                translation: (0.0, 100.0),
            }),
            bbox,
            vao,
            _vbos: vb_w,
            n: ctrl_pts.len() as u32,
        };

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
            tex: MultisampleTexture { tex },
            fbuf,
            content: vec![text],
        }
    }
}

/// Transform a buffer of glyphs to a bundle of GPU-ready buffers.
/// This step is ultimately unnecessary if we instead change the glyph buffer representation.
fn vertex_data(glyphs: &Vec<Glyph>) -> (Vec<(f32, f32)>, Vec<(f32, f32)>, Vec<u32>, Vec<u32>, u32) {
    let positions = glyphs
        .iter()
        .flat_map(|glyph| {
            [
                (glyph.x + glyph.bbox.x0, glyph.y + glyph.bbox.y0),
                (glyph.x + glyph.bbox.x1, glyph.y + glyph.bbox.y0),
                (glyph.x + glyph.bbox.x1, glyph.y + glyph.bbox.y1),
                (glyph.x + glyph.bbox.x0, glyph.y + glyph.bbox.y1),
            ]
        })
        .collect();

    let uvs = glyphs
        .iter()
        .flat_map(|glyph| {
            [
                (glyph.bbox.x0, glyph.bbox.y0),
                (glyph.bbox.x1, glyph.bbox.y0),
                (glyph.bbox.x1, glyph.bbox.y1),
                (glyph.bbox.x0, glyph.bbox.y1),
            ]
        })
        .collect();

    let ids = glyphs
        .iter()
        .flat_map(|glyph| {
            [
                glyph.glyph_id as _,
                glyph.glyph_id as _,
                glyph.glyph_id as _,
                glyph.glyph_id as _,
            ]
        })
        .collect();

    let idx = (0..glyphs.len())
        .flat_map(|i| {
            let offset = 4 * i as u32;
            [
                offset + 0, /* First triangle. */
                offset + 1,
                offset + 2,
                offset + 0, /* Second triangle. */
                offset + 2,
                offset + 3,
            ]
        })
        .collect();

    let n = glyphs.len() as u32;

    (positions, uvs, ids, idx, n)
}
