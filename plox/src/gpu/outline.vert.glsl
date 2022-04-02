#version 440

in vec2 position;
out vec2 uv;

uniform mat4 p;
uniform mat4 m;

void main() {
    mat4 mvp = p * m;
    gl_Position = mvp * vec4(position, 0.0, 1.0);

    switch (gl_VertexID % 3) {
    case 0:
        uv = vec2(0.0, 0.0);
        break;
    case 1:
        uv = vec2(0.5, 0.0);
        break;
    case 2:
        uv = vec2(1.0, 1.0);
        break;
    }
}
