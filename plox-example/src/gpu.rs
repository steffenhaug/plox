//! Defines interop with GPU, i. e. buffer types,
//! vertex types, and so on.
use crate::shader::{Shader, UniformMat4};
use crate::State;
use gl::types::*;
use plox::atlas::Atlas;
use plox::shaping::Glyph;
use std::ptr;

pub struct Vao<const N: u32> {
    array: GLuint,
}

pub struct Vbo {
    buffer: GLuint,
}

pub struct Ibo {
    buffer: GLuint,
}

pub struct Ssbo {
    buffer: GLuint,
}

/// # Render
///
/// Trait that is implemented by "renderers". A renderer is an extremely
/// thin abstraction, providing no safety guarantees, whose sole job is
/// to perform the necessary state transitions and draw calls to move data
/// from the CPU to GPU, and render something on the screen.
pub trait Render {
    unsafe fn invoke(&self, state: &State);
}

pub struct TextRenderer {
    shader: Shader,
    vao: Vao<3>,
    model_matrix_u: UniformMat4,
    proj_matrix_u: UniformMat4,
    n_quads: u32,
}

impl Render for TextRenderer {
    unsafe fn invoke(&self, state: &State) {
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

        gl::DrawElements(
            gl::TRIANGLES,
            6 * self.n_quads as GLint,
            gl::UNSIGNED_INT,
            ptr::null(),
        );
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

impl TextRenderer {
    pub unsafe fn new() -> Self {
        let shader = Shader::text_shader();
        let model_matrix_u = shader.uniform_mat4("model");
        let proj_matrix_u = shader.uniform_mat4("proj");

        let bef = std::time::Instant::now();
        let atlas = Atlas::new(&plox::font::LM_MATH);
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
        let input = include_str!("../../report.md");
        let bef = std::time::Instant::now();
        let text = plox::shaping::shape(input, &plox::font::LM_MATH);
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

//
// Buffer implementations.
//
// Again, these are extremely thin wrappers.

impl<const N: u32> Vao<N> {
    #[inline(always)]
    unsafe fn gen() -> Self {
        let mut array = 0;
        gl::GenVertexArrays(1, &mut array);
        Vao { array }
    }

    #[inline(always)]
    unsafe fn enable_attrib_arrays(&self) {
        self.bind();
        for i in 0..N {
            gl::EnableVertexAttribArray(i);
        }
    }

    #[inline(always)]
    unsafe fn attrib_ptr(&self, index: GLuint, size: GLsizei, ty: GLenum) {
        self.bind();
        let stride = 0; // Tightly packed atributes.
        let pointer = ptr::null(); // No offset in the buffer.
        let normalized = gl::FALSE;
        gl::VertexAttribPointer(index, size, ty, normalized, stride, pointer);
    }

    #[inline(always)]
    unsafe fn attrib_iptr(&self, index: GLuint, size: GLsizei, ty: GLenum) {
        self.bind();
        let stride = 0; // Tightly packed atributes.
        let pointer = ptr::null(); // No offset in the buffer.
        gl::VertexAttribIPointer(index, size, ty, stride, pointer);
    }

    #[inline(always)]
    unsafe fn bind(&self) {
        gl::BindVertexArray(self.array);
    }
}

impl Vbo {
    #[inline(always)]
    unsafe fn gen() -> Vbo {
        let mut buffer = 0;
        gl::GenBuffers(1, &mut buffer);
        Vbo { buffer }
    }

    #[inline(always)]
    unsafe fn bind(&self) {
        gl::BindBuffer(gl::ARRAY_BUFFER, self.buffer);
    }

    #[inline(always)]
    unsafe fn data<T>(&self, vertices: &[T]) {
        self.bind();
        gl::BufferData(
            gl::ARRAY_BUFFER,
            gl_buf_size(vertices),
            gl_ptr(vertices),
            gl::STATIC_DRAW,
        );
    }
}

impl Ibo {
    #[inline(always)]
    unsafe fn gen() -> Ibo {
        let mut buffer = 0;
        gl::GenBuffers(1, &mut buffer);
        Ibo { buffer }
    }

    #[inline(always)]
    unsafe fn bind(&self) {
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.buffer);
    }

    #[inline(always)]
    unsafe fn data(&self, indices: &[u32]) {
        self.bind();
        gl::BufferData(
            gl::ELEMENT_ARRAY_BUFFER,
            gl_buf_size(indices),
            gl_ptr(indices),
            gl::STATIC_DRAW,
        );
    }
}

impl Ssbo {
    #[inline(always)]
    unsafe fn gen() -> Self {
        let mut buffer = 0;
        gl::GenBuffers(1, &mut buffer);
        Ssbo { buffer }
    }

    #[inline(always)]
    unsafe fn bind(&self) {
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, self.buffer);
    }

    #[inline(always)]
    unsafe fn data<T>(&self, ssbo: &[T]) {
        self.bind();
        gl::BufferData(
            gl::SHADER_STORAGE_BUFFER,
            gl_buf_size(ssbo),
            gl_ptr(ssbo),
            gl::STATIC_DRAW,
        );
    }

    #[inline(always)]
    unsafe fn bind_base(&self, index: u32) {
        gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, index, self.buffer);
    }
}

// Get # of bytes in an array.
#[inline(always)]
fn gl_buf_size<T>(val: &[T]) -> GLsizeiptr {
    std::mem::size_of_val(&val[..]) as _
}

// Get the OpenGL-compatible pointer to an arbitrary array of numbers
#[inline(always)]
fn gl_ptr<T>(val: &[T]) -> *const GLvoid {
    val.as_ptr() as _
}
