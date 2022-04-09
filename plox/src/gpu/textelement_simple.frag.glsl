#version 440

in vec2 uv;
out vec4 color;

uniform sampler2DMS tex;
uniform ivec2 tex_dims;

void main() {
    int x = int(tex_dims.x * clamp(uv.x, 0, 1));
    int y = int(tex_dims.y * clamp(uv.y, 0, 1));

    float alpha = 0.0;
    int N = 8;

    for (int i = 0; i < N; i++) {
        float sample_alpha = texelFetch(tex, ivec2(x, y), i).r;
        alpha += sample_alpha;
    }

    alpha /= float(N);

    color = vec4(0, 0, 0, clamp(alpha, 0.0, 1.0));
}
