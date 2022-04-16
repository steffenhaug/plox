#version 440

#define PI 3.1415926538

out vec4 color;
in  vec2 uv;

uniform float width;
uniform float radius;
uniform vec2  arc;

vec4 blend(vec4 src, vec4 dst) {
    float alpha_0 = dst.a + src.a * (1 - dst.a);
    vec3  color_0 = dst.rgb*dst.a + src.rgb*src.a*(1-dst.a);
    return vec4((1.0 / alpha_0) * color_0, alpha_0);
}


void main() {
    // Compute some pretty colors.
    vec3 C1 = vec3(0.9, 0.0, 0.9);
    vec3 C2 = vec3(0.0, 0.9, 0.9);
    const vec4 outline_color = vec4(mix(C1, C2, 1-length(uv)), 1.0);
    const vec4 fill_color = vec4(mix(C1, C2, length(uv)), 1/1.6);

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
    vec4 o_color = vec4(outline_color.rgb, alpha*beta*outline_color.a);
    vec4 f_color = length(r) < radius ? fill_color : vec4(0,0,0,0);

    color = blend(f_color, o_color);
}
