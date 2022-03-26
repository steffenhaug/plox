---
title: Graphics project
author: Steffen Haug
colorlinks: true
links-as-notes: true
header-includes:
- \usepackage{mathtools}
- \interfootnotelinepenalty=10000
classoption:
- twocolumn
---
# Goal
- rasterize bezier curves
- rasterize text _properly_
- plots
- performance is not a goal; correctness is
    - rationale: I'm not trying to rasterize all of wikipedia, workloads will be relatively small

# Bézier curves background
### Definition
- bezier curve
- spline as seq of bezier curves

### Rasterization
- software rasterization is trivial: map over t
- hardware rasterization is easy but slightly harder:
    - (x, y) is on the curve if B(t) - (x, y) = 0
    - amounts to solving a polynomial equation


# Font loading
- used ruztybuzz

# Glyph rasterization
The first step of glyph rasterization is to convert the FreeType-strokes into cubic Bézier curves. It would simplify matters _massively_ to restrict ourselves to _quadratic_ Bézier curves, and indeed this is what most material online does, and this is perfectly fine if restricting ourselves to _TrueType_ fonts, as these are simply made up of quadratics, but the problem is that _OpenType_ fonts use _cubic_ curves, so restricting ourselves to quadratics means we either have to choose a different font (the \LaTeX-font is `.otf`) or _approximate_ the cubic Bézier curves. To be clear, this approximation of one lower order can be done quickly and accurately, so it is an interesting option should we meet performance problems with the high-order techniques.
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
$P_0, P_1, P_2, P_3$ is via _Bernstein polynomials_,

$$
\arraycolsep=0.1em
\begin{array}{rcllll}
B(t) & = & P_0 &   & (1 - t)^3 &  \\
     & + & P_1 & 3 & (1 - t)^2 & t  \\
     & + & P_2 & 3 & (1 - t)   & t^2  \\
     & + & P_3 &   &           & t^3 
\end{array}
$$

which as you can see, is just a linear combination of the control points, each weighted by
polynomials of that "trade off" between $t$ and $t-1$. That's very pretty, but doesn't
help us much in finding the roots. We rearrange in terms of powers of $t$:^[By using
WolframAlpha or somthing like that of course]

\begin{align*}
B(t) &=  P_0 \\
     &+ (- 3 P_0 + 3 P_1)t \\
     &+ (3 P_0 - 6 P_1 + 3 P_2) t^2 \\
     &+ (-P_0 + 3 P_1 - 3 P_2 + P_3) t^3
\end{align*}

