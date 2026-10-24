#version 300 es
precision highp float;
precision highp sampler2D;
in vec2 uv;
out vec4 outputColor;
uniform sampler2D data;
uniform sampler2D edgesX,edgesY;
uniform int rectilinear;
uniform int reduction;
uniform vec2 range,viewLow,viewHigh;
uniform float normWidth;
uniform int normMode,palette;
uniform int colorCount;
uniform vec3 colorStops[32];
float transformValue(float v){if(normMode==1)return log(v);if(normMode==2)return asinh(v/normWidth);return v;}
vec3 cmap(float t){if(colorCount>=2){float q=clamp(t,0.,1.)*float(colorCount-1);int i=min(int(floor(q)),colorCount-2);return mix(colorStops[i],colorStops[i+1],q-float(i));}vec3 a,b,c,d,e;if(palette==0){a=vec3(.13,.26,.45);b=vec3(.34,.58,.70);c=vec3(.97,.96,.93);d=vec3(.82,.46,.31);e=vec3(.49,.13,.18);}else if(palette==1){a=vec3(.97,.96,.91);b=vec3(.67,.78,.72);c=vec3(.31,.56,.55);d=vec3(.12,.31,.40);e=vec3(.05,.13,.23);}else{a=vec3(.08,.08,.16);b=vec3(.35,.13,.31);c=vec3(.71,.24,.25);d=vec3(.97,.61,.28);e=vec3(1.,.94,.73);}float q=clamp(t,0.,1.)*4.;if(q<1.)return mix(a,b,q);if(q<2.)return mix(b,c,q-1.);if(q<3.)return mix(c,d,q-2.);return mix(d,e,q-3.);}
int cellIndex(sampler2D edges,float p,int n){float low=texelFetch(edges,ivec2(0,0),0).r,high=texelFetch(edges,ivec2(n,0),0).r;float x=mix(low,high,p);int a=0,b=n;for(int k=0;k<24;k++){if(b-a<=1)break;int m=(a+b)/2;float edge=texelFetch(edges,ivec2(m,0),0).r;if(x<edge)b=m;else a=m;}return min(a,n-1);}
// Opt-in maximum over every source cell intersecting a screen pixel.
// This preserves isolated finite observations when a raster is minified;
// missing cells remain missing. Dynamic bounds cover the entire footprint.
float sampleField(vec2 p, ivec2 n) {
    ivec2 ij=rectilinear==1?ivec2(cellIndex(edgesX,p.x,n.x),cellIndex(edgesY,p.y,n.y)):clamp(ivec2(p*vec2(n)),ivec2(0),n-1);
    if(reduction==0)return texelFetch(data,ij,0).r;
    vec2 halfPixel=0.5*(abs(dFdx(p))+abs(dFdy(p)));
    vec2 a=clamp(p-halfPixel,vec2(0),vec2(1));
    vec2 b=clamp(p+halfPixel,vec2(0),vec2(1));
    ivec2 lo,hi;
    if(rectilinear==1){
        lo=ivec2(cellIndex(edgesX,a.x,n.x),cellIndex(edgesY,a.y,n.y));
        hi=ivec2(cellIndex(edgesX,b.x,n.x),cellIndex(edgesY,b.y,n.y));
    }else{
        lo=clamp(ivec2(floor(a*vec2(n))),ivec2(0),n-1);
        hi=clamp(ivec2(ceil(b*vec2(n)))-1,lo,n-1);
    }
    float peak=uintBitsToFloat(0x7fc00000u);
    for(int y=lo.y;y<=hi.y;y++)for(int x=lo.x;x<=hi.x;x++){
        float v=texelFetch(data,ivec2(x,y),0).r;
        if(!isnan(v)&&!isinf(v)&&(normMode!=1||v>0.)&&(isnan(peak)||v>peak))peak=v;
    }
    return peak;
}
void main(){
    vec2 p=mix(viewLow,viewHigh,uv);
    // Derivatives must be evaluated before divergent out-of-bounds branches.
    ivec2 n=textureSize(data,0);float v=sampleField(p,n);
    if(any(lessThan(p,vec2(0)))||any(greaterThan(p,vec2(1)))||isnan(v)||isinf(v)||(normMode==1&&v<=0.)){outputColor=vec4(1);return;}
    float a=transformValue(range.x),b=transformValue(range.y);
    outputColor=vec4(cmap((transformValue(v)-a)/(b-a)),1);
}
