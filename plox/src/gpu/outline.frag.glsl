#version 440

in vec2 uv;
layout(location = 0) out float color;

void main() {
    if (uv.s*uv.s - uv.t < 0) {
        // inside
        color = 1.0;
    } else {
        color = 0.0;
    }
}
