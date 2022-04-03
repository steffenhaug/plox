#version 440

out vec2 uv;

uniform vec2 coverage;
uniform vec2 bl;
uniform vec2 tr;
uniform mat4 p;
uniform mat4 m;

void main() {
    const vec2 vertex_buffer[6] = vec2[](
        vec2(bl.x, bl.y), /* 1st triangle */
        vec2(tr.x, bl.y),
        vec2(tr.x, tr.y),
        vec2(bl.x, bl.y), /* 2nd triangle */
        vec2(tr.x, tr.y),
        vec2(bl.x, tr.y)
    );

    const vec2 uv_buffer[6] = vec2[](
        vec2(         0,          0), /* 1st triangle */
        vec2(coverage.x,          0),
        vec2(coverage.x, coverage.y),
        vec2(         0,          0), /* 2nd triangle */
        vec2(coverage.x, coverage.y),
        vec2(         0, coverage.y)
    );
    
    mat4 mvp = p * m;
    gl_Position = mvp * vec4(vertex_buffer[gl_VertexID], 0.0, 1.0);
    uv = uv_buffer[gl_VertexID];
}
