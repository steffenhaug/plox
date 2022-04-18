#version 440

in vec2 uv;
out vec4 color;

uniform sampler2D tex;
uniform ivec2 tex_dims;

void main() {
    int x = 4 * int(tex_dims.x * uv.x);
    int y = 4 * int(tex_dims.y * uv.y);

    float alpha = 0.0;
    for (int i = 0; i < 4; i++) {
        for (int j = 0; j < 4; j++) {
            alpha += texelFetch(tex, ivec2(x + i, y + j), 0).r;
        }
    }
    alpha /= 16.0;

    color = vec4(0, 0, 0, clamp(alpha, 0.1, 1.0));
}
