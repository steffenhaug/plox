#version 440

#define PI 3.1415926538

out vec4 color;
in  vec2 uv;

uniform float width;
uniform float radius;
uniform vec2  arc;

void main() {
    vec3 C1 = vec3(0.9, 0.0, 0.9);
    vec3 C2 = vec3(0.0, 0.9, 0.9);
    const vec4 outline_color = vec4(mix(C1, C2, uv.y), 0.5);

    // Polar cordinates (len(r), φ)
    vec2  r   = (width + radius) * (2.0 * uv - 1.0);
    // Map [-π, π] -> [0, 2π]
    float phi = mod((atan(r.y, r.x) + 2*PI), 2*PI);

    // Compute the mask in the r-direction.
    float R  = abs(length(r) - radius) - width;
    float dR = fwidth(R);
    float alpha = 1 - clamp((R + dR) / dR, 0, 1);

    // Angular mask
    float phi_max = arc.y;
    float phi_min = arc.x;
    if (phi_min < phi && phi < phi_max) {
    color = vec4(outline_color.rgb, alpha*outline_color.a);
    }

}
