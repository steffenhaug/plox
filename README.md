graphics project for tdt4230
# gpu-accelerated plotting.
(plox = plot + oxidize or some shit)

(and by plotting i mean falling head first into the text rendering rabbit hole)

(plese send help)

# inspiration, background
- making _nice_ plots in Python is really annoying
    - latex as backend, generate postscript
    - doesnt work for animations at all
    - very slow

- inspired by beautiful maths videos
    - 3b1b (https://www.youtube.com/watch?v=p_di4Zn4wz4 ~5:30)
    - freya holmér (https://www.youtube.com/watch?v=aVwxzDHniEw&t=292s   0:40)
    - mathologer (https://www.youtube.com/watch?v=N-KXStupwsc&t=1806s  30:45)

- need a tool that can create figures that are
    - animated (60 fps+ videos)
    - _high quality_ typography, meaning: scale independant, no artifacts
        - texture atlas and distance fields immediately disqualified

- main challenge is text, but also
    - arbitrarty quadratic spline contours
    - lines & dots
    - circle arcs
    - axes, grid
    - dotted lines

problem: implicit geometry is difficult in the gpu pipeline.

Uses a variant of Evan Wallaces color flipping method but replaces the color accumulation
techniques for winding number calculation and anti-aliasing by drawing with XOR color logic 
into a multisample α-texture.

Disadvangage: This method generates a _fuck ton_ of vertices.
Storing the curves in pixel-coordinates would speed up vertex processing, so that's
a possible optimization.


## tentative progression plan:
1. ✅font loading (utilize a libraty hopefully) and obtain bezier curves
2. ✅single-character software rasterizer (write a png or something)
    - figure out AA here
3. ✅single-character hardware rasterizer
4. ✅application of transformations to single character (translation and scaling)
6. ✅ (kinda) typesetting of "latex"
7. typesetting of non-character bezier-curve based things (axes, tick marks, etc)

## immediate to-do list:
- ECS restructure
    - animation done via an update system?
- profiling, tracing and logging
- line breaking algorithm (i think this is easy) needed for labels if they get long
    without justification, just group characters separated by space
        - push the text if it fits
        - if the text would extend past right margin, reset cursor x to zero and
          add baseline-skip to cursor y

## future work
- restructure rasterization of typeset expressions to mask the whole expression in one go and then rasterize
    - saves a lot of shader switches
- latex parser or some shit
- run Lua/Fennel in a separate thread with an event loop proxy
- better gl abstraction or use vulkan
- replace the arc<rwlock> mess (crossbeam atomic cells maybe)

## simplifying assumptions:
- use latex font (include in repo, ttf only, hardcoded path, cant go wrong) legal to redistribute

∫

## resources
### curves in general + backround math
https://www.youtube.com/watch?v=aVwxzDHniEw&t=1282s freya bezier curves
https://www.youtube.com/watch?v=N-KXStupwsc   MATHOLOGER CUBIC

https://news.ycombinator.com/item?id=30901537

https://blog.mecheye.net/2019/05/why-is-2d-graphics-is-harder-than-3d-graphics/

FONT FORGE FONT RELATED MATH
https://fontforge.org/docs/techref/pfaeditmath.html

FONT RENDERING PIPELINE
https://mrandri19.github.io/2019/07/24/modern-text-rendering-linux-overview.html

https://pomax.github.io/bezierinfo/

FontForge to inspect curve data

curve preprocessing:
https://www.sirver.net/blog/2011/08/23/degree-reduction-of-bezier-curves/

algorithm for tesselating bezier curves
https://github.com/alexheretic/ab-glyph/blob/master/rasterizer/src/raster.rs


https://crates.io/crates/hyphenation

### opengl in general
https://www.glprogramming.com/red/chapter10.html

maybe this can simplify anti aliasing 
https://www.khronos.org/registry/OpenGL-Refpages/gl4/html/fwidth.xhtml

MSAA texture
https://stackoverflow.com/questions/42878216/opengl-how-to-draw-to-a-multisample-framebuffer-and-then-use-the-result-as-a-n

Specifically Off-screen AA
https://learnopengl.com/Advanced-OpenGL/Anti-Aliasing

read about Blitting, might be possible to not invoke the shader pipeline
to copy rasterized texture to the screen
i think what i _really_ want is a multisampled renderbuffer and not a texture, since
im literally just copying

### wallaces method
http://www.glprogramming.com/red/chapter14.html#name13 backround on stencil buffer flippign method
https://medium.com/@evanwallace/easy-scalable-text-rendering-on-the-gpu-c3f4d782c5ac - winding nuymber / bezier curves
https://www.microsoft.com/en-us/research/wp-content/uploads/2005/01/p1000-loop.pdf
https://news.ycombinator.com/item?id=11440599

### font stuff
https://simoncozens.github.io/fonts-and-layout/opentype.html

https://people.eecs.berkeley.edu/~fateman/temp/neuform.pdf

MATH symbols unicode table
https://unicode-search.net/unicode-namesearch.pl?term=MATHEMATICAL
