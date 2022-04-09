#version 440

in vec2 position;

uniform mat4 mvp;

/// When drawing the characters mask, we simply replace the
/// middle control point with an aritrary point, for example
/// the origin. Then stencil buffer flipping will handle the
/// rest.
void main() {
    // This sends the middle control point (index mod 3 = 1) to the origin.
    float mask = abs(gl_VertexID % 3 - 1);
    gl_Position = mvp * vec4(mask * position, 0.0, 1.0);
}
