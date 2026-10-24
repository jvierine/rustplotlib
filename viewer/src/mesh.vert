#version 300 es
precision highp float;
layout(location=0) in vec3 position;
layout(location=1) in vec3 normal;
layout(location=2) in float value;
layout(location=3) in vec3 rgb;
uniform vec3 centre,axisScale;
uniform float radius,aspect;
uniform vec3 camera;
out vec3 worldNormal;
out vec3 color;
out float scalar;
void main() {
    float a=camera.x,b=camera.y,d=camera.z;
    vec3 front=vec3(cos(a)*cos(b),sin(a)*cos(b),sin(b));
    vec3 right=vec3(-sin(a),cos(a),0.);
    vec3 up=cross(front,right);
    vec3 q=(position-centre)*axisScale/radius;
    float z=d-dot(q,front);
    float f=2.41421356237;
    gl_Position=vec4(dot(q,right)*f/aspect,dot(q,up)*f,
                     (20.1/19.9)*z-(4./19.9),z);
    worldNormal=normal/axisScale;
    scalar=value;
    color=rgb;
}
