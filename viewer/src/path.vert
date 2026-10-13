#version 300 es
precision highp float;
in vec3 position;
in vec3 rgb;
uniform vec2 camera;
uniform vec3 centre;
uniform float extent,aspect;
out vec3 color;
void main(){float c=cos(camera.x),s=sin(camera.x),cp=cos(camera.y),sp=sin(camera.y);vec3 q=position-centre;vec3 right=vec3(s,c,0.),up=vec3(-c*sp,s*sp,cp),front=vec3(c*cp,-s*cp,sp);gl_Position=vec4(dot(q,right)/(extent*aspect),dot(q,up)/extent,-dot(q,front)/200.,1.);color=rgb;}
