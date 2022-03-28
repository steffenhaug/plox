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

int winding_number(vec2 uv) {
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

        // Calculate x1=B_x(sol.s) and x2=B_x(sol.t)
        float sol_x1 = at(i, sol.s).x;
        float sol_x2 = at(i, sol.t).x;

        // Add the solutions contributions to the winding number if applicable.
        if ((k & 1) != 0 && (sol_x1 > uv.x)) {
            w++;
        }

        if ((k & 2) != 0 && (sol_x2 > uv.x)) {
            w--;
        }
    }

    return w;
}

// I found a bug in the Nvidia driver (510.47.03):
// The shader compiler doesn't understand that reading `curves.length()`
// should by itself disqualify `curves` from being optimized away. Even
// setting `color = vec4(float(curves.length()), ...)`, where the length
// _directly_ influences the color doesn't work unless _some_ element of
// the buffer have some relation to an `out` symbol.
out float hack;
#define dont_remove_ssbo     \
    do {                     \
        hack = curves[0].x0; \
    } while(false);

void main() {
    dont_remove_ssbo;

    color = vec4(1.0, uv.x/1000.0, uv.y/1000.0, 1.0);
    int w = winding_number(uv);
    if (w != 0) {
        color = vec4(float(w), 0.0, 0.0, 1.0);
    }
}
