---
title: Graphics project
author: Steffen Haug
colorlinks: true
links-as-notes: true
header-includes:
- \usepackage{mathtools}
- \usepackage{xcolor}
- \interfootnotelinepenalty=10000
classoption:
- twocolumn
---
# Goal
- rasterize bezier curves
- rasterize text _properly_
- plots
- performance is not a goal; correctness is.
    - rationale: I'm not trying to rasterize all of wikipedia, workloads will be relatively small

# Bézier curves background
### Definition
- bezier curve
- spline as seq of bezier curves

### Rasterization
- software rasterization is trivial: map over t
- hardware rasterization is slightly harder:
    - (x, y) is on the curve if B(t) - (x, y) = 0
    - amounts to solving a polynomial equation


# Font loading
- used ttf_parser to parse files
- used ruztybuzz for shapign; port of harfbuzz's shaping algo

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
That sounds fantastic, and it almost is! But there is a big prpblem:
The calculation outlined suffers badly from numerical precision!
Particularily where Bézier curves are joined in points that are tangent to the
ray.

![Horizontal artefacts as a result of problems related to numberical precision. Yikes!](report/problems.png)

Since this artefacting stems from numerical imprecision, it is extremely
sensitive to translation of the glyphs. In fact, the position of the nice-looking $\alpha$ was _very_ carefully
selected so as to not have any visible artefacts. In other words, while mathematically correct,
this technique is not _quite_ feasible. Thankfully, it is not that hard to salvage.
[a paper by Eric Lengyel](https://jcgt.org/published/0006/02/02/paper.pdf) describes
an incredibly elegant lookup-table based way to compute the winding number.
This approach requires restricting our curves to quadratics after all.
Perhaps delving into all these calculations was a waste of time, but i think outlining
the problems of this approach in detail is valuable. I also think it is important to document
the things I have learned that _didn't_ make it into the final implementation.
Anyways, our next task is splitting cubic spline segmments into quadratics.
This is outlined for the general case in
[an online article](https://www.sirver.net/blog/2011/08/23/degree-reduction-of-bezier-curves/),
but we have it easy since we have a specific order in mind.


## Degree reduction of Bézier curves
In the general case, the matrix $\mathrm M$ that lifts a Bézier curve from
degree $n-1$ to degree $n$ is $n + 1 \times n$. Think about it: We want to apply
the matrix to $n$ control points and get $n + 1$ control points.
If we were to raise quadratic curves ($n-1=2$) we would need a $4 \times 3$ matrix.
$$
\renewcommand\arraystretch{1.3}
\mathrm M = \begin{pmatrix}
1         &           &            \\
\frac 1 3 & \frac 2 3 &            \\
          & \frac 2 3 & \frac 1 3  \\
          &           &         1  \\
\end{pmatrix}
$$
We make use, without proof, of the fact that
\begin{equation}
B_{n-1} = (\mathrm M ^T \mathrm M)^{-1} \mathrm M ^T B_{n}
\end{equation}
but note that since the dimensions are given, the product can be pre-computed:
$$
\mathrm M' \coloneqq \frac 1 {20} \begin{pmatrix}
19 & 3 & -3 & 1 \\
-5 & 15 & 15 & -5 \\
1 & -3 & 3 & 19
\end{pmatrix}
$$
and
\begin{equation}
\mathrm M' \begin{pmatrix}
P_0 \\ P_1 \\ P_2 \\ P_3
\end{pmatrix}
= \frac 1 {20} \begin{pmatrix}
19 P_0 +  3 P_1 -  3 P_2 +    P_3 \\
-5 P_0 + 15 P_1 + 15 P_2 -  5 P_3 \\
   P_0 -  3 P_1 +  3 P_2 + 19 P_3
\end{pmatrix}
\end{equation}
Not exactly pretty! But how does it perform?

![The \LaTeX-characters $\alpha \rightarrow \vartheta$ approximated with quadratic Bézier curves.](report/awful.png)

It is _awful_! :-) Even though it is a "good" approximation, this strategy even moves the
end-points.
Now, there are better ways to do this, by for example splitting the high-order curve into low-order
curves and calculating a low-order curve based on points on the line, for example at
$t=0,t=0.5,t=1$, but that sounds like an awful amount of work when `.ttf` versions of the font is
easy to generate with something like FontForge. Now, I wrote earlier that i wanted to use
`.otf` fonts as a possibility so we can be maximally flexible with the users choice of fonts, but
that sounds like a problem for future me!

![The \LaTeX-characters $\alpha \rightarrow \vartheta$ approximated using
FontForge.](report/ttf_sample.png)

With fonts converted with FontForge the result is indistinguishable from the higher order.
In fact, lower-order approximations can be done to arbitrary precision, FontForge has just inserted
way more curves!

## Bézier curve equivalence classes
I think outlining how this winding number calculation works is worth a section, even though all the math is
a lot simpler compared to the cubic case I just described, and the LUT is just implemented verbatim
from the referenced article. The LUT solution is so _mind-bogglingly clever_ that I can't help
myself.

First of all, the Bernstein polynomial representation of a quadratic Bézier curve is
$$
\arraycolsep=0.1em
B(t) = 
\begin{array}{cllll}
  & P_0 &   & (1 - t)^2 &  \\
      + & P_1 & 2 & (1 - t)   & t  \\
      + & P_2 &   & (1 - t)   & t^2  \\
\end{array}
= \begin{array}{cll}
    &     P_0                &   \\
  + & (-2 P_0 + 2 P_1)       & t \\
  + & (   P_0 - 2 P_1 + P_2) & t^2
\end{array}
$$
which clearly means that given
$a \coloneqq P_0 - 2P_1 + P_2$,
$b \coloneqq -2 P_0 + 2 P_1$, and
$c \coloneqq P_0$,
\begin{equation}
t_{1,2} = \frac {-b \pm \sqrt {b^2 - 4ac}} {2a}
\end{equation}
The winding number is then calculated in the same way:
Given a point P, imagine a ray in (for example) positive $x$-direction,
find all the intersections, categorize them based on whether we are entering or leaving $\Omega$,
and sum up. But instead of doing the error-prone numerical categorization, there is a genius 
trick we can employ to categorize the solutions. If we calculate `jmp` as follows:
```Rust
// Calculate the jump:
let jmp = if y0 > 0.0 { 8 } else { 0 }
        + if y1 > 0.0 { 4 } else { 0 }
        + if y2 > 0.0 { 2 } else { 0 };
```
Notice that for any distinct combination of point configurations (configuration, in this context,
meaning whether a point is "above" the ray or not  -- i.e. whether $y > 0$), `jmp` will take
distinct values from $\{ 0, 2, 4, 6, \dots, 14\}$. For example, if $P_0, P_1$ is above,
and $P_2$ is below the ray, `jmp = 12`. The LUT is implemented as follows:
```
let class = 0x2E74 >> jmp;
```
Yes -- it's that simple.
The table is organized such that after shifting the binary number $\color{orange}10 \color{blue}11 \color{orange}10 \color{blue}01 \color{orange}11 \color{blue}01 \color{orange}00$,
the _lowest two bits_ tells us how to interpret our two solutions $t_1$ and $t_2$.
In other words, it defines _equivalence classes_ of Bézier curves based on the layout of their 
control points, and a Bézier curves equivalence class determines if a solution should count
towards the winding number.

![Victory!](report/victory.png)

Now, armed with an algorithm that doesn't suck, we are ready to take on the GPU.

# Hardware-accellerated rasterization
The plan is as follows: Let a "text-box" be represented by a single quad,
and put all the Bézier curve control points in a uniform buffer.
We bake two unsigned integer-attributes into the vertex defning the range of
Bézier curves that is part of text in its box to restrict the loop in the fragment shader.
I'm not _exactly_ sure how the GPU executes such code;
since it is a SIMT architecture, at the very least all the threads will be waiting
for the longest loop. I'm not sure how the memory access works when all the threads
have a different loop counter and is accessing different parts of the buffer,
if different threads in the same block can cache different parts of the uniform buffer and so on.
There might not be much benefit to such optimization, I'm choosing to remain optimistic however.

The main challenge is the volume of uniforms: For a large box of text, there might
be _tens of thousands_ of curves. A single $\alpha$ has 41 quadratic bezier curves,
and a single curve has 3 points $\times$ 2 coordinates $\times$ 4 bytes $=24$ bytes.
Some random website said that a page of text generally contains $\sim 3000$ characters,
in other words a pages worth of text could contain $3000*41*24$ bytes, approximately
3 _megabytes_ of curves.
We could of course split our text into multiple draw-calls, but that doesn't really
_solve_ the problem: All the data still needs to be fed to the GPU, and that just
introduces more costly state transitions and overhead. Almost certainly, sending
that much data would be faster to do in one go. Uniform buffer objects can only
store (at least as per defined by the spec) 16KB.
Needless to say, we need some other strategy for handing off data to the GPU.
And on a similar note, a lot of the time the data will not actually change from
drawcall to drawcall: Translating text around the screen will of course be done
by applying affine transformations to the text-box quad, never actually touching
the curves, we only need to hit the curve buffer if the text itself changes,
and this would be a significant CPU time-save.
So how do we do achieve both support for _large_ buffers, and persistance across
pipeline invocations?

SSBOs -- Shader Storage Buffer Objects -- seem very appealing, these buffers
provide at least 128MB (as per the spec, in reality usually arbitrarily large),
and can also be dynamically sized; a single buffer oject doesnt have a (compile-time)
static size, like uniform buffers do.
This is very cool, and is core since OpenGL 4.3, which means it is fair game
for this project as it's meant to use modern OpenGL. But, this means our program can no longer work on MacOS.
Sucks to suck i guess.
