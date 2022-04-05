//! GPU Module
//!
//! This contains everything that is used for feeding the GPU data.
//!
//! The philosophy here is to create the *thinnest* concievable wrapper around raw OpenGL calls.
//! Rationale: Writing a "perfect" safe abstraction is more work than writing any graphical
//! application itself is likely to be. Existing abstractions are incomplete and leaky, so the best
//! course of action is to take on the responsibility of managing the OpenGL state manually, and
//! make the management as transparent as possible, so it is always clear *exactly* what is going
//! on at all times. This means liberal use of `unsafe` unfortunately, but is is limited to the GPU
//! subsystem.
//!
//! This does not provide an OpenGL context. The client should use something like glutin to create
//! a context before utilizing functionality from this module.
//!
//! The abstraction provided is limited: All it provides is newtypes for IDs and methods that
//! supply some default parameters to avoid misuse.
pub mod shader;
pub mod text; // text rendering
pub mod typeset;

use gl::types::*;
use std::ptr;

/// # Render
///
/// A renderer is a monolithic bundle of all the data that is necessary to perform the rendering of
/// some part of the application. The monolithic architecture gives tight control over *exactly*
/// which `gl` calls are made, so the performance can be predictale. This can be as complex as it
/// needs to, for example it could contain a scene graph and so on.
pub trait Render {
    type State;
    unsafe fn invoke(&self, state: &Self::State);
}

//
// Video memory management.
//

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

pub struct MultisampleTexture {
    pub tex: GLuint,
}

//
// Utility functions.
//

// Get # of bytes in an array.
#[inline(always)]
pub fn gl_buf_size<T>(val: &[T]) -> GLsizeiptr {
    std::mem::size_of_val(&val[..]) as GLsizeiptr
}

// Get the OpenGL-compatible pointer to an arbitrary array of numbers
#[inline(always)]
pub fn gl_ptr<T>(val: &[T]) -> *const GLvoid {
    val.as_ptr() as *const GLvoid
}

//
// Implementations
//

impl<const N: u32> Vao<N> {
    #[inline(always)]
    pub unsafe fn gen() -> Self {
        let mut array = 0;
        gl::GenVertexArrays(1, &mut array);
        Vao { array }
    }

    #[inline(always)]
    pub unsafe fn enable_attrib_arrays(&self) {
        gl::BindVertexArray(self.array);
        for i in 0..N {
            gl::EnableVertexAttribArray(i);
        }
    }

    #[inline(always)]
    pub unsafe fn attrib_ptr(&self, index: GLuint, size: GLsizei, ty: GLenum) {
        gl::BindVertexArray(self.array);
        let stride = 0; // Tightly packed atributes.
        let pointer = ptr::null(); // No offset in the buffer.
        let normalized = gl::FALSE;
        gl::VertexAttribPointer(index, size, ty, normalized, stride, pointer);
    }

    #[inline(always)]
    pub unsafe fn attrib_iptr(&self, index: GLuint, size: GLsizei, ty: GLenum) {
        gl::BindVertexArray(self.array);
        let stride = 0; // Tightly packed atributes.
        let pointer = ptr::null(); // No offset in the buffer.
        gl::VertexAttribIPointer(index, size, ty, stride, pointer);
    }

    #[inline(always)]
    pub unsafe fn bind(&self) {
        gl::BindVertexArray(self.array);
    }
}

impl Vbo {
    #[inline(always)]
    pub unsafe fn gen() -> Vbo {
        let mut buffer = 0;
        gl::GenBuffers(1, &mut buffer);
        Vbo { buffer }
    }

    #[inline(always)]
    pub unsafe fn bind(&self) {
        gl::BindBuffer(gl::ARRAY_BUFFER, self.buffer);
    }

    #[inline(always)]
    pub unsafe fn data<T>(&self, vertices: &[T]) {
        gl::BindBuffer(gl::ARRAY_BUFFER, self.buffer);
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
    pub unsafe fn gen() -> Ibo {
        let mut buffer = 0;
        gl::GenBuffers(1, &mut buffer);
        Ibo { buffer }
    }

    #[inline(always)]
    pub unsafe fn bind(&self) {
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.buffer);
    }

    #[inline(always)]
    pub unsafe fn data(&self, indices: &[u32]) {
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.buffer);
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
    pub unsafe fn gen() -> Self {
        let mut buffer = 0;
        gl::GenBuffers(1, &mut buffer);
        Ssbo { buffer }
    }

    #[inline(always)]
    pub unsafe fn bind(&self) {
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, self.buffer);
    }

    #[inline(always)]
    pub unsafe fn data<T>(&self, ssbo: &[T]) {
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, self.buffer);
        gl::BufferData(
            gl::SHADER_STORAGE_BUFFER,
            gl_buf_size(ssbo),
            gl_ptr(ssbo),
            gl::STATIC_DRAW,
        );
    }

    #[inline(always)]
    pub unsafe fn bind_base(&self, index: u32) {
        gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, index, self.buffer);
    }
}

impl MultisampleTexture {
    #[inline(always)]
    pub unsafe fn bind(&self) {
        gl::BindTexture(gl::TEXTURE_2D_MULTISAMPLE, self.tex);
    }

    /// Create a single-channel multisampled texture. Used as alpha-channel
    /// for anti-aliased text.
    #[inline(always)]
    pub unsafe fn alpha(width: u32, height: u32) -> MultisampleTexture {
        let mut tex = 0;
        gl::GenTextures(1, &mut tex);
        gl::BindTexture(gl::TEXTURE_2D_MULTISAMPLE, tex);

        gl::TexImage2DMultisample(
            gl::TEXTURE_2D_MULTISAMPLE,
            // 16x hard-coded for now. Could tweak this at type level.
            16,
            // A single color channel.
            gl::R8,
            // Negative dims is actually error, so casting a u32
            // is better than exposing the real type.
            width as i32,
            height as i32,
            // Use fixed sample locations.
            gl::TRUE,
        );

        gl::BindTexture(gl::TEXTURE_2D_MULTISAMPLE, 0);
        MultisampleTexture { tex }
    }
}
