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
pub mod circle; // circle, circle arc rendering
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

pub struct Texture {
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

#[inline(always)]
pub fn gl_mut_ptr<T>(val: &mut [T]) -> *mut GLvoid {
    val.as_ptr() as *mut GLvoid
}

pub unsafe fn gl_viewport() -> (GLint, GLint, GLint, GLint) {
    let mut vp = [0; 4];
    gl::GetIntegerv(gl::VIEWPORT, gl_mut_ptr(&mut vp) as *mut i32);
    (vp[0], vp[1], vp[2], vp[3])
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

impl Texture {
    #[inline(always)]
    pub unsafe fn bind(&self) {
        gl::BindTexture(gl::TEXTURE_2D, self.tex);
    }

    /// Create a single-channel multisampled texture. Used as alpha-channel
    /// for anti-aliased text.
    #[inline(always)]
    pub unsafe fn alpha(width: u32, height: u32) -> Texture {
        let mut tex = 0;
        gl::GenTextures(1, &mut tex);
        gl::BindTexture(gl::TEXTURE_2D, tex);

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,             /* Level of detail */
            gl::R8 as i32, /* Internal format (A single color channel) */
            width as i32,
            height as i32,
            0,                 /* Border. (Must be zero per the docs) */
            gl::RED,           /* Format for pixel data. */
            gl::UNSIGNED_BYTE, /* Type of pixel data. */
            std::ptr::null(),
        );

        // Disable mipmapping
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);

        gl::BindTexture(gl::TEXTURE_2D, 0);
        Texture { tex }
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
