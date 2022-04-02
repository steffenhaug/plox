#version 440

in vec2 uv;
out vec4 color;

uniform sampler2D tex;

void main() {
    float alpha = texture(tex, uv).r;
    color = vec4(1.0, 0.3, 0.0, alpha);
}
