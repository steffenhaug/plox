extern crate nalgebra_glm as glm;
mod util;

use plox::atlas::Atlas;
use plox::font;
use plox::gpu::{
    text::{TextElement, TextRenderer, TextRendererState, Transform},
    typeset::{Node, Typeset},
    Render,
};

use glutin::event::{ElementState::*, Event, KeyboardInput, VirtualKeyCode::*, WindowEvent};
use glutin::event_loop::ControlFlow;

use std::ptr;
use std::sync::{Arc, RwLock};

// Initial window size.
pub const SCREEN_W: u32 = 800;
pub const SCREEN_H: u32 = 800;

type Mutable<T> = Arc<RwLock<T>>;

/// Contains everything that is used to feed data to the GPU.
pub struct State<'a> {
    atlas: Atlas<'a>,
    win_dims: (u32, u32),
    mouse: Mutable<(f32, f32)>,
    fps_text: Mutable<TextElement>,
    text_renderer: TextRenderer,
}

/// Performs drawing operations.
unsafe fn render(state: &State) {
    gl::ClearColor(1.0, 1.0, 1.0, 1.0);
    gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
    state.text_renderer.invoke(&TextRendererState {
        win_dims: state.win_dims,
    });
}

impl<'a> State<'a> {
    /// Create a new state.
    /// This involves, among other things, compiling and linking shaders,
    /// which means you need a valid GL context, which makes this unsafe.
    unsafe fn new() -> State<'a> {
        let mut text_renderer = TextRenderer::new();
        let atlas = Atlas::new(&font::LM_MATH);

        // shared mouse position
        let mouse = Arc::new(RwLock::new((0.0, 0.0)));

        // Typeset a test integral
        let lim1 = Typeset::text("Ω", &atlas);
        let sum = Typeset::integral(Some(lim1), None, &atlas);
        let body = Typeset::text("\u{1D453}(\u{1D467})d\u{1D467}", &atlas);

        // Give it translation defined by mouse position.
        let m = mouse.clone();
        let int = Typeset::seq(vec![sum, body]).transform(Box::new(move || Transform {
            scale: f32::max(50.0, 2.0 * m.read().unwrap().1 - 400.0),
            translation: *m.read().unwrap(),
        }));

        // Submit it to the renderer.
        text_renderer.submit(int);

        let fps = TextElement::new(" ", &atlas);
        let bbox = fps.bbox;
        let fps_text = Arc::new(RwLock::new(fps));
        let fps = Typeset {
            content: Node::Text(fps_text.clone()),
            bbox,
            transform: Box::new(|| Transform {
                scale: 25.0,
                translation: (10.0, 10.0),
            }),
        };

        text_renderer.submit(fps);

        State {
            win_dims: (SCREEN_W, SCREEN_H),
            mouse,
            atlas,
            fps_text,
            text_renderer,
        }
    }
}

fn main() {
    //
    // Glutin boilerplate.
    //
    let el = glutin::event_loop::EventLoop::new();

    let wb = glutin::window::WindowBuilder::new()
        .with_title("JalLaTeX")
        .with_resizable(true)
        .with_inner_size(glutin::dpi::PhysicalSize::new(SCREEN_W, SCREEN_H));

    let cb = glutin::ContextBuilder::new()
        // I need this version for SSBO. Without this, it defaulted to ES 3.2.
        // Maybe that's a Nvidia driver / Wayland thing.
        // I think Waylands compositor handles this. With VSync enabled we don't get a context.
        // Should probably set this flag based on whether we are running under Wayland if
        // that is possible to find out.
        .with_vsync(false);

    let wc = cb.build_windowed(wb, &el).unwrap();

    // Some notable changes from Gloom: I render on the main thread using
    // the `RedrawRequested`-event, instead of using a separate thread.
    // This allows redrawing sa lazily as possible, since this is not a game,
    // updates might be rare and we would rather not hog the processing power.

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
        gl::Disable(gl::MULTISAMPLE);
        gl::Enable(gl::BLEND);
        gl::BlendFuncSeparate(
            gl::SRC_ALPHA,
            gl::ONE_MINUS_SRC_ALPHA,
            gl::ONE,
            gl::ONE_MINUS_SRC_ALPHA,
        );
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
    let mut state = unsafe { State::new() };

    //
    // Event loop.
    //
    // We can actually do simuation on a separate thread if we .join it in
    // the handling `LoopDestroyed`. Via an event loop proxy, this means we
    // could concurrently generate requests to udpate the animation state,
    // without doing any ugly timing hacks in the main event loop.
    //
    // Specifically, a separate thread can load a CSV file or something, and
    // start sending animation events. Or it could hook into Lua or Python
    // and get data live!
    el.run(move |event, _, ctrl| {
        *ctrl = ControlFlow::Wait;

        match event {
            // Redraw if requested to.
            // This is done in two scenarios:
            //  1. If the OS has invalidated the windows content, for example
            //     by resizing
            //  2. We explicitly request it.
            Event::RedrawRequested(_) => {
                unsafe {
                    let beg = std::time::Instant::now();
                    render(&state);
                    let end = std::time::Instant::now();
                    state.fps_text.write().unwrap().update(
                        &format!(
                            "Δt = {}ns ({}ms)",
                            (end - beg).as_nanos(),
                            (end - beg).as_millis()
                        ),
                        &state.atlas,
                    );
                }
                ctx.swap_buffers().unwrap();
            }
            //
            // Window events.
            //
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *ctrl = ControlFlow::Exit;
                }
                // Even with non-resizable window, some window managers still allows
                // changing the window size by force, so it is important to do this
                // correctly even if we aren't necessary planning to do it.
                WindowEvent::Resized(dims) => {
                    unsafe {
                        gl::Viewport(0, 0, dims.width as _, dims.height as _);
                    }
                    state.win_dims = (dims.width, dims.height);
                    ctx.resize(dims);
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
                    (R, Pressed) => {
                        println!("R pressed.");
                        // how to manually redraw
                        ctx.window().request_redraw();
                    }
                    (_, _) => (),
                },
                WindowEvent::CursorMoved { position, .. } => {
                    // Translate into normal (x, y) coordinates.
                    let x = (position.x as f32).round();
                    let y = (state.win_dims.1 as f32 - position.y as f32).round();
                    *state.mouse.write().unwrap() = (x, y);
                    ctx.window().request_redraw();
                }
                _ => (),
            },
            _ => (),
        }
    });
}
