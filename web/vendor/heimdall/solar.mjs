// Extracted from HEIMDALL web/public/flow.js. Earth-fixed Cartesian axes.
import {project,dot} from './frames.mjs';
const radians=Math.PI/180;

export function latitudeRing(axis,latitude){
 const n=Math.hypot(axis[0],axis[1]),u=n>1e-12?[-axis[1]/n,axis[0]/n,0]:[1,0,0],v=[axis[1]*u[2]-axis[2]*u[1],axis[2]*u[0]-axis[0]*u[2],axis[0]*u[1]-axis[1]*u[0]],s=Math.sin(latitude*radians),c=Math.cos(latitude*radians),points=[];
 for(let degree=0;degree<=360;degree+=.5){const a=degree*radians,p=axis.map((x,k)=>x*s+c*(u[k]*Math.cos(a)+v[k]*Math.sin(a)));points.push([Math.atan2(p[1],p[0])/radians,Math.asin(Math.max(-1,Math.min(1,p[2])))/radians]);}return points;
}

// Preserve HEIMDALL's display ephemeris. Precision-orbit clients supply their own Sun vector.
export function solarFrame(time){
 const date=new Date(time),year=date.getUTCFullYear(),decimalYear=year+(time-Date.UTC(year,0,1))/(Date.UTC(year+1,0,1)-Date.UTC(year,0,1));
 const jd=time/86400000+2440587.5,t=(jd-2451545)/36525,l0=(280.46646+t*(36000.76983+t*.0003032))*radians,m=(357.52911+t*(35999.05029-.0001537*t))*radians,lambda=l0+(1.914602-.004817*t-.000014*t*t)*Math.sin(m)*radians+.019993*Math.sin(2*m)*radians+.000289*Math.sin(3*m)*radians,epsilon=(23.439291-.0130042*t)*radians,decl=Math.asin(Math.sin(epsilon)*Math.sin(lambda)),ra=Math.atan2(Math.cos(epsilon)*Math.sin(lambda),Math.cos(lambda)),gmst=(280.46061837+360.98564736629*(jd-2451545)+.000387933*t*t-t*t*t/38710000)*radians,solarLon=ra-gmst;
 return {year:decimalYear,sun:[Math.cos(decl)*Math.cos(solarLon),Math.cos(decl)*Math.sin(solarLon),Math.sin(decl)],ecliptic:[-Math.sin(epsilon)*Math.sin(gmst),-Math.sin(epsilon)*Math.cos(gmst),Math.cos(epsilon)],subsolar:[solarLon/radians,decl/radians]};
}

// The same near/far-side clipping for Canvas and WebGL renderers. Radii are Earth radii.
export function sunDirectionGuide(sun,front,endRadius=1.65){
 const z=Math.max(-1,Math.min(1,dot(sun,front)));
 const startRadius=z>=0?1.03:Math.max(1.03,1.01/Math.max(1e-9,Math.sqrt(Math.max(0,1-z*z))));
 return {start:sun.map(v=>v*startRadius),end:sun.map(v=>v*endRadius),startRadius,endRadius,
  visible:startRadius<endRadius,farSide:z<0,label:z<0?'Sun (far side)':'Sun'};
}

// HEIMDALL's existing Canvas Sun-line renderer, with scene globals passed explicitly.
export function drawSunDirection(ctx,sun,camera,screen,{dpr=1,endRadius=1.65}={}){
 const guide=sunDirectionGuide(sun,camera.front,endRadius),b=screen(project(guide.end,camera));
 ctx.strokeStyle='#ffe17d';ctx.fillStyle='#ffe17d';ctx.font=12*dpr+'px sans-serif';
 if(guide.visible){
  const a=screen(project(guide.start,camera));ctx.beginPath();ctx.moveTo(...a);ctx.lineTo(...b);ctx.stroke();
  const angle=Math.atan2(b[1]-a[1],b[0]-a[0]),size=8*dpr;ctx.beginPath();ctx.moveTo(...b);
  ctx.lineTo(b[0]-size*Math.cos(angle-.4),b[1]-size*Math.sin(angle-.4));ctx.lineTo(b[0]-size*Math.cos(angle+.4),b[1]-size*Math.sin(angle+.4));ctx.closePath();ctx.fill();
  ctx.fillText(guide.label,b[0]+8,b[1]-8);
 }else ctx.fillText('Sun direction: behind Earth',12,24*dpr);
 return guide;
}
