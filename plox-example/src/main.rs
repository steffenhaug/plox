extern crate nalgebra_glm as glm;
mod util;

use glm::vec2;
use plox::atlas::Atlas;
use plox::font;
use plox::gpu::{
    circle::{CircleElement, CircleRenderer, CircleShader},
    shader::Shader,
    text::{TextElement, TextRenderer, TextShader},
    typeset::Typeset,
    Transform,
};
use plox::line::{LineElement, LineRenderer, LineShader, Segment};
use plox::spline::Cubic;

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
    translation_component: Option<glm::Vec2>,
    scale_component: Option<f32>,
    animation_component: Option<Arc<dyn Fn(&mut Thing, &Ecs)>>,
    circle_component: Option<CircleElement>,
    circle_shader_component: Option<CircleShader>,
    bezier_component: Option<Cubic>,
    line_component: Option<LineElement>,
    line_shader_component: Option<LineShader>,
}

struct Ecs {
    content: Vec<Thing>,
}

impl Ecs {
    fn push(&mut self, e: Thing) -> usize {
        let id = self.content.len();
        self.content.push(e);
        id
    }

    fn pos_of(&self, id: usize) -> glm::Vec2 {
        self.content[id]
            .translation_component
            // Replace with zero translation if component not present.
            .unwrap_or(vec2(0.0, 0.0))
    }
}

