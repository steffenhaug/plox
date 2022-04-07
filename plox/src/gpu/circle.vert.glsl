#version 440

out vec2 uv;

uniform float width;
uniform float radius;
uniform mat4 mvp;

void main() {
    const float R = radius + width;

    const vec2 vertex_buffer[6] = vec2[](
        vec2(-R, -R), /* 1st triangle */
        vec2( R, -R),
        vec2( R,  R),
        vec2(-R, -R), /* 2nd triangle */
        vec2( R,  R),
        vec2(-R,  R)
    );

    const vec2 uv_buffer[6] = vec2[](
        vec2(0, 0), /* 1st triangle */
        vec2(1, 0),
        vec2(1, 1),
        vec2(0, 0), /* 2nd triangle */
        vec2(1, 1),
        vec2(0, 1)
    );
    
    gl_Position = mvp * vec4(vertex_buffer[gl_VertexID], 0.0, 1.0);
    uv = uv_buffer[gl_VertexID];
}
