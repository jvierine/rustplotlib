// Display frames in Earth-fixed Cartesian coordinates. Conventions:
// https://www.spenvis.oma.be/help/background/coortran/coortran.html
// Degree-one IGRF-14: https://www.ngdc.noaa.gov/IAGA/vmod/coeffs/igrf14coeffs.txt
export const dot=(a,b)=>a[0]*b[0]+a[1]*b[1]+a[2]*b[2];
export const cross=(a,b)=>[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
const unit=a=>{const n=Math.hypot(...a);return a.map(v=>v/n);};
const g10=[-31543,-31464,-31354,-31212,-31060,-30926,-30805,-30715,-30654,-30594,-30554,-30500,-30421,-30334,-30220,-30100,-29992,-29873,-29775,-29692,-29619.4,-29554.63,-29496.57,-29441.46,-29403.41,-29350];
const g11=[-2298,-2298,-2297,-2306,-2317,-2318,-2316,-2306,-2292,-2285,-2250,-2215,-2169,-2119,-2068,-2013,-1956,-1905,-1848,-1784,-1728.2,-1669.05,-1586.42,-1501.77,-1451.37,-1410.3];
const h11=[5922,5909,5898,5875,5845,5817,5808,5812,5821,5810,5815,5820,5791,5776,5737,5675,5604,5500,5406,5306,5186.1,5077.99,4944.26,4795.99,4653.35,4545.5];
export function dipoleNorth(year){
 const y=Math.max(1900,Math.min(2030,year));
 const coefficient=(values,rate)=>{
  if(y>=2025)return values[25]+(y-2025)*rate;
  const i=Math.floor((y-1900)/5),f=(y-1900)/5-i;
  return values[i]*(1-f)+values[i+1]*f;
 };
 return unit([-coefficient(g11,10),-coefficient(h11,-21.5),-coefficient(g10,12.6)]);
}
export function frameBasis(frame,sun,ecliptic,year){
 if(frame==='geo')return {x:[1,0,0],y:[0,1,0],z:[0,0,1]};
 const x=unit(sun),north=frame==='gsm'?dipoleNorth(year):ecliptic;
 const y=unit(cross(north,x)),z=unit(cross(x,y));
 return {x,y,z};
}
export function cameraBasis(basis,yaw,pitch){
 const c=Math.cos(yaw),s=Math.sin(yaw),cp=Math.cos(pitch),sp=Math.sin(pitch);
 const forward=basis.x.map((v,i)=>v*c-basis.y[i]*s);
 return {right:basis.x.map((v,i)=>v*s+basis.y[i]*c),
  up:basis.z.map((v,i)=>v*cp-forward[i]*sp),
  front:basis.z.map((v,i)=>v*sp+forward[i]*cp)};
}
export function sunUpBasis(basis,sun){
 const z=unit(sun);
 const reference=Math.abs(dot(z,basis.z))<.99?basis.z:basis.y;
 const x=unit(reference.map((v,i)=>v-dot(reference,z)*z[i]));
 return {x,y:unit(cross(z,x)),z};
}
export function clampGlobePitch(pitch,sunUp=false){
 const limit=sunUp?85*Math.PI/180:Math.PI/2;
 return Math.max(-limit,Math.min(limit,pitch));
}
export function project(q,camera){return [dot(q,camera.right),dot(q,camera.up),dot(q,camera.front)];}
