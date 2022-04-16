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

    vec3 C1 = vec3(0.9, 0.0, 0.9);
    vec3 C2 = vec3(0.0, 0.9, 0.9);
    color = vec4(mix(C1, C2, uv.x), alpha*beta);
}
