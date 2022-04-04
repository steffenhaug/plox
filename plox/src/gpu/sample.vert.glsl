#version 440

out vec2 uv;

uniform vec2 coverage;
uniform vec4 bbox;
uniform mat4 mvp;

void main() {
    const vec2 vertex_buffer[6] = vec2[](
        vec2(bbox.x, bbox.y), /* 1st triangle */
        vec2(bbox.z, bbox.y),
        vec2(bbox.z, bbox.w),
        vec2(bbox.x, bbox.y), /* 2nd triangle */
        vec2(bbox.z, bbox.w),
        vec2(bbox.x, bbox.w)
    );

    const vec2 uv_buffer[6] = vec2[](
        vec2(         0,          0), /* 1st triangle */
        vec2(coverage.x,          0),
        vec2(coverage.x, coverage.y),
        vec2(         0,          0), /* 2nd triangle */
        vec2(coverage.x, coverage.y),
        vec2(         0, coverage.y)
    );
    
    gl_Position = mvp * vec4(vertex_buffer[gl_VertexID], 0.0, 1.0);
    uv = uv_buffer[gl_VertexID];
}
