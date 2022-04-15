#version 430

in  vec2 uv;
out vec4 color;

uniform float width;

void main() {
    float W = abs(width * (2*uv.y - 1)) - width;
    float dW = fwidth(W);
    float alpha = 1 - clamp((W + dW) / dW, 0, 1);

    float L = abs(uv.x - 0.5) - 0.5;
    float dL = fwidth(L);
    float beta = 1 - clamp((L + dL) / dL, 0, 1);

    color = vec4(0, 0, 0, alpha*beta);
}