impl Thing {
    /// Create a thing with no components.
    fn new() -> Self {
        Thing {
            typeset_text_component: None,
            text_shader_component: None,
            translation_component: None,
            scale_component: None,
            animation_component: None,
            circle_component: None,
            circle_shader_component: None,
            line_component: None,
            line_shader_component: None,
            bezier_component: None,
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

    fn line_shader(mut self, shader: LineShader) -> Self {
        self.line_shader_component = Some(shader);
        self
    }

    fn circle_shader(mut self, shader: CircleShader) -> Self {
        self.circle_shader_component = Some(shader);
        self
    }

    fn scale(mut self, scale: f32) -> Self {
        self.scale_component = Some(scale);
        self
    }

    fn translate(mut self, trans: glm::Vec2) -> Self {
        self.translation_component = Some(trans);
        self
    }

    fn animation(mut self, anim: impl 'static + Fn(&mut Thing, &Ecs)) -> Self {
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

    fn bezier(mut self, bezier: Cubic) -> Self {
        self.bezier_component = Some(bezier);
        self
    }

    // Accessors for filter_map based systems.

    fn typeset_text_component(
        &self,
    ) -> Option<(
        &Typeset,
        Option<glm::Vec2>,
        Option<f32>,
        Option<&TextShader>,
    )> {
        self.typeset_text_component.as_ref().map(|typeset| {
            (
                typeset,
                self.translation_component,
                self.scale_component,
                self.text_shader_component.as_ref(),
            )
        })
    }

    fn circle_component(
        &self,
    ) -> Option<(&CircleElement, Option<glm::Vec2>, Option<&CircleShader>)> {
        self.circle_component.as_ref().map(|circ| {
            (
                circ,
                self.translation_component,
                self.circle_shader_component.as_ref(),
            )
        })
    }

    fn line_component(&self) -> Option<(&LineElement, Option<glm::Vec2>, Option<&LineShader>)> {
        self.line_component.as_ref().map(|line| {
            (
                line,
                self.translation_component,
                self.line_shader_component.as_ref(),
            )
        })
    }

    fn bezier_component(&mut self) -> Option<(&Cubic, &mut Option<LineElement>, Option<f32>)> {
        self.bezier_component
            .as_ref()
            .map(|bez| (bez, &mut self.line_component, self.scale_component))
    }

    fn animation_component(&self) -> Option<(Arc<dyn Fn(&mut Thing, &Ecs)>, &mut Thing)> {
        // This gives the animation component the ability to even replace the
        // entities animation component. it is incredibly bad, but a lot easier
        // to implement, and a lot more flexible, than safe alternatives.
        let danger = unsafe {
            let im = self as *const Thing;
            let sorry = im as *mut Thing;
            &mut *sorry
        };

        if let Some(anim) = &self.animation_component {
            return Some((anim.clone(), danger));
        }

        None
    }
}

/// Re-tesselation system.
/// This takes every entity with a Bezier component, and updates
/// its linear spline component by completely re-calculating the
/// tesselation.
unsafe fn retesselate(state: &mut State) {
    for (bez, tess, mb_scale) in state
        .ecs
        .content
        .iter_mut()
        .filter_map(Thing::bezier_component)
    {
        let spline = Segment::spline(&bez.sample());
        let scale = mb_scale.unwrap_or(1.0);
        if let Some(tess) = tess {
            tess.update(spline.segments(), scale);
        } else {
            tess.replace(LineElement::new(spline.segments(), scale));
        };
    }
}

/// Animation system
fn animate(state: &mut State) {
    for (anim, thing) in state
        .ecs
        .content
        .iter()
        .filter_map(Thing::animation_component)
    {
        anim(thing, &state.ecs);
    }
}

/// Rendering "system" :v)
unsafe fn render(state: &State) {
    gl::ClearColor(1.0, 1.0, 1.0, 1.0);
    gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

    // There are a wide variety of "renderable" components, including
    // text, textured quads, circles, lines, Bézier curves, etc.

    for thing in &state.ecs.content {
        if let Some((renderable, maybe_trans, maybe_scale, maybe_shader)) =
            thing.typeset_text_component()
        {
            // Substitute default text shader in the absence of a specific one.
            let text_shader = maybe_shader.unwrap_or(&state.default_text_shader);
            let trans = maybe_trans.unwrap_or(vec2(0.0, 0.0));
            let scale = maybe_scale.unwrap_or(1.0);
            let text_transform = Transform {
                scale,
                translation: (trans.x, trans.y),
            };
            renderable.traverse_scenegraph(&state.text_renderer, &text_transform, text_shader);
        }

        if let Some((circle, maybe_trans, maybe_shader)) = thing.circle_component() {
            let circle_trans = maybe_trans.unwrap_or(vec2(0.0, 0.0));
            let circle_shader =
                maybe_shader.unwrap_or(&state.circle_renderer.default_circle_shader);
            circle.rasterize(&state.circle_renderer, circle_trans, circle_shader);
        }

        if let Some((line, maybe_trans, maybe_shader)) = thing.line_component() {
            let line_trans = maybe_trans.unwrap_or(vec2(0.0, 0.0));
            let shader = maybe_shader.unwrap_or(&state.line_renderer.default_line_shader);

            line.rasterize(&state.line_renderer, line_trans, shader);
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

        // Shared mouse position
        let mouse = Arc::new(RwLock::new((0.0, 0.0)));

        // Give it translation defined by mouse position.
        let m = mouse.clone();

        let mut ecs = Ecs {
            content: Vec::new(),
        };

        // Frame timing display.
        let fps = Arc::new(RwLock::new(TextElement::new(" ", &atlas)));
        ecs.push(
            Thing::new()
                .typeset_text(Typeset::elem(fps.clone()))
                .translate(vec2(10.0, 20.0))
                .scale(40.0),
        );

        // SVG test.
        let outline = plox::svg::parse(plox::svg::RUST_LOGO_SVG_SRC);
        let svg = TextElement::outlined(outline);
        ecs.push(Thing::new()
                 .scale(150.0)
                 .translate(vec2(10.0, 60.0))
                 .typeset_text(Typeset::elem(Arc::new(RwLock::new(svg)))));

        ecs.push(
            Thing::new()
                .typeset_text(Typeset::elem(Arc::new(RwLock::new(TextElement::new("SVG:", &atlas)))))
                .translate(vec2(10.0, 220.0))
                .scale(40.0),
        );

        let outline = plox::svg::parse(plox::svg::MAXWELL_SVG_SRC);
        let svg = TextElement::outlined(outline);
        ecs.push(Thing::new()
                 .scale(150.0)
                 .translate(vec2(10.0, 260.0))
                 .text_shader(Shader::fancy_blit().into())
                 .typeset_text(Typeset::elem(Arc::new(RwLock::new(svg)))));

        let outline = plox::svg::parse(plox::svg::FT_SVG_SRC);
        let svg = TextElement::outlined(outline);
        ecs.push(Thing::new()
                 .scale(350.0)
                 .translate(vec2(10.0, 425.0))
                 .typeset_text(Typeset::elem(Arc::new(RwLock::new(svg)))));


        // The initial position of the Bézier (to provide some defaults).
        let bezier = Cubic::pts(
            vec2(200.0, 200.0),
            vec2(300.0, 150.0),
            vec2(470.0, 250.0),
            vec2(500.0, 150.0),
        );

        //
        // Control points.
        //

        let p1 = ecs.push(Thing::new().translate(bezier.p0));
        let p2 = ecs.push(Thing::new().translate(bezier.p1));

        let p3 = ecs.push(
            Thing::new()
                .translate(bezier.p2)
                .animation(move |thing, _| {
                    let (x, y) = *m.read().unwrap();
                    thing.translation_component.replace(vec2(x, y));
                }),
        );

        let p4 = ecs.push(Thing::new().translate(bezier.p3));

        //
        // Line between control points.
        //

        ecs.push(
            Thing::new()
                .line(LineElement::line(bezier.p0, bezier.p1, 2.0))
                .animation(move |thing, ecs| {
                    if let Some(com) = &mut thing.line_component {
                        com.update_line(ecs.pos_of(p1), ecs.pos_of(p2), 2.0);
                    }
                }),
        );

        ecs.push(
            Thing::new()
                .line(LineElement::line(bezier.p2, bezier.p3, 2.0))
                .animation(move |thing, ecs| {
                    if let Some(com) = &mut thing.line_component {
                        com.update_line(ecs.pos_of(p3), ecs.pos_of(p4), 2.0);
                    }
                }),
        );

        //
        // The actual Bézier
        //

        let spline = Segment::spline(&bezier.sample());
        let line = LineElement::new(spline.segments(), 2.0);

        ecs.push(
            Thing::new()
                .bezier(bezier)
                .scale(2.0)
                .line(line)
                .line_shader(Shader::fancy_line().into())
                .animation(move |thing, ecs| {
                    thing.bezier_component.replace(Cubic {
                        p0: ecs.pos_of(p1),
                        p1: ecs.pos_of(p2),
                        p2: ecs.pos_of(p3),
                        p3: ecs.pos_of(p4),
                    });
                }),
        );

        //
        // Displayed control points.
        //

        ecs.push(
            Thing::new()
                .circle(CircleElement::new(5.0).width(2.0))
                .animation(move |thing, ecs| {
                    thing.translation_component.replace(ecs.pos_of(p1));
                }),
        );

        ecs.push(
            Thing::new()
                .circle(CircleElement::new(5.0).width(2.0))
                .animation(move |thing, ecs| {
                    thing.translation_component.replace(ecs.pos_of(p2));
                }),
        );

        ecs.push(
            Thing::new()
                .circle(CircleElement::new(5.0).width(2.0))
                .animation(move |thing, ecs| {
                    thing.translation_component.replace(ecs.pos_of(p4));
                }),
        );


        ecs.push(
            Thing::new()
                .circle(CircleElement::new(10.0).width(2.0))
                .animation(move |thing, ecs| {
                    thing.translation_component.replace(ecs.pos_of(p3));
                })
                .circle_shader(Shader::fancy_circle().into()),
        );


        State {
            win_dims: (SCREEN_W, SCREEN_H),
            mouse,
            atlas,
            fps,
            text_renderer,
            default_text_shader: default_text_shader.into(),
            circle_renderer,
            line_renderer,
            ecs,
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
                let beg = std::time::Instant::now();
                animate(&mut state);
                unsafe {
                    retesselate(&mut state);
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
