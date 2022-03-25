---
title: Graphics project
author: Steffen Haug
colorlinks: true
---
# Font loading
- used ruztybuzz

# Glyph rasterization
The first step of glyph rasterization is to convert the FreeType-strokes into cubic Bézier curves. It would simplify matters _massively_ to restrict ourselves to _quadratic_ Bézier curves, and indeed this is what most material online does, and this is perfectly fine if restricting ourselves to _TrueType_ fonts, as these are simply made up of quadratics, but the problem is that _OpenType_ fonts use _cubic_ curves, so restricting outselves to quadratics means we either have to choose a different font (the \LaTeX-font is `.otf`) or _approximate_ the cubic Bézier curves.
It is possible to obtain `.ttf` versions of the \LaTeX-font online, but for the sake of argument, lets assume we want to maybe let the user configure the font, then restricting to `.ttf` is not super user-friendly.

The next step, once the Bézier curves have been computed from the font, is to _fill_ the character. This turns out to be fairly non-trivial, mostly because we use cubics.
What we want is a way to check -- independently -- if a point lies within the boundary of the glyph (independent because we want to do it in a fragment shader, of course).
There are numerous ways to do this, many of which are described online.
A popular approach for 3D graphics systems that need performance _at all costs_ use an approach known as _signed distance functions_, which are (pre-)computed for rasterized glyps which lets you quickly approximate a fragments position relative to the curve at runtime, but at the cost of loss of detail -- sharp features are rounded out. This is precisely what we _don't want_ since the \LaTeX-font has extremely thin serifs.
Another common approach is to make use of the _winding number_ for a given point, and this is what I will do.

The principle is fairly simple, but the result is suprising: No matter if a glyph has holes or non-convex boundaries, the correct winding-number can be calculated by checking for intersections along _any_ ray emitting from a point.
As far as i can tell, this is known as Dan Sunday's winding number algorithm.
A description of something similar (in the context of glyph rendering) is given [here](https://wdobbie.com/post/gpu-text-rendering-with-vector-textures/).
This requires us to know two things: How do we know if a Bézier curve intersects with an arbitrary line -- say a horizontal line?
And how do we know which _direction_ the line is going at this point?
Essentially, for a given Bézier curve $B(t)$, for which $t$ is $B_y(t) = 0$,
and what is $dB_y / dt$ at this $t$?

An alternate representation for the (cubic) Bézier curve $B$, which is typically most readily expressed by linear interpolation between its control points
$P_0, P_1, P_2, P_3$ is via _Bernstein polynomials_:

\begin{align*}
B(t) &=  (1 - t)^3 P_0 \\
     &+ 3(1 - t)^2 t P_1 \\
     &+ 3(1 - t)   t^2 P_2 \\
     &+       t^3 P_3
\end{align*}

which is incredibly ugly, but fortunately can be
[automatically](
https://www.symbolab.com/solver/step-by-step/P_%7B0%7D%5Cleft(1%20-%20t%5Cright)%5E%7B3%7D%20%2B%203P_%7B1%7D%5Cleft(1-t%5Cright)%5E%7B2%7D%20%5Ccdot%20t%20%2B%203P_%7B2%7D%5Cleft(1-t%5Cright)%20%5Ccdot%20t%5E%7B2%7D%20%2B%20P_%7B3%7D%20t%5E%7B3%7D?or=input
)
expanded, so we just need to collect terms of equal powers of $t$:

\begin{align*}
B(t) &=  P_0 \\
     &+ (- 3 P_0 + 3 P_1)t \\
     &+ (3 P_0 - 6 P_1 + 3 P_2) t^2 \\
     &+ (-P_0 + 3 P_1 - 3 P_2 + P_3) t^3
\end{align*}

recalling that $P_1 \dots P_3$ are just 2D points that we are interpolating between, the $y$-coordinate of the interpolated point can be obtained simply by interpolating between $y_1 \dots y_3$. (If this is unclear, think of a point $P$ as a linear combination of its $x$- and $y$-component, and remember that scaling and adding preserves linear combinations, which is all linear interpolation really does).
Making the substitutions
$c = y_0$,
$b = - 3 y_0 + 3 y_1$,
$a = 3 y_0 - 6 y_1 + 3 y_2$, and
$d = -y_0 + 3 y_1 - 3 y_2 + y_3$,
we thus have
\begin{align*}
B_y(t) &= d t^3 + a t^2 + b t + c \\
\frac {dB_y} {dt} &=
    3d t^2 + 2a t + b
\end{align*}
which is finally starting to look sensible.
Noting that multiplying by a constant does not change the roots, and
that we do not care about the magnitude of the derivative, only the sign, we divide through by $d$ to get:
\begin{align}
B_y(t) &=  t^3 + a t^2 + b t + c \\
\frac {dB_y} {dt} &=
    3 t^2 + 2a t + b
\end{align}
