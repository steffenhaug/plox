//! # Text Renderer implementation.
use crate::atlas::Atlas;
use crate::font;
use crate::gpu::{
    self,
    shader::{Shader, UniformMat4},
    Ibo, Render, Ssbo, Vao, Vbo,
};
use crate::shaping::{self, Glyph};

pub struct TextRenderer {
    shader: Shader,
    model_matrix_u: UniformMat4,
    proj_matrix_u: UniformMat4,
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
    // Vertex layout: (pos, uv, id)
    vao: Vao<3>,
    vbos: (Vbo, Vbo, Vbo),
    ibo: Ibo,
    n_quads: u32,
}

impl TextElement {
    pub fn with_transform(mut self, f: impl 'static + Fn(&TextRendererState) -> Transform) -> Self {
        self.transform = Box::new(f);
        self
    }

    /// Sets up all the buffers to prepare for pushing data.
    pub unsafe fn new() -> Self {
        let vao = Vao::<3>::gen();
        vao.enable_attrib_arrays();

        let vb_pos = Vbo::gen();
        vb_pos.bind();
        vao.attrib_ptr(0, 2, gl::FLOAT);

        let vb_uv = Vbo::gen();
        vb_uv.bind();
        vao.attrib_ptr(1, 2, gl::FLOAT);

        let vb_id = Vbo::gen();
        vb_id.bind();
        vao.attrib_iptr(2, 1, gl::UNSIGNED_INT);

        let ibo = Ibo::gen();

        TextElement {
            transform: Transform::identity(),
            vao,
            vbos: (vb_pos, vb_uv, vb_id),
            ibo,
            n_quads: 0,
        }
    }

    /// Push data to the GPU.
    pub unsafe fn data(&mut self, glyhps: &Vec<Glyph>) {
        let (pos, uv, id, idx, n) = vertex_data(glyhps);
        let (pos_buf, uv_buf, id_buf) = &self.vbos;

        // Set vertex data.
        pos_buf.data(&pos);
        uv_buf.data(&uv);
        id_buf.data(&id);

        // Set index data.
        self.ibo.data(&idx);

        // Set the number of quads.
        self.n_quads = n;
    }
}

impl Render for TextRenderer {
    type State = TextRendererState;
    unsafe fn invoke(&self, state: &Self::State) {
        // Bind the text shader.
        self.shader.bind();

        // Compute projection matrix.
        let (w, h) = state.win_dims;
        let p: glm::Mat4 = glm::ortho(0.0, w as f32, 0.0, h as f32, 0.0, 1000.0);

        for txt in &self.content {
            txt.vao.bind();

            let Transform {
                scale,
                translation: (x, y),
                ..
            } = (txt.transform)(state);

            let m: glm::Mat4 = glm::translation(&glm::vec3(x, y, 0.0))
                * glm::scaling(&glm::vec3(scale, scale, 0.0));

            // Todo: abstract this (send matrices to the shader program)
            gl::UniformMatrix4fv(self.model_matrix_u.0, 1, 0, m.as_ptr());
            gl::UniformMatrix4fv(self.proj_matrix_u.0, 1, 0, p.as_ptr());
            gpu::draw_quads(txt.n_quads);
        }
    }
}

impl TextRenderer {
    pub unsafe fn new() -> Self {
        let shader = Shader::text_shader();
        let model_matrix_u = shader.uniform_mat4("model");
        let proj_matrix_u = shader.uniform_mat4("proj");

        let atlas = Atlas::new(&font::LM_MATH);

        //
        // Font atlas
        //

        let beziers_buf = Ssbo::gen();
        beziers_buf.data(&atlas.outlines);
        beziers_buf.bind_base(0);

        let lut_buf = Ssbo::gen();
        lut_buf.data(&atlas.lut);
        lut_buf.bind_base(1);

        // Create a text element
        let input = "\u{2207}\u{03B1} = \u{222B}\u{1D453}d\u{03BC}";
        let glyphs = shaping::shape(input, &font::LM_MATH);
        let mut text = TextElement::new().with_transform(|state| Transform {
            scale: 80.0,
            translation: state
                .mouse
                .map_or((400.0, 400.0), |(mx, my)| (mx as f32, my as f32)),
        });
        text.data(&glyphs);

        let glyphs = shaping::shape("Multiple text element test", &font::LM_MATH);
        let mut heading = TextElement::new().with_transform(|_| Transform {
            scale: 60.0,
            translation: (80.0, 700.0)
        });
        heading.data(&glyphs);

        TextRenderer {
            shader,
            model_matrix_u,
            proj_matrix_u,
            content: vec![heading, text],
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
                glyph.glyph_id,
                glyph.glyph_id,
                glyph.glyph_id,
                glyph.glyph_id,
            ]
        })
        .collect();

    let idx = (0..glyphs.len() as u32)
        .flat_map(|i| {
            let offset = 4 * i;
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
