#version 440

#define PI 3.1415926538

out vec4 color;
in  vec2 uv;

uniform float width;
uniform float radius;
uniform vec2  arc;

void main() {
    const vec4 outline_color = vec4(0,0,0,1);

    // Polar cordinates (len(r), φ)
    vec2  r   = (width + radius) * (2.0 * uv - 1.0);
    // Map [-π, π] -> [0, 2π]
    float phi = mod((atan(r.y, r.x) + 2*PI), 2*PI);

    // Compute the mask in the r-direction.
    float R  = abs(length(r) - radius) - width;
    float dR = fwidth(R);
    float alpha = 1 - clamp((R + dR) / dR, 0, 1);

    // Angular mask
    float phi_max = mod(arc.y, 2*PI);
    float phi_min = mod(arc.x, 2*PI);
    float beta =      step(phi_min, phi)
               * (1 - step(phi_max, phi));

    color = vec4(outline_color.rgb, beta*alpha*outline_color.a);
}
