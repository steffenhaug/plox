extern crate nalgebra_glm as glm;
mod util;

use plox::atlas::Atlas;
use plox::font;
use plox::gpu::{
    circle::{CircleElement, CircleRenderer},
    shader::Shader,
    text::{TextElement, TextRenderer, TextShader},
    typeset::Typeset,
    Transform,
};
use plox::line::{LineElement, LineRenderer, Segment};

use glutin::event::{ElementState::*, Event, KeyboardInput, VirtualKeyCode::*, WindowEvent};
use glutin::event_loop::ControlFlow;

use std::f32::consts::PI;
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
    fps: Mutable<TextElement>,
    // Text renderer.
    text_renderer: TextRenderer,
    default_text_shader: TextShader,
    // Circle renderer.
    circle_renderer: CircleRenderer,
    // Line segment renderer.
    line_renderer: LineRenderer,
    // All the renderable objects.
    ecs: Ecs,
}

/// Behold: The worlds most shit ECS! (Confirmed world record)

struct Thing {
    typeset_text_component: Option<Typeset>,
    text_shader_component: Option<TextShader>,
    transform_component: Option<Transform>,
    animation_component: Option<Arc<dyn Fn() -> Transform>>,
    circle_component: Option<CircleElement>,
    line_component: Option<LineElement>,
}

struct Ecs {
    content: Vec<Thing>,
}

impl Thing {
    /// Create a thing with no components.
    fn new() -> Self {
        Thing {
            typeset_text_component: None,
            text_shader_component: None,
            transform_component: None,
            animation_component: None,
            circle_component: None,
            line_component: None,
        }
    }

    /// Effectively deletes without forcing O(n) reallocation of the ECS.
    #[allow(dead_code)]
    fn nuke(&mut self) {
        *self = Thing::new();
    }

    // Builder pattern for adding components.

    fn typeset_text(mut self, text: Typeset) -> Self {
        self.typeset_text_component = Some(text);
        self
    }

    fn text_shader(mut self, shader: TextShader) -> Self {
        self.text_shader_component = Some(shader);
        self
    }

    fn transform(mut self, transform: Transform) -> Self {
        self.transform_component = Some(transform);
        self
    }

    fn animation(mut self, anim: impl 'static + Fn() -> Transform) -> Self {
        self.animation_component = Some(Arc::new(anim));
        self
    }

    fn circle(mut self, circle: CircleElement) -> Self {
        self.circle_component = Some(circle);
        self
    }

    fn line(mut self, line: LineElement) -> Self {
        self.line_component = Some(line);
        self
    }

    // Accessors for filter_map based systems.

    fn typeset_text_component(
        &self,
    ) -> Option<(&Typeset, Option<&Transform>, Option<&TextShader>)> {
        self.typeset_text_component.as_ref().map(|typeset| {
            (
                typeset,
                self.transform_component.as_ref(),
                self.text_shader_component.as_ref(),
            )
        })
    }

    fn circle_component(&self) -> Option<(&CircleElement, Option<&Transform>)> {
        self.circle_component
            .as_ref()
            .map(|circ| (circ, self.transform_component.as_ref()))
    }

    fn line_component(&self) -> Option<(&LineElement, Option<&Transform>)> {
        self.line_component
            .as_ref()
            .map(|line| (line, self.transform_component.as_ref()))
    }

    fn animation_component(
        &mut self,
    ) -> Option<(&Arc<dyn Fn() -> Transform>, &mut Option<Transform>)> {
        self.animation_component
            .as_ref()
            .map(|anim| (anim, &mut self.transform_component))
    }
}

/// Animation system
fn animate(state: &mut State) {
    for (anim, maybe_transform) in state
        .ecs
        .content
        .iter_mut()
        .filter_map(Thing::animation_component)
    {
        *maybe_transform = Some(anim());
    }
}

