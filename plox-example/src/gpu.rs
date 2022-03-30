//! Defines interop with GPU, i. e. buffer types,
//! vertex types, and so on.
use crate::shader::{Shader, UniformMat4};
use crate::State;
use gl::types::*;
use plox::spline::Quadratic;
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
    model_matrix_u: UniformMat4,
    proj_matrix_u: UniformMat4,
    vao: Vao<2>,
    quads: u32,
}

impl Render for TextRenderer {
    unsafe fn invoke(&self, state: &State) {
        self.vao.bind();
        self.shader.bind();

        let m: glm::Mat4 = glm::translation(&glm::vec3(390.0, 390.0, 0.0))
            * glm::scaling(&glm::vec3(20.0, 20.0, 0.0));

        // Compute projection matrix.
        let (w, h) = state.win_dims;
        let p: glm::Mat4 = glm::ortho(0.0, w as f32, 0.0, h as f32, 0.0, 1000.0);

        // Todo: abstract this (send matrices to the shader program)
        gl::UniformMatrix4fv(self.model_matrix_u.0, 1, 0, m.as_ptr());
        gl::UniformMatrix4fv(self.proj_matrix_u.0, 1, 0, p.as_ptr());

        // Text is always rendered on quads; a quad has 6 vertices.
        let n_elements = 6 * self.quads;

        println!("draw call");

        gl::DrawElements(
            gl::TRIANGLES,
            n_elements as GLsizei,
            gl::UNSIGNED_INT,
            ptr::null(),
        );
    }
}

impl TextRenderer {
    pub unsafe fn new() -> Self {
        let shader = Shader::text_shader();
        let model_matrix_u = shader.uniform_mat4("model");
        let proj_matrix_u = shader.uniform_mat4("proj");

        // Scale and translage the spline.
        // This REALLY needs some work lmao
        let input = "\u{2207}\u{03B1} = \u{222B}\u{1D453}\u{2009}d\u{03BC}";
        let text = plox::shaping::shape(input, &plox::font::LM_MATH);

        // 800 tall window
        //

        let bbox = text.bbox();

        dbg!(bbox);

        let v: [(f32, f32); 4] = [
            (bbox.x0, bbox.y0),
            (bbox.x1, bbox.y0),
            (bbox.x1, bbox.y1),
            (bbox.x0, bbox.y1),
        ];

        let data = text.strokes().cloned().collect::<Vec<Quadratic>>();

        let i: [u32; 6] = [
            0, 1, 2, /* first triangle */
            0, 2, 3, /* second triangle */
        ];

        let vao = Vao::<2>::gen();
        vao.enable_attrib_arrays();

        let verts = Vbo::gen();
        verts.data(&v);
        vao.attrib_ptr(0, 2, gl::FLOAT);

        let uvs = Vbo::gen();
        uvs.data(&v);
        vao.attrib_ptr(1, 2, gl::FLOAT);

        let ibo = Ibo::gen();
        ibo.data(&i);

        let ssbo = Ssbo::gen();
        ssbo.data(&data[..]);
        ssbo.bind_base(0);

        TextRenderer {
            shader,
            vao,
            model_matrix_u,
            proj_matrix_u,
            quads: 1,
        }
    }
}

//
// Buffer implementations.
//
// Again, these are extremely thin wrappers.

impl<const N: u32> Vao<N> {
    unsafe fn gen() -> Self {
        let mut array = 0;
        gl::GenVertexArrays(1, &mut array);
        Vao { array }
    }

    unsafe fn enable_attrib_arrays(&self) {
        self.bind();
        for i in 0..N {
            gl::EnableVertexAttribArray(i);
        }
    }

    unsafe fn attrib_ptr(&self, index: GLuint, size: GLsizei, ty: GLenum) {
        self.bind();
        let stride = 0; // Tightly packed atributes.
        let pointer = ptr::null(); // No offset in the buffer.
        let normalized = gl::FALSE;
        gl::VertexAttribPointer(index, size, ty, normalized, stride, pointer);
    }

    unsafe fn bind(&self) {
        gl::BindVertexArray(self.array);
    }
}

impl Vbo {
    unsafe fn gen() -> Vbo {
        let mut buffer = 0;
        gl::GenBuffers(1, &mut buffer);
        Vbo { buffer }
    }

    unsafe fn bind(&self) {
        gl::BindBuffer(gl::ARRAY_BUFFER, self.buffer);
    }

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
    unsafe fn gen() -> Ibo {
        let mut buffer = 0;
        gl::GenBuffers(1, &mut buffer);
        Ibo { buffer }
    }

    unsafe fn bind(&self) {
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.buffer);
    }

    unsafe fn data<T>(&self, indices: &[T]) {
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
    unsafe fn gen() -> Self {
        let mut buffer = 0;
        gl::GenBuffers(1, &mut buffer);
        Ssbo { buffer }
    }

    unsafe fn bind(&self) {
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, self.buffer);
    }

    unsafe fn data<T>(&self, ssbo: &[T]) {
        self.bind();
        gl::BufferData(
            gl::SHADER_STORAGE_BUFFER,
            gl_buf_size(ssbo),
            gl_ptr(ssbo),
            gl::STATIC_DRAW,
        );
    }

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
