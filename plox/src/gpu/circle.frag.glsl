#version 440

out vec4 color;
in  vec2 uv;

uniform float width;
uniform float radius;
uniform vec2  arc;

void main() {
    const vec4 outline_color = vec4(0,0,0,1);

    vec2  r   = (width + radius) * (2.0 * uv - 1.0);
    float phi = atan(r.y, r.x);

    if (arc.x < phi && phi < arc.y) {
        float R  = abs(length(r) - radius) - width;
        float dR = fwidth(R);
        float alpha = 1.0 - (R + dR) / dR;
        color = vec4(outline_color.rgb, alpha*outline_color.a);
    }
}
