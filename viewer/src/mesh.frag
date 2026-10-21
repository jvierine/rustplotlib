#version 300 es
precision highp float;
in vec3 worldNormal;
in vec3 color;
in float scalar;
uniform sampler2D colormap;
uniform vec2 range;
uniform float colorCount;
uniform int lineMode;
out vec4 outputColor;
void main() {
    if(lineMode==1){outputColor=vec4(color,1.);return;}
    float t=clamp((scalar-range.x)/(range.y-range.x),0.,1.);
    vec3 c=scalar < -1e20 ? color : texture(colormap,vec2((.5+t*(colorCount-1.))/colorCount,.5)).rgb;
    vec3 n=normalize(worldNormal);
    float light=.68+.32*abs(dot(n,normalize(vec3(.4,-.6,1.))));
    outputColor=vec4(c*light,1.);
}
