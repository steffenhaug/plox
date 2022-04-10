//! Shader abstraction.
use gl::{types::*, FRAGMENT_SHADER, VERTEX_SHADER};
use std::ffi::CString;
use std::ptr;
use std::sync::Arc;

/// A shader is just a wrapper around its program ID.
#[derive(Clone)]
pub struct Shader {
    pub shader: GLuint,
    on_bind: Option<Arc<dyn Fn()>>
}

//
// Uniform types.
//

pub trait Uniform {
    fn wrap(uniform: GLint) -> Self;
}

macro_rules! uniform {
    ( $x:ident ) => {
        #[derive(Debug, Clone, Copy)]
        pub struct $x(pub GLint);

        impl Uniform for $x {
            #[inline(always)]
            fn wrap(uniform: GLint) -> Self {
                $x(uniform)
            }
        }
    };
}

uniform!(UniformMat4);
uniform!(UniformVec2);
uniform!(UniformVec2i);
uniform!(UniformVec4);
uniform!(UniformFloat);

// Shaders programs:
// I just include them in the binary, so the binary is portable.
const TXT_FILL_FRAG: &str = include_str!("fill.frag.glsl");
const TXT_FILL_VERT: &str = include_str!("fill.vert.glsl");
const TXT_OUTLINE_FRAG: &str = include_str!("outline.frag.glsl");
const TXT_OUTLINE_VERT: &str = include_str!("outline.vert.glsl");

const TXT_BLIT_VERT: &str = include_str!("textelement.vert.glsl");
const TXT_BLIT_FRAG: &str = include_str!("textelement_simple.frag.glsl");
const TXT_BLIT_FRAG_FANCY: &str = include_str!("textelement_fancy.frag.glsl");

const CIRCLE_VERT: &str = include_str!("circle.vert.glsl");
const CIRCLE_FRAG: &str = include_str!("circle.frag.glsl");

impl Shader {
    pub unsafe fn fill() -> Shader {
        let vert = Shader::compile(VERTEX_SHADER, TXT_FILL_VERT);
        let frag = Shader::compile(FRAGMENT_SHADER, TXT_FILL_FRAG);
        let program = gl::CreateProgram();
        gl::AttachShader(program, vert);
        gl::AttachShader(program, frag);
        Shader::link(program);
        gl::DeleteShader(vert);
        gl::DeleteShader(frag);
        Shader { shader: program, on_bind: None }
    }

    pub unsafe fn outline() -> Shader {
        let vert = Shader::compile(VERTEX_SHADER, TXT_OUTLINE_VERT);
        let frag = Shader::compile(FRAGMENT_SHADER, TXT_OUTLINE_FRAG);
        let program = gl::CreateProgram();
        gl::AttachShader(program, vert);
        gl::AttachShader(program, frag);
        Shader::link(program);
        gl::DeleteShader(vert);
        gl::DeleteShader(frag);
        Shader { shader: program, on_bind: None }
    }

    pub unsafe fn simple_blit() -> Shader {
        let vert = Shader::compile(VERTEX_SHADER, TXT_BLIT_VERT);
        let frag = Shader::compile(FRAGMENT_SHADER, TXT_BLIT_FRAG);
        let program = gl::CreateProgram();
        gl::AttachShader(program, vert);
        gl::AttachShader(program, frag);
        Shader::link(program);
        gl::DeleteShader(vert);
        gl::DeleteShader(frag);
        Shader { shader: program, on_bind: None }
    }

    pub unsafe fn fancy_blit() -> Shader {
        let vert = Shader::compile(VERTEX_SHADER, TXT_BLIT_VERT);
        let frag = Shader::compile(FRAGMENT_SHADER, TXT_BLIT_FRAG_FANCY);
        let program = gl::CreateProgram();
        gl::AttachShader(program, vert);
        gl::AttachShader(program, frag);
        Shader::link(program);
        gl::DeleteShader(vert);
        gl::DeleteShader(frag);
        Shader { shader: program, on_bind: None }
    }