## Solving the cubic
Solutions to cubic equations like $B(t) = 0$ has an explicit formula, akin to the famous quadratic formula,
just a little bit hairier. I will provide an explanation of how to get $B(t)$ in a standardized
form to apply the formula, because it is a central part of the fragment shader, but the details
are really not super important: The important thing is that the _is a way_ to find the roots
in constant time with a reasonable number of floating point operations, so feel free to gloss
over. Sometimes I might introduce things without proof or motivation for brevity.
A very nicely motivated explanation of this formula is given in
[a highly entertaining YouTube video by Mathologer](https://www.youtube.com/watch?v=N-KXStupwsc).

Recalling that $P_1 \dots P_3$ are just 2D points that we are interpolating between,
the $y$-coordinate of the interpolated point can be obtained simply by interpolating 
between $y_1 \dots y_3$. (If this is unclear, think of a point $P$ as a linear 
combination of its $x$- and $y$-component, and remember that scaling and adding 
preserves linear combinations).
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

Next, we introduce a standard coordinate shift
$$
t = s - \frac a 3 \implies B(s) = s^3 + ps + q
$$
where
$$
p = \frac {3 b - a^2} 3 \quad \text{and} \quad q = \frac {2a^3 - 9ab + 27c} {27}
$$
and solutions to _this_ polynomial (almost) has a neat explicit formula. The discriminant
$$
\Delta = \left(
    \frac q 2
\right)^2
+ \left(
    \frac p 3
\right)^3
$$
tells which of a few special cases we have:
$\Delta < 0 \implies \text{three real solutions}$,
$\Delta = 0 \implies \text{two real solutions}$, and
$\Delta > 0 \implies \text{one real solution}$.
The $\Delta \geq 0$ cases are simple: The formula for the solution $s$ can be expressed using $\sqrt \Delta$:
\begin{equation}
u = \sqrt[3]{
- \frac q 2 - \sqrt \Delta
}
\quad\text{and}\quad
v = \sqrt[3]{
- \frac q 2 + \sqrt \Delta
}
\end{equation}
and the solutions are
\begin{equation}
s_1 = u + v
\quad\text{and}\quad
s_{2,3} = - \frac 1 2 (u + v) \pm (u - v) \frac {\sqrt 3} 2 i
\end{equation}
To see how this gives one and two real solutions, notice that in the $\sqrt \Delta = 0$ case,
$u = v \implies u - v = 0$, so one of the complex solutions "becomes" real, thus giving us another.


However, in the $\Delta < 0$ case (in which, remember, we have three _real_ solutions), $\sqrt \Delta$
will always be a _complex_ number. In fact, all three solutions are actually sums of _complex
conjugates_! Our formula still technically gives correct answers, but it is _annoying_ do do in a shader
because representing $u$ and $v$ as complex numbers requires implementing the basic arithmetic
ourselves. We can circumvent the complex arithmetic entirely by writing the conjugates in polar
coordinates:
$$
r = \sqrt { \left(
- \frac  p 3
\right) ^3}
\quad \text{and} \quad
\vartheta = \mathrm{atan2} \left(
\sqrt {-\Delta}, - \frac q 2
\right)
$$
In which case our solutions are
\begin{align}
\begin{split}
s_0 &= 2 \sqrt[3] r \cos \frac {\vartheta} 3 \\
s_1 &= 2 \sqrt[3] r \cos \frac {\vartheta + 2 \pi} 3 \\
s_2 &= 2 \sqrt[3] r \cos \frac {\vartheta + 4 \pi} 3
\end{split}
\end{align}
and to get our solution back in terms of $t$, simply apply the coordinate shift $t = s - a / 3$
again.

Now, the algorithm to calculate a point $P$s winding number in a region $\Omega$
outlined by the a Bézier spline $(B(t))_i$ simply amounts to finding
the solutions $t$ for $B_y(t) = P_y$ such that $B_x > P_x$.
Intuitively: All the points where a ray eminating from $P$ towards positive $x$ 
direction crosses the boundary.
We characterize these solutions based on whether they are _leaving_ or _entering_ $\Omega$.
The convention based on the orientation of $B$ is that the interior of $\Omega$ is always
"to our left", in other words if $dB_y / dt > 0$ (the curve is "moving upwards") the ray is _leaving_
$\Omega$, and analogously if $dB_y / dt < 0$ the ray is _entering_ $\Omega$.

Arbitrarily, we may for example say that a solutions contribution to the winding 
number should be +1 if the ray is entering $\Omega$, and -1 if leaving.
With this convention, fragments with negative winding numbers are _inside_ $\Omega$.
Intuitively: If the winding number is negative, we have _left_ more often than we have _entered_,
so in sum the ray eminating from this point is _leaving_ $\Omega$, and thus the point is inside.

![The glyph $\alpha$ with its boundary colored based on $dB_y/dt$, and samples in a lattice with
net negative winding number indicated.](report/alpha_diff_outline.png)

To summarize, we can rasterize a region $\Omega$ outlined by a spline
$(B(t))_{i=0}^n$ in $O(n)$ time where $n$ is the number of Bézier curves in the spline.
The spline is not restricted to be just one glyph! We can just add the curves corresponding
to other glyphs (at the corect offsets) in the same buffer, but since every fragment has 
to check solutions against every curve, there might still be a performance benefit to breaking
strings into separate draw calls, although at the expense of more state transitions.
