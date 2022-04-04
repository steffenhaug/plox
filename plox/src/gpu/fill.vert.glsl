#version 440

in vec2 position;

uniform mat4 mvp;

/// When drawing the characters mask, we simply replace the
/// middle control point with an aritrary point, for example
/// the origin. Then stencil buffer flipping will handle the
/// rest.
void main() {
    float delete_midpoint = abs(gl_VertexID % 3 - 1);
    gl_Position = mvp * delete_midpoint * vec4(position, 0.0, 1.0);
}
