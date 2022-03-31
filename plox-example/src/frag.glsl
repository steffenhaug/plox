#version 440

out vec4 color;

in PASS_TO_FRAGMENT_SHADER
{
    vec2  uv;
    float du;
    float dv;
    flat uint glyph;
};


/// A quadratic Bézier curve.
struct Quadratic {
    float x0; /* Control point #1 */
    float y0;
    float x1; /* Control point #2 */
    float y1;
    float x2; /* Control point #3 */
    float y2;
};

readonly layout(std430, binding = 0) buffer beziers
{
    Quadratic curves[];
};

/// A Glyph is just a range in the Bézier curve array.
struct Glyph {
    uint start;
    uint end;
};

readonly layout(std430, binding = 1) buffer lut
{
    Glyph atlas[];
};

bool approx(float x, float y) {
    return abs(x - y) < 1.0e-6;
}

vec2 at(int i, float t) {
    vec2 q1 = mix(vec2(curves[i].x0, curves[i].y0),
                  vec2(curves[i].x1, curves[i].y1), t);

    vec2 q2 = mix(vec2(curves[i].x1, curves[i].y1),
                  vec2(curves[i].x2, curves[i].y2), t);

    return mix(q1, q2, t);
}

vec2 solve(float c, float b, float a) {
    // Check if we need a linear approximation.
    if (approx(0.0, a)) {
        // Divide by zero is actually okay; the equivalence classes
        // will make sure invalid solutions are not used. Divide by
        // zero here corresponds to horizontal lines, and those are
        // simply defined to not cause intersections.
        return vec2(-c/b, -c/b);
    }

    float  delta = b*b - 4.0*a*c;

    return vec2(
        (-b + sqrt(delta)) / (2.0 * a),
        (-b - sqrt(delta)) / (2.0 * a)
    );
}

int wn(vec2 uv, uint start, uint end) {
    int w = 0;
    
    for (uint i = start; i < end; i++) {
        // Get the control points for the Bézier curve with shifted y.
        float y0 = curves[i].y0 - uv.y;
        float y1 = curves[i].y1 - uv.y;
        float y2 = curves[i].y2 - uv.y;
        float x0 = curves[i].x0;
        float x1 = curves[i].x1;
        float x2 = curves[i].x2;

        // Calculate the lookup shift.
        int jmp = ((y0 > 0.0) ? 8 : 0)
                + ((y1 > 0.0) ? 4 : 0)
                + ((y2 > 0.0) ? 2 : 0);

        // Calculate the Bézier curves equivalence class.
        int k = 0x2E74 >> jmp;

        // Calculate coefficients for B_y(t).
        float c =  1.0 * y0;
        float b = -2.0 * y0 + 2.0 * y1;
        float a =  1.0 * y0 - 2.0 * y1 + 1.0 * y2;

        // Solve B_y(t) = 0.
        vec2 sol = solve(c, b, a);

        // Calculate coefficients for B_x(t).
        c =  1.0 * x0;
        b = -2.0 * x0 + 2.0 * x1;
        a =  1.0 * x0 - 2.0 * x1 + 1.0 * x2;

        // Calculate x1=B_x(sol.s) and x2=B_x(sol.t)
        x1 = c + b*sol.s + a*(sol.s*sol.s);
        x2 = c + b*sol.t + a*(sol.t*sol.t);

        // Add the solutions contributions to the winding number if applicable.

        if ((k & 1) != 0 && x1 > uv.x) {
            w++;
        }

        if ((k & 2) != 0 && x2 > uv.x) {
            w--;
        }
    }

    return w;
}

/// Sample the text with 16 uniformly spaced samples.
float sample_MSAAx16(vec2 uv, uint start, uint end) {
    mat4 W = mat4(
        wn(uv + vec2(du * -0.75, dv * -0.75), start, end),
        wn(uv + vec2(du * -0.75, dv * -0.25), start, end),
        wn(uv + vec2(du * -0.75, dv *  0.25), start, end),
        wn(uv + vec2(du * -0.75, dv *  0.75), start, end),
        wn(uv + vec2(du * -0.25, dv * -0.75), start, end),
        wn(uv + vec2(du * -0.25, dv * -0.25), start, end),
        wn(uv + vec2(du * -0.25, dv *  0.25), start, end),
        wn(uv + vec2(du * -0.25, dv *  0.75), start, end),
        wn(uv + vec2(du *  0.25, dv * -0.75), start, end),
        wn(uv + vec2(du *  0.25, dv * -0.25), start, end),
        wn(uv + vec2(du *  0.25, dv *  0.25), start, end),
        wn(uv + vec2(du *  0.25, dv *  0.75), start, end),
        wn(uv + vec2(du *  0.75, dv * -0.75), start, end),
        wn(uv + vec2(du *  0.75, dv * -0.25), start, end),
        wn(uv + vec2(du *  0.75, dv *  0.25), start, end),
        wn(uv + vec2(du *  0.75, dv *  0.75), start, end)
    );

    float alpha = (W[0][0] != 0 ? 1.0/16.0 : 0.0)
                + (W[0][1] != 0 ? 1.0/16.0 : 0.0)
                + (W[0][2] != 0 ? 1.0/16.0 : 0.0)
                + (W[0][3] != 0 ? 1.0/16.0 : 0.0)
                + (W[1][0] != 0 ? 1.0/16.0 : 0.0)
                + (W[1][1] != 0 ? 1.0/16.0 : 0.0)
                + (W[1][2] != 0 ? 1.0/16.0 : 0.0)
                + (W[1][3] != 0 ? 1.0/16.0 : 0.0)
                + (W[2][0] != 0 ? 1.0/16.0 : 0.0)
                + (W[2][1] != 0 ? 1.0/16.0 : 0.0)
                + (W[2][2] != 0 ? 1.0/16.0 : 0.0)
                + (W[2][3] != 0 ? 1.0/16.0 : 0.0)
                + (W[3][0] != 0 ? 1.0/16.0 : 0.0)
                + (W[3][1] != 0 ? 1.0/16.0 : 0.0)
                + (W[3][2] != 0 ? 1.0/16.0 : 0.0)
                + (W[3][3] != 0 ? 1.0/16.0 : 0.0);

    return alpha;
}

/// Sample the text with a single sample. Useful for debugging.
float sample_single(vec2 uv, uint start, uint end) {
    return wn(uv, start, end) != 0.0 ? 1.0 : 0.0;
}

void main() {
    uint start = atlas[glyph].start;
    uint end = atlas[glyph].end;
    float alpha = sample_single(uv, start, end);
    color = vec4(0.0, 0.0, 0.0, alpha);
}
