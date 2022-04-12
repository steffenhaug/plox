#version 430

in  vec2 uv;
out vec4 color;

uniform float width;

#define LIM 50

int mandelbrot() {
  /* Normalize coordinates to obtain c. */
  float c_re = 2*uv.x - 1.5;
  float c_im = 2*uv.y - 1;

  /* Iteration starts at 0. */
  float re = 0.0;
  float im = 0.0;

  int N;
  for (N = 0; N < LIM; N++) {
    float tmp = re;
    re = (re * re - im * im) + c_re;
    im = 2.0 * tmp * im + c_im;

    /* D = squared distance */
    float D = re * re + im * im;
    if (D > 4) {
      break;
    }

  }

  return N;
}

void main() {
    float W = abs(width * (2*uv.y - 1)) - width;
    float dW = fwidth(W);
    float alpha = 1 - clamp((W + dW) / dW, 0, 1);

    float L = abs(uv.x - 0.5) - 0.5;
    float dL = fwidth(L);
    float beta = 1 - clamp((L + dL) / dL, 0, 1);

    vec3 C1 = vec3(1.0, 0.0, 1.0);
    vec3 C2 = vec3(0.0, 1.0, 1.0);

    int N = mandelbrot();
    float t = N == LIM ? 0 : (float(N) / float(LIM));

    color = vec4(t*mix(C1, C2, t), alpha*beta);
}