    pub unsafe fn circle() -> Shader {
        let vert = Shader::compile(VERTEX_SHADER, CIRCLE_VERT);
        let frag = Shader::compile(FRAGMENT_SHADER, CIRCLE_FRAG);
        let program = gl::CreateProgram();
        gl::AttachShader(program, vert);
        gl::AttachShader(program, frag);
        Shader::link(program);
        gl::DeleteShader(vert);
        gl::DeleteShader(frag);
        Shader { shader: program, on_bind: None }
    }

    pub fn on_bind(&mut self, callback: impl 'static + Fn()) {
        self.on_bind = Some(Arc::new(callback));
    }

    pub unsafe fn bind(&self) {
        gl::UseProgram(self.shader);

        if let Some(closure) = &self.on_bind {
            closure();
        }
    }

    pub unsafe fn uniform<U: Uniform>(&self, name: &str) -> U {
        if let Some(loc) = self.uniform_location(name) {
            return U::wrap(loc);
        }

        panic!("invalid uniform {}", name)
    }

    unsafe fn uniform_location(&self, name: &str) -> Option<GLint> {
        self.bind();
        let name = CString::new(name).ok()?;
        let loc = gl::GetUniformLocation(self.shader, name.as_ptr());
        if loc != -1 {
            Some(loc)
        } else {
            None
        }
    }

    /// Compile a shader with error check.
    unsafe fn compile(ty: GLenum, src: &str) -> GLuint {
        let sh = gl::CreateShader(ty);

        // Ok to unwrap because its `include_str!`ed so can't have null bytes.
        // This forces allocation of the strings, but it only happens once,
        // so that's okay.
        let src = CString::new(src).unwrap();
        gl::ShaderSource(sh, 1, &src.as_ptr(), ptr::null());
        gl::CompileShader(sh);

        // Check for compile errors.
        let mut ok = gl::FALSE as GLint;
        gl::GetShaderiv(sh, gl::COMPILE_STATUS, &mut ok);

        if ok != gl::TRUE as GLint {
            let mut info_log = Vec::with_capacity(512);
            let mut len = 0;
            info_log.set_len(512 - 1);
            gl::GetShaderInfoLog(sh, 512, &mut len, info_log.as_mut_ptr() as *mut GLchar);

            // Inform the user what went wrong.
            println!(
                "ERROR (SHADER COMPILE): {}",
                String::from_utf8_lossy(&info_log[..len as usize - 1])
            );

            // Exit - there is zero point in trying any form of recovery,
            // the user simply needs to fix the shader.
            panic!("shader compile");
        }

        sh
    }

    /// Links a shader program.
    unsafe fn link(prog: GLuint) {
        // Do the linking.
        gl::LinkProgram(prog);

        // Check for linker errors.
        let mut ok = gl::FALSE as GLint;
        gl::GetProgramiv(prog, gl::LINK_STATUS, &mut ok);

        if ok != gl::TRUE as GLint {
            let mut info_log = Vec::with_capacity(512);
            let mut len = 0;
            info_log.set_len(512 - 1);
            gl::GetProgramInfoLog(prog, 512, &mut len, info_log.as_mut_ptr() as *mut GLchar);

            // Inform the user what went wrong.
            println!(
                "ERROR (SHADER LINK): {}",
                String::from_utf8_lossy(&info_log[..len as usize - 1])
            );

            // Exit - there is zero point in trying any form of recovery,
            // the user simply needs to fix the shader.
            panic!("shader link");
        }

        gl::ValidateProgram(prog);
    }
}

impl UniformMat4 {
    #[inline(always)]
    pub unsafe fn data(&self, mat: &glm::Mat4) {
        gl::UniformMatrix4fv(self.0, 1, 0, mat.as_ptr());
    }
}

impl UniformVec2 {
    #[inline(always)]
    pub unsafe fn data(&self, x: f32, y: f32) {
        gl::Uniform2f(self.0, x, y);
    }
}

impl UniformVec4 {
    #[inline(always)]
    pub unsafe fn data(&self, x: f32, y: f32, z: f32, w: f32) {
        gl::Uniform4f(self.0, x, y, z, w);
    }
}

impl UniformVec2i {
    #[inline(always)]
    pub unsafe fn data(&self, x: i32, y: i32) {
        gl::Uniform2i(self.0, x, y);
    }
}

impl UniformFloat {
    #[inline(always)]
    pub unsafe fn data(&self, x: f32) {
        gl::Uniform1f(self.0, x);
    }
}
