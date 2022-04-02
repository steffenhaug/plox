//! # Text Renderer implementation.
use crate::atlas::Atlas;
use crate::font;
use crate::gpu::{
    shader::{Shader, UniformMat4, UniformSampler2D, UniformVec2},
    Render, Vao, Vbo,
};
use crate::shaping::{self, Glyph};
use crate::spline::Rect;

pub struct TextRenderer {
    fill: Shader,
    fill_mvp: (UniformMat4, UniformMat4),
    outline: Shader,
    outline_mvp: (UniformMat4, UniformMat4),
    sample: Shader,
    sample_mvp: (UniformMat4, UniformMat4),
    u_tex: UniformSampler2D,
    u_bl: UniformVec2,
    u_tr: UniformVec2,
    u_max_uv: UniformVec2,
    tex: u32,
    fbuf: u32,
    vao: Vao<0>,
    content: Vec<TextElement>,
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
    // This is a bit spahetti, but it makes animation super easy.
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
        let Transform {
            scale,
            translation: (x, y),
        } = (text.transform)(state);

        // Coordinates in pixels
        let bbox = text.bbox;
        let (x0, x1) = ((scale * bbox.x0).floor(), (scale * bbox.x1).ceil());
        let (y0, y1) = ((scale * bbox.y0).floor(), (scale * bbox.y1).ceil());
        let w = x1 - x0;
        let h = y1 - y0;

        // Look at a correctly sized box in the texture
        gl::Viewport(0, 0, w as i32, h as i32);

        // Scale up to pixel-coordinates.
        let texture_scale = glm::scaling(&glm::vec3(scale, scale, 0.0));

        // Project onto the texture viewport.
        let texture_projection = glm::ortho(x0, x1, y0, y1, 0.0, 100.0);

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
        let (u_proj, u_model) = &self.fill_mvp;
        gl::UniformMatrix4fv(u_proj.0, 1, 0, texture_projection.as_ptr());
        gl::UniformMatrix4fv(u_model.0, 1, 0, texture_scale.as_ptr());
        gl::DrawArrays(gl::TRIANGLES, 0, text.n as i32);

        // Finish the outlines of the glyphs.
        self.outline.bind();
        let (u_proj, u_model) = &self.outline_mvp;
        gl::UniformMatrix4fv(u_proj.0, 1, 0, texture_projection.as_ptr());
        gl::UniformMatrix4fv(u_model.0, 1, 0, texture_scale.as_ptr());
        gl::DrawArrays(gl::TRIANGLES, 0, text.n as i32);

        // Disable the XOR flipping, and unbind the texture.
        gl::Disable(gl::COLOR_LOGIC_OP);
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        gl::Viewport(0, 0, state.win_dims.0 as _, state.win_dims.1 as _);

        // Draw a quad with the texture on it.
        self.sample.bind();
        let (u_proj, u_model) = &self.sample_mvp;

        let (win_w, win_h) = state.win_dims;
        let window_projection = glm::ortho(0.0, win_w as f32, 0.0, win_h as f32, 0.0, 100.0);
        let model_matrix =
            glm::translation(&glm::vec3(x, y, 0.0)) * glm::scaling(&glm::vec3(scale, scale, 0.0));

        gl::BindTexture(gl::TEXTURE_2D, self.tex);
        gl::Uniform2f(self.u_bl.0, bbox.x0, bbox.y0);
        gl::Uniform2f(self.u_tr.0, bbox.x1, bbox.y1);
        gl::Uniform2f(self.u_max_uv.0, w / 4096.0, h / 4096.0);
        gl::UniformMatrix4fv(u_proj.0, 1, 0, window_projection.as_ptr());
        gl::UniformMatrix4fv(u_model.0, 1, 0, model_matrix.as_ptr());
        gl::DrawArrays(gl::TRIANGLES, 0, 6);
    }
}

impl TextRenderer {
    pub unsafe fn new() -> Self {
        let fill = Shader::fill();
        let fill_mvp = (fill.uniform_mat4("p"), fill.uniform_mat4("m"));

        let outline = Shader::outline();
        let outline_mvp = (fill.uniform_mat4("p"), fill.uniform_mat4("m"));

        let sample = Shader::sample();
        let u_tex = sample.uniform_sampler2d("tex");
        let u_bl = sample.uniform_vec2("bl");
        let u_tr = sample.uniform_vec2("tr");
        let u_max_uv = sample.uniform_vec2("max_uv");
        let sample_mvp = (sample.uniform_mat4("p"), sample.uniform_mat4("m"));

        let atlas = Atlas::new(&font::LM_MATH);

        let input = "∫";
        let glyphs = shaping::shape(input, &font::LM_MATH);
        dbg!(&glyphs);
        let (beg, end) = atlas.lut[glyphs[0].glyph_id];

        let mut w: Vec<(f32, f32)> = Vec::with_capacity(100);

        for i in beg..end {
            let curve = atlas.outlines[i];
            w.push(curve.0.into());
            w.push(curve.1.into());
            w.push(curve.2.into());
        }

        let vao = Vao::<1>::gen();
        vao.enable_attrib_arrays();

        let vb_w = Vbo::gen();
        vb_w.data(&w);
        vao.attrib_ptr(0, 2, gl::FLOAT);

        let text = TextElement {
            transform: Box::new(|_| Transform {
                scale: 400.0,
                translation: (200.0, 200.0),
            }),
            bbox: glyphs[0].bbox,
            vao,
            _vbos: vb_w,
            n: w.len() as u32,
        };

        let mut fbuf = 0;
        gl::GenFramebuffers(1, &mut fbuf);
        gl::BindFramebuffer(gl::FRAMEBUFFER, fbuf);

        let mut tex = 0;
        gl::GenTextures(1, &mut tex);
        gl::BindTexture(gl::TEXTURE_2D, tex);

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::R8 as _,
            4096,
            4096,
            0,
            gl::RED,
            gl::UNSIGNED_BYTE,
            std::ptr::null(),
        );

        // Disable mipmapping
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as _);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as _);
        gl::FramebufferTexture(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, tex, 0);
        gl::DrawBuffers(1, &[gl::COLOR_ATTACHMENT0] as *const _);

        if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
            panic!("we fucked it");
        }

        // gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

        TextRenderer {
            fill,
            fill_mvp,
            outline,
            outline_mvp,
            sample,
            sample_mvp,
            u_tex,
            u_bl,
            u_tr,
            u_max_uv,
            tex,
            fbuf,
            vao: Vao::<0>::gen(),
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