/// Rendering "system" :v)
unsafe fn render(state: &State) {
    gl::ClearColor(1.0, 1.0, 1.0, 1.0);
    gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

    // A default transform to substitute if a renderable lacks a transform.
    let id = Transform::identity();

    for thing in &state.ecs.content {
        if let Some((renderable, maybe_transform, maybe_shader)) = thing.typeset_text_component() {
            // Substitute default text shader in the absence of a specific one.
            let text_shader = maybe_shader.unwrap_or(&state.default_text_shader);
            let text_transform = maybe_transform.unwrap_or(&id);
            renderable.traverse_scenegraph(&state.text_renderer, text_transform, text_shader);
        }

        if let Some((circle, maybe_transform)) = thing.circle_component() {
            let circle_transform = maybe_transform.unwrap_or(&id);
            circle.rasterize(&state.circle_renderer, circle_transform);
        }

        if let Some((line, maybe_transform)) = thing.line_component() {
            let line_transform = maybe_transform.unwrap_or(&id);
            line.rasterize(
                &state.line_renderer,
                line_transform,
                &state.line_renderer.line_shader,
            );
        }
    }
}

impl<'a> State<'a> {
    /// Create a new state.
    /// This involves, among other things, compiling and linking shaders,
    /// which means you need a valid GL context, which makes this unsafe.
    unsafe fn new() -> State<'a> {
        let text_renderer = TextRenderer::new();
        let circle_renderer = CircleRenderer::new();
        let line_renderer = LineRenderer::new();

        let atlas = Atlas::new(&font::LM_MATH);
        let default_text_shader = Shader::simple_blit();
        let colored_text = Shader::fancy_blit();

        // Shared mouse position
        let mouse = Arc::new(RwLock::new((0.0, 0.0)));

        // Typeset a test integral
        let lim1 = Typeset::text("Ω", &atlas);
        let sum = Typeset::integral(Some(lim1), None, &atlas);
        let body = Typeset::text("\u{1D453}(\u{1D465})d\u{1D707}(\u{1D465})", &atlas);

        // Give it translation defined by mouse position.
        let m = mouse.clone();

        let int = Typeset::seq(vec![sum, body]);

        let mut content = Vec::with_capacity(32);

        content.push(
            Thing::new()
                .typeset_text(int)
                .text_shader(colored_text.into())
                .transform(Transform {
                    scale: 120.0,
                    translation: (100.0, 200.0),
                }),
        );

        let fps = Arc::new(RwLock::new(TextElement::new(" ", &atlas)));

        content.push(
            Thing::new()
                .typeset_text(Typeset::elem(fps.clone()))
                .transform(Transform {
                    scale: 40.0,
                    translation: (10.0, 10.0),
                }),
        );

        content.push(
            Thing::new()
                .circle(CircleElement::new(200.0).width(3.0).arc(0.3, 5.0))
                .transform(Transform {
                    scale: 1.0,
                    translation: (400.0, 400.0),
                })
                .animation(move || Transform {
                    scale: f32::max(50.0, 2.0 * m.read().unwrap().1 - 400.0),
                    translation: *m.read().unwrap(),
                }),
        );

        let n = 250;
        let graph: Vec<glm::Vec2> = (0..n)
            .map(|i| {
                let t = 2.0 * PI * (i as f32 / n as f32);
                let x = 100.0 * t;
                let y = 50.0 * f32::sin(t);
                glm::vec2(x, y)
            })
            .collect();

        let spline = Segment::spline(&graph);

        let line = LineElement::new(spline.segments(), 150.0);
        content.push(Thing::new().line(line).transform(Transform {
            scale: 1.0,
            translation: (200.0, 600.0),
        }));

        State {
            win_dims: (SCREEN_W, SCREEN_H),
            mouse,
            atlas,
            fps,
            text_renderer,
            default_text_shader: default_text_shader.into(),
            circle_renderer,
            line_renderer,
            ecs: Ecs { content },
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
                // Update animations at the start of the frame.
                animate(&mut state);
                unsafe {
                    let beg = std::time::Instant::now();
                    render(&state);
                    let end = std::time::Instant::now();
                    state.fps.write().unwrap().update(
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
