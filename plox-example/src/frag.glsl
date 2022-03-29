#version 430

out vec4 color;

in PASS_TO_FRAGMENT_SHADER
{
    vec2 uv;
};


struct Quadratic {
    float x0; // Control point #1
    float y0;
    float x1; // Control point #2
    float y1;
    float x2; // Control point #3
    float y2;
};

uniform float spacing;

layout(std430, binding = 0) buffer beziers
{
    Quadratic curves[];
};

bool approx(float x, float y) {
    return abs(x - y) < 1.0e-3;
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
        // will make sure invalid solutions remain unused.
        return vec2(-c/b, -c/b);
    }

    // Δ = 0 => one root (with multiplicity two)
    // Δ > 0 => two distinct roots
    // Δ < 0 => imaginary roots (will be NaN, but never used)
    float  delta = b*b - 4.0*a*c;

    return vec2(
        (-b + sqrt(delta)) / (2.0 * a),
        (-b - sqrt(delta)) / (2.0 * a)
    );
}

int wn(vec2 uv) {
    int w = 0;

    for (int i = 0; i < curves.length(); i++) {
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

        if ((k & 1) != 0 && (x1 > uv.x)) {
            w++;
        }

        if ((k & 2) != 0 && (x2 > uv.x)) {
            w--;
        }
    }

    return w;
}

float MSAAx4(vec2 uv) {
    // Gets the alpha channel after antialiasing. This allows you to use it with
    // a custom color, and then blend it onto a background color.

    // We use a slightly rotated set of sample points relative to the pixel grid.
    float space = 1000.0 / 400.0; // todo : get this from uniform
    vec2 a = space * vec2(-0.35, -0.10);
    vec2 b = space * vec2( 0.10, -0.35);
    vec2 c = space * vec2( 0.35,  0.10);
    vec2 d = space * vec2( 0.10,  0.35);

    mat2 W = mat2(
        wn(uv + a), wn(uv + b),
        wn(uv + c), wn(uv + d)
    );

    float alpha = (W[0][0] != 0 ? 0.25 : 0.0)
                + (W[0][1] != 0 ? 0.25 : 0.0)
                + (W[1][0] != 0 ? 0.25 : 0.0)
                + (W[1][1] != 0 ? 0.25 : 0.0);

    return alpha;
}

float MSAAx16(vec2 uv) {
    float space = 1000.0 / 400.0;
    mat4 W = mat4(
        wn(uv + vec2(-0.75, -0.75)),
        wn(uv + vec2(-0.75, -0.25)),
        wn(uv + vec2(-0.75,  0.25)),
        wn(uv + vec2(-0.75,  0.75)),
        wn(uv + vec2(-0.25, -0.75)),
        wn(uv + vec2(-0.25, -0.25)),
        wn(uv + vec2(-0.25,  0.25)),
        wn(uv + vec2(-0.25,  0.75)),
        wn(uv + vec2( 0.25, -0.75)),
        wn(uv + vec2( 0.25, -0.25)),
        wn(uv + vec2( 0.25,  0.25)),
        wn(uv + vec2( 0.25,  0.75)),
        wn(uv + vec2( 0.75, -0.75)),
        wn(uv + vec2( 0.75, -0.25)),
        wn(uv + vec2( 0.75,  0.25)),
        wn(uv + vec2( 0.75,  0.75))
    );

    float alpha = 0;
    for (int i =  0; i < 4; i++) {
        for (int j =  0; j < 4; j++) {
            if (W[i][j] != 0) {
                alpha += 1.0/16.0;
            }
        }
    }

    return alpha;
}

float SS(vec2 uv) {
    // Get the single-sample alpha.
    return wn(uv) != 0.0 ? 1.0 : 0.0;
}





void main() {
    // Compare MSAA and SS
    float border = 60.0;
    float ms = MSAAx16(uv);
    color = vec4(1.0-ms, 1.0-ms, 1.0-ms, 1.0);
}
