#version 430

in vec2 position;
in vec2 uv;

uniform mat4 model;
uniform mat4 proj;

out PASS_TO_FRAGMENT_SHADER
{
    vec2 uv;
    float du;
    float dv;
} fragment;

void main() {
    mat4 mvp = proj * model;
    gl_Position = mvp * vec4(position, 0.0, 1.0);

    // Pass on information to the fragment shader.
    fragment.uv = uv;
    mat3 M3x3 = mat3(model);
    fragment.du = 1.0 / (2.0 * length(M3x3 * vec3(1.0, 0.0, 0.0)));
    fragment.dv = 1.0 / (2.0 * length(M3x3 * vec3(0.0, 1.0, 0.0)));
}

