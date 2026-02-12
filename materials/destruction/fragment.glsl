#version 100
precision lowp float;

varying vec2 uv;
varying vec4 color;

uniform sampler2D Texture;
uniform sampler2D Mask;

void main() {
    vec4 res = texture2D(Texture, uv);
    vec4 mask = texture2D(Mask, uv);

    // Discard if the mask is dark OR if the original sprite was transparent
    if (mask.r < 0.5 || res.a < 0.1) {
        discard;
    }

    gl_FragColor = res * color;
}