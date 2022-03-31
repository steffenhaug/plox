# gpu-accelerated plotting.
(plox = plot + oxidize or some shit)


## notes on text rendering
there are essentially two options:
- rasterizing text to a texture, and drawing it on a quad
    - probably easy to do at load time with external lib
    - will be less flexible, no dynamic text probably
- 1 glyph = 1 quad with (bezier?) curves in texture
    - SIGNIFICANTLY harder
    - flexible


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
- rendering of multiple text elements
    - verts need curve index range
    - TextRenderer needs to manage the curve buffer
        - does this approach scale? is it more sensible to upload the
          entire font atlas?
            pros:
            - no need to "collect garbage" if text elements are removed
            - curve buffer doesnt grow for ever if text is added
            cons:
            - need to send code points as a uniform with each draw call.
              this is probably cheap; way more compact than curve data

- profiling, tracing and logging



## things
check if allsorts shaper has better API

## simplifying assumptions:
- use latex font (include in repo, otf only, hardcoded path, cant go wrong) legal to redistribute

∫

## resources
https://medium.com/@evanwallace/easy-scalable-text-rendering-on-the-gpu-c3f4d782c5ac - winding nuymber / bezier curves
https://github.com/rougier/freetype-gl - distance fields
https://www.youtube.com/watch?v=aVwxzDHniEw&t=1282s freya bezier curves
https://jcgt.org/published/0006/02/02/paper.pdf <- this one is great
https://pcwalton.github.io/2017/02/14/pathfinder
https://medium.com/@raphlinus/inside-the-fastest-font-renderer-in-the-world-75ae5270c445
https://www.microsoft.com/en-us/research/wp-content/uploads/2005/01/p1000-loop.pdf
https://simoncozens.github.io/fonts-and-layout/opentype.html

curve preprocessing:
https://www.sirver.net/blog/2011/08/23/degree-reduction-of-bezier-curves/

MATH symbols unicode table
https://unicode-search.net/unicode-namesearch.pl?term=MATHEMATICAL

FontForge to inspect curve data

https://pomax.github.io/bezierinfo/

https://www.youtube.com/watch?v=N-KXStupwsc   MATHOLOGER CUBIC

FONT FORGE FONT RELATED MATH
https://fontforge.org/docs/techref/pfaeditmath.html

https://mrandri19.github.io/2019/07/24/modern-text-rendering-linux-overview.html

maybe this can simplify anti aliasing 
https://www.khronos.org/registry/OpenGL-Refpages/gl4/html/fwidth.xhtml


## rust-libraries
rustybuzz - text shaping. not sure if i need this if im gonna typeset math myself but anyway
