//! # Text Renderer implementation.
use crate::gpu::{
    self,
    shader::{Shader, UniformMat4},
    Render, Vao, Ibo, Vbo, Ssbo
};
use crate::shaping::{self, Glyph};
use crate::font;
use crate::atlas::Atlas;

pub struct TextRenderer {
    shader: Shader,
    vao: Vao<3>,
    model_matrix_u: UniformMat4,
    proj_matrix_u: UniformMat4,
    n_quads: u32,
}

pub struct TextRendererState {
    // The current dimensions of the window.
    pub win_dims: (u32, u32),
    // The mouse cursor position in the window if the mouse is in the window.
    pub mouse: Option<(f32, f32)>,
}

impl Render for TextRenderer {
    type State = TextRendererState;
    unsafe fn invoke(&self, state: &Self::State) {
        self.vao.bind();
        self.shader.bind();

        let (mx, my) = state.mouse.unwrap_or((400.0, 400.0));
        let m: glm::Mat4 = glm::translation(&glm::vec3(mx, my, 0.0))
            * glm::rotation(3.1415 / 3.0, &glm::vec3(0.0, 0.0, 1.0))
            * glm::scaling(&glm::vec3(75.0, 75.0, 0.0));

        // Compute projection matrix.
        let (w, h) = state.win_dims;
        let p: glm::Mat4 = glm::ortho(0.0, w as f32, 0.0, h as f32, 0.0, 1000.0);

        // Todo: abstract this (send matrices to the shader program)
        gl::UniformMatrix4fv(self.model_matrix_u.0, 1, 0, m.as_ptr());
        gl::UniformMatrix4fv(self.proj_matrix_u.0, 1, 0, p.as_ptr());

        gpu::draw_quads(self.n_quads);
    }
}

impl TextRenderer {
    pub unsafe fn new() -> Self {
        let shader = Shader::text_shader();
        let model_matrix_u = shader.uniform_mat4("model");
        let proj_matrix_u = shader.uniform_mat4("proj");

        let bef = std::time::Instant::now();
        let atlas = Atlas::new(&font::LM_MATH);
        let aft = std::time::Instant::now();
        println!("Outlining time = {}ms", (aft - bef).as_millis());

        // send atlas to the GPU
        let beziers_buf = Ssbo::gen();
        beziers_buf.data(&atlas.outlines);
        beziers_buf.bind_base(0);

        let lut_buf = Ssbo::gen();
        lut_buf.data(&atlas.lut);
        lut_buf.bind_base(1);

        //
        // create vertex array
        //
        let input = "\u{2207}\u{03B1} = \u{222B}\u{1D453}d\u{03BC}";
        let bef = std::time::Instant::now();
        let text = shaping::shape(input, &font::LM_MATH);
        let aft = std::time::Instant::now();
        println!("Shaping time = {}ms", (aft - bef).as_millis());

        let vao = Vao::<3>::gen();
        vao.enable_attrib_arrays();

        let (n_quads, pos, uv, id, idx) = vertex_buffer(&text);

        let vb_pos = Vbo::gen();
        vb_pos.data(&pos);
        vao.attrib_ptr(0, 2, gl::FLOAT);

        let vb_uv = Vbo::gen();
        vb_uv.data(&uv);
        vao.attrib_ptr(1, 2, gl::FLOAT);

        let vb_id = Vbo::gen();
        vb_id.data(&id);
        vao.attrib_iptr(2, 1, gl::UNSIGNED_INT);

        let ibo = Ibo::gen();
        ibo.data(&idx);

        // Synchronize. (just for profiling purposes).
        gl::Finish();

        TextRenderer {
            shader,
            vao,
            model_matrix_u,
            proj_matrix_u,
            n_quads,
        }
    }
}

fn vertex_buffer(
    glyphs: &Vec<Glyph>,
) -> (u32, Vec<(f32, f32)>, Vec<(f32, f32)>, Vec<u32>, Vec<u32>) {
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

    (n, positions, uvs, ids, idx)
}
