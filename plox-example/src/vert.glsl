#version 430

in vec2 position;
in vec2 uv;

out PASS_TO_FRAGMENT_SHADER
{
    vec2 uv;
} fragment;

void main() {
    fragment.uv = uv;
    gl_Position = vec4(position, 0.0, 1.0);
}

