#version 300 es
precision highp float;
in vec2 uv;
out vec4 outputColor;
uniform sampler2D image;
uniform vec2 camera;
uniform float aspect;
void main(){vec2 xy=(uv-.5)*vec2(aspect,1.)*2.25;float d=dot(xy,xy);if(d>1.){outputColor=vec4(1.);return;}
float c=cos(camera.x),s=sin(camera.x),cp=cos(camera.y),sp=sin(camera.y);vec3 right=vec3(s,c,0.),up=vec3(-c*sp,s*sp,cp),front=vec3(c*cp,-s*cp,sp);vec3 q=xy.x*right+xy.y*up+sqrt(1.-d)*front;
float lon=atan(q.y,q.x),lat=asin(q.z);vec4 value=texture(image,vec2(lon/6.28318530718+.5,lat/3.14159265359+.5));
float light=.5+.5*max(dot(q,normalize(vec3(1.,-.5,.6))),0.);vec3 base=vec3(.17,.32,.41)*light;
float line=min(abs(sin(lon*6.)),abs(sin(lat*6.)));float grid=1.-smoothstep(0.,.018,line);base=mix(base,vec3(.55,.64,.69),.22*grid);
outputColor=vec4(mix(base,value.rgb,value.a),1.);}
