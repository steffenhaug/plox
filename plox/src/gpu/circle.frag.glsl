#version 440

#define PI 3.1415926538

out vec4 color;
in  vec2 uv;

uniform float width;
uniform float radius;
uniform vec2  arc;

void main() {
    const vec4 outline_color = vec4(0,0,0,1);

    // Polar cordinates (|r|, φ)
    vec2 r = (width + radius) * (2.0 * uv - 1.0);

    // Map [-π, π] -> [0, 2π]
    float phi = mod((atan(r.y, r.x) + 2*PI), 2*PI);

    // Compute the mask in the r-direction.
    float R  = abs(length(r) - radius) - width;
    float dR = fwidth(R);
    float alpha = 1 - clamp((R + dR) / dR, 0, 1);

    // Compute the mask in the φ-direction.
    float psi = (arc.x + arc.y) / 2.0;
    float A = abs(phi - psi) - (arc.y - psi);
    float dA = fwidth(A);
    float beta = 1 - clamp((A + dA) / dA, 0, 1);
    
    // Draw the circle arc.
    color = vec4(outline_color.rgb, alpha*beta*outline_color.a);
}
