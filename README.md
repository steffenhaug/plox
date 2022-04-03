graphics project for tdt4230
# gpu-accelerated plotting.
(plox = plot + oxidize or some shit)

## tentative progression plan:
1. ✅font loading (utilize a libraty hopefully) and obtain bezier curves
2. ✅single-character software rasterizer (write a png or something)
    - figure out AA here
3. ✅single-character hardware rasterizer
4. ✅application of transformations to single character (translation and scaling)
5. typesetting engine that parses "latex" into a scene graph
    - special symbols have hardcoded rules
    - use kerning data
6. typesetting of "latex"
7. typesetting of non-character bezier-curve based things (axes, tick marks, etc)

## immediate to-do list:
- profiling, tracing and logging
- text elements need to know their total bounding box
  and use this as a reference point for translation
  either that, or wrap them in a "box" abstraction (ala fbox)
- design layout system
    option 1: hard code layouts for integrals etc
    option 2: generic scaled boxes, overset/underset boxes
- "latex" parser -> scene graph
- figure out dynamic text. simple idea: arc<refcell<text>> and store the arc
  in the renderer
- line breaking algorithm (i think this is easy) needed for labels if they get long
    without justification, just group characters separated by space
        - push the text if it fits
        - if the text would extend past right margin, reset cursor x to zero and
          add baseline-skip to cursor y
- drawing outlines of boxes for debugging
    - line shader: dashed, dotted, etc



## things
check if allsorts shaper has better API

## simplifying assumptions:
- use latex font (include in repo, otf only, hardcoded path, cant go wrong) legal to redistribute

∫

## resources
### curves in general + backround math
https://www.youtube.com/watch?v=aVwxzDHniEw&t=1282s freya bezier curves
https://www.youtube.com/watch?v=N-KXStupwsc   MATHOLOGER CUBIC

FONT FORGE FONT RELATED MATH
https://fontforge.org/docs/techref/pfaeditmath.html

FONT RENDERING PIPELINE
https://mrandri19.github.io/2019/07/24/modern-text-rendering-linux-overview.html

https://pomax.github.io/bezierinfo/

FontForge to inspect curve data

curve preprocessing:
https://www.sirver.net/blog/2011/08/23/degree-reduction-of-bezier-curves/

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

MATH symbols unicode table
https://unicode-search.net/unicode-namesearch.pl?term=MATHEMATICAL


## rust-libraries
rustybuzz - text shaping. not sure if i need this if im gonna typeset math myself but anyway


# performance
It is pretty good.
creating font atlases for large fonts takes milliseconds, courtesy of rayon.
rasterization is expensive, but there is still room for optimizing the shader, its
a very naïve multisampling in the current iteration.
Specifically, ~4x speedup should theoretically be possible just by reusing roots.
I'm currently re-calculating the bounding box of glyphs when shaping, even though this data is
cached in the font atlas, because it is convenient. There is some time to gain here.

```
valgrind --tool=callgrind --dump-instr=yes --collect-jumps=yes --simulate-cache=yes target/release/plox-example
```

