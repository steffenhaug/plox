//! Shader loading, compiling and linking.
use gl::{types::*, FRAGMENT_SHADER, VERTEX_SHADER};
use std::ffi::CString;
use std::ptr;

const SHADER_SRC_FRAG: &str = include_str!("frag.glsl");
const SHADER_SRC_VERT: &str = include_str!("vert.glsl");

pub struct Shader {
    pub shader: GLuint,
}

pub struct Uniform {
    uniform: GLuint,
}

impl Shader {
    /// A shader dedicated to rendering text.
    pub unsafe fn text_shader() -> Shader {
        // Compile the individual shaders.
        let vert = Shader::compile(VERTEX_SHADER, SHADER_SRC_VERT);
        let frag = Shader::compile(FRAGMENT_SHADER, SHADER_SRC_FRAG);

        // Link the shaders.
        let program = gl::CreateProgram();
        gl::AttachShader(program, vert);
        gl::AttachShader(program, frag);
        Shader::link(program);

        // If we didn't panic, all is well. This GPU memory can be freed.
        gl::DeleteShader(vert);
        gl::DeleteShader(frag);

        // Return the complete shader program.
        Shader { shader: program }
    }

    pub unsafe fn bind(&self) {
        gl::UseProgram(self.shader);
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
