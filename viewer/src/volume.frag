#version 300 es
precision highp float;
precision highp sampler3D;
in vec2 uv;
out vec4 outputColor;
uniform sampler3D data;
uniform vec2 camera,range;
uniform vec3 boxExtent;
uniform float density,cut,aspect;
uniform int colorCount;
uniform bool shellMode;
uniform vec2 shellRadii;
uniform vec3 colorStops[32];
float sampleVolume(vec3 p){ivec3 n=textureSize(data,0);vec3 q=p*vec3(n)-.5;ivec3 a=ivec3(floor(q));vec3 w=fract(q);float v=0.;for(int k=0;k<2;k++)for(int j=0;j<2;j++)for(int i=0;i<2;i++){ivec3 o=ivec3(i,j,k);vec3 weight=mix(1.-w,w,vec3(o));v+=weight.x*weight.y*weight.z*texelFetch(data,clamp(a+o,ivec3(0),n-1),0).r;}return v;}
mat3 rotation(){float a=camera.x,b=camera.y;mat3 ry=mat3(cos(a),0.,-sin(a),0.,1.,0.,sin(a),0.,cos(a));mat3 rx=mat3(1.,0.,0.,0.,cos(b),sin(b),0.,-sin(b),cos(b));return ry*rx;}
vec3 cmap(float t){if(colorCount>=2){float q=clamp(t,0.,1.)*float(colorCount-1);int i=min(int(floor(q)),colorCount-2);return mix(colorStops[i],colorStops[i+1],q-float(i));}vec3 a=vec3(.15,.28,.45),b=vec3(.77,.39,.28),c=vec3(1.,.85,.54);return t<.5?mix(a,b,2.*t):mix(b,c,2.*t-1.);}
void main(){mat3 r=rotation();vec2 xy=(uv-.5)*vec2(aspect,1.)*1.45;vec3 origin=r*vec3(xy,2.2)+.5;vec3 direction=r*vec3(0.,0.,-1.);vec3 inv=1./direction;vec3 low=.5-.5*boxExtent,high=.5+.5*boxExtent;vec3 t0=(low-origin)*inv,t1=(high-origin)*inv;vec3 nearV=min(t0,t1),farV=max(t0,t1);float nearT=max(max(nearV.x,nearV.y),nearV.z),farT=min(min(farV.x,farV.y),farV.z);if(farT<=nearT){outputColor=vec4(1);return;}float ds=(farT-nearT)/160.;vec3 rgb=vec3(0);float alpha=0.;for(int i=0;i<160;i++){vec3 p=(origin+direction*(nearT+(float(i)+.5)*ds)-low)/boxExtent;vec3 q=p-.5;
if(shellMode){
 float start=nearT+float(i)*ds;
 for(int axis=0;axis<2;axis++){
  int k=axis==0?0:2;
  if(abs(direction[k])<1e-8)continue;
  float hit=(.5-origin[k])/direction[k];
  if(hit<start || hit>=start+ds)continue;
  vec3 cutPoint=origin+direction*hit;vec3 cq=cutPoint-.5;
  if(cq[axis==0?2:0]<=0. || length(cq)<=shellRadii.x || length(cq)>=shellRadii.y)continue;
  float value=sampleVolume(cutPoint);float t=clamp((value-range.x)/(range.y-range.x),0.,1.);
  float a=.98;rgb+=(1.-alpha)*a*cmap(t);alpha+=(1.-alpha)*a;
 }
 if(alpha>.995)break;
}

if(shellMode){
 float radius=length(q);
 if(radius<shellRadii.x){
  vec3 normal=normalize(q);float light=.35+.65*max(dot(normal,normalize(vec3(.5,.8,1.))),0.);
  float lat=asin(normal.y),lon=atan(normal.z,normal.x);
  float grid=max(step(.975,cos(lat*12.)),step(.992,cos(lon*12.)));
  vec3 earth=mix(vec3(.20,.40,.53),vec3(.60,.72,.78),grid)*light;
  rgb+=(1.-alpha)*earth;alpha=1.;break;
 }
 if(radius>shellRadii.y || (q.x>0. && q.z>0.))continue;
}
float value=sampleVolume(p);float t=clamp((value-range.x)/(range.y-range.x),0.,1.);float a=1.-exp(-density*max(t-cut,0.)*ds*5.);
rgb+=(1.-alpha)*a*cmap(t);alpha+=(1.-alpha)*a;if(alpha>.995)break;}outputColor=vec4(rgb+(1.-alpha)*vec3(1),1);}
