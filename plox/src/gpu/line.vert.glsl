#version 430

in  vec2 position;
in  vec2 uv_in;
out vec2 uv;

uniform mat4 mvp;

void main() {
    gl_Position = mvp * vec4(position, 0.0, 1.0);
    uv = uv_in;
}
