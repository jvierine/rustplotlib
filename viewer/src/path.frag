#version 300 es
precision highp float;
in vec3 color;
out vec4 outputColor;
void main(){outputColor=vec4(color,1.);}
