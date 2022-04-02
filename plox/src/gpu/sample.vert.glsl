#version 440

out vec2 uv;

// Positions (in units of 1em) of the bottom-left and top-right corner.
uniform vec2 bl;
uniform vec2 tr;
uniform vec2 max_uv;
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

    float max_u = max_uv.x;
    float max_v = max_uv.y;
    const vec2 uv_buffer[6] = vec2[](
        vec2(    0,     0), /* 1st triangle */
        vec2(max_u,     0),
        vec2(max_u, max_v),
        vec2(    0,     0), /* 2nd triangle */
        vec2(max_u, max_v),
        vec2(    0, max_v)
    );
    
    mat4 mvp = p * m;
    gl_Position = mvp * vec4(vertex_buffer[gl_VertexID], 0.0, 1.0);
    uv = uv_buffer[gl_VertexID];
}
