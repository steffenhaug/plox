//! Defines interop with GPU, i. e. buffer types,
//! vertex types, and so on.
use crate::shader::{Shader, Uniform};
use plox::spline::{Spline, Quadratic};
use gl::types::*;
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
    unsafe fn invoke(&self);
}

pub struct TextRenderer {
    shader: Shader,
    vao: Vao<2>,
    quads: u32,
}

impl Render for TextRenderer {
    unsafe fn invoke(&self) {
        self.vao.bind();
        self.shader.bind();

        // Text is always rendered on quads; a quad has 6 vertices.
        let n_elements = 6 * self.quads;

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

        let v: [(f32, f32); 4] = [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)];
        let uv: [(f32, f32); 4] = [(0.0, 0.0), (1000.0, 0.0), (1000.0, 1000.0), (0.0, 1000.0)];
        let i: [u32; 6] = [
            0, 1, 2, /* first triangle */
            0, 2, 3, /* second triangle */
        ];

        // width of screen: 800
        // uv in range 0-1000 covering 400 pixels
        // => 1000 / 400 uvs per pixel

        let vao = Vao::<2>::gen();
        vao.enable_attrib_arrays();

        let verts = Vbo::gen();
        verts.data(&v);
        vao.attrib_ptr(0, 2, gl::FLOAT);

        let uvs = Vbo::gen();
        uvs.data(&uv);
        vao.attrib_ptr(1, 2, gl::FLOAT);

        let ibo = Ibo::gen();
        ibo.data(&i);

        // Scale and translage the spline.
        // This REALLY needs some work lmao
        let text = plox::shaping::shape("\u{2207}\u{03B1}\u{2254}\u{03D1} Anti-aliasing!").scale(0.03);
        let text = text.strokes().map(Spline::translate(0.0, 50.0)).collect::<Vec<Quadratic>>();

        let ssbo = Ssbo::gen();
        ssbo.data(&text);
        ssbo.bind_base(0);

        TextRenderer {
            shader,
            vao,
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
fn gl_ptr<T>(val: &[T]) -> *const GLvoid {
   val.as_ptr() as _
}
