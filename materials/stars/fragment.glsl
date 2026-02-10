#version 100
precision lowp float;

varying vec2 uv;
uniform float Time;
uniform vec2 Resolution;

#define NS 500.

float L(vec2 uv, vec2 ofs, float b, float l) {
    return smoothstep(0., 1000., b*max(0.1, l)/pow(max(0.0000000000001, length(uv-ofs)), 1./max(0.1, l)));
}

float rand(vec2 co, float s){
    float PHI = 1.61803398874989484820459;
    return fract(tan(distance(co*PHI, co)*s)*co.x);
}

vec2 H12(float s) {
    float x = rand(vec2(243.234,63.834), s)-.5;
    float y = rand(vec2(53.1434,13.1234), s)-.5;
    return vec2(x, y);
}

void main() {
    // Standardize coordinates like Shadertoy
    vec2 uv = gl_FragCoord.xy / Resolution.xy;
    uv -= .5;
    uv.x *= Resolution.x / Resolution.y;

    vec4 col = vec4(.0);
    vec4 b = vec4(0.01176470588, 0.05098039215, 0.14117647058, 1.);
    vec4 p = vec4(0.13333333333, 0.07843137254, 0.13725490196, 1.);
    vec4 lb = vec4(0.10196078431, 0.21568627451, 0.33333333333, 1.);
    
    vec4 blb = mix(b, lb, -uv.x*.2-(uv.y*.5));
    col += mix(blb, p, uv.x-(uv.y*1.5));
    
    for(float i=0.; i < NS; i++) {
        vec2 ofs = H12(i+1.);
        ofs *= vec2(1.8, 1.1);
        float r = (mod(i, 20.) == 0.)? 0.25+abs(sin(i/50.)): 0.25;
        // iTime replaced with Time
        col += vec4(L(uv, ofs, r+(sin(fract(Time)*.1*i)+1.)*0.02, 1.));
    }

    gl_FragColor = col;
}