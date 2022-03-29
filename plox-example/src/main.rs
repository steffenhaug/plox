mod gpu;
mod shader;
mod util;

use crate::gpu::{Render, TextRenderer};

use glutin::event::{Event, KeyboardInput, VirtualKeyCode::*, WindowEvent};
use glutin::event_loop::ControlFlow;
use glutin::{Api::OpenGl, GlRequest::Specific};

use std::ptr;

const SCREEN_W: u32 = 1000;
const SCREEN_H: u32 = 1000;

/// Contains everything that is used to feed data to the GPU.
struct State {
    text_renderer: TextRenderer,
}

/// Performs drawing operations.
/// Unsafe because the responsibility of not performing undefined
/// OpenGL behavious lies on the caller. So the state needs to not
/// have invalid handles, the right GL context needs to be active,
/// and so on.
unsafe fn render(state: &State) {
    println!("[INFO] Redraw requested.");
    gl::ClearColor(0.0, 0.0, 0.0, 1.0);
    gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
    state.text_renderer.invoke();
}

impl State {
    /// Create a new state.
    /// This involves, among other things, compiling and linking shaders,
    /// which means you need a valid GL context, which makes this unsafe.
    unsafe fn new() -> State {
        let text_renderer = gpu::TextRenderer::new();
        State { text_renderer }
    }
}

fn main() {
    //
    // Glutin boilerplate.
    //
    let el = glutin::event_loop::EventLoop::new();

    let wb = glutin::window::WindowBuilder::new()
        .with_title("JalLaTeX")
        .with_resizable(false)
        .with_inner_size(glutin::dpi::LogicalSize::new(SCREEN_W, SCREEN_H));

    let cb = glutin::ContextBuilder::new()
        // I need this version for SSBO. Without this, it defaulted to ES 3.2.
        // Maybe that's a Nvidia driver / Wayland thing.
        .with_gl(Specific(OpenGl, (4, 3)))
        // I think Waylands compositor handles this. With VSync enabled we don't get a context.
        // Should probably set this flag based on whether we are running under Wayland if
        // that is possible to find out.
        .with_vsync(false);

    let wc = cb.build_windowed(wb, &el).unwrap();

    // Some notable changes from Gloom: I render on the main thread using
    // the `RedrawRequested`-event, instead of using a separate thread.
    // I was having some problems with the rendering thread panicking when
    // I close the window, because the main thread exits without joining.
    // This is possible to fix, but I'd rather deal with it if it becomes
    // a problem, instead of prematurely.

    // Load OpenGL function pointers + make our window the current context.
    let ctx = unsafe {
        let c = wc.make_current().unwrap();
        gl::load_with(|sym| c.get_proc_address(sym) as *const _);
        c
    };

    //
    // OpenGL Initializerion.
    //
    unsafe {
        /*
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LESS);
        gl::Enable(gl::CULL_FACE);
        gl::Disable(gl::MULTISAMPLE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        */
        gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
        gl::DebugMessageCallback(Some(util::debug_callback), ptr::null());

        // Print some diagnostics
        let vendor = util::get_gl_string(gl::VENDOR);
        let renderer = util::get_gl_string(gl::RENDERER);
        println!("{}: {}", vendor, renderer);
        println!("OpenGL\t: {}", util::get_gl_string(gl::VERSION));
        println!(
            "GLSL\t: {}",
            util::get_gl_string(gl::SHADING_LANGUAGE_VERSION)
        );
    }

    //
    // Program state.
    //
    let state = unsafe { State::new() };

    //
    // Event loop.
    //
    el.run(move |event, _, ctrl| {
        *ctrl = ControlFlow::Wait;

        match event {
            Event::RedrawRequested(_) => {
                unsafe {
                    render(&state);
                }
                ctx.swap_buffers().unwrap();
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *ctrl = ControlFlow::Exit;
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(key),
                            state,
                            ..
                        },
                    ..
                } => match (key, state) {
                    //
                    // Keyboard innput handling.
                    //
                    (Escape, _) => *ctrl = ControlFlow::Exit,
                    (_, _) => (),
                },
                _ => (),
            },
            _ => (),
        }
    });
}
