import {scientificMaps} from './colormap-data.mjs';
export const colormapNames=['vik','cork','berlin','roma','vanimo','plasma','turbo','black-white','blue-red','rainbow'];
const cache=new Map();
export function resolveColormap(name,signed=false,dtec=false){return colormapNames.includes(name)?name:dtec?'plasma':signed?'blue-red':'rainbow';}
function hsl(h,s,l){return [0,8,4].map(n=>{const k=(n+h/30)%12;return Math.round(255*(l-s*Math.min(l,1-l)*Math.max(-1,Math.min(k-3,9-k,1))));});}
const plasma=['0d0887','6a00a8','b12a90','e16462','fca636','f0f921'];
function anchored(stops,t){
 const p=Math.max(0,Math.min(1,t))*(stops.length-1),a=Math.floor(p),b=Math.min(stops.length-1,a+1),f=p-a;
 return [0,1,2].map(c=>Math.round(parseInt(stops[a].slice(c*2,c*2+2),16)*(1-f)+parseInt(stops[b].slice(c*2,c*2+2),16)*f));
}
function turboColor(t){
 const k=[[.13572138,4.6153926,-42.66032258,132.13108234,-152.94239396,59.28637943],[.09140261,2.19418839,4.84296658,-14.18503333,4.27729857,2.82956604],[.1066733,12.64194608,-60.58204836,110.36276771,-89.90310912,27.34824973]];
 return k.map(row=>Math.round(255*Math.max(0,Math.min(1,row.reduceRight((sum,value)=>sum*t+value)))));
}
export function colormapBytes(name){
 name=resolveColormap(name);if(cache.has(name))return cache.get(name);
 const table=new Uint8Array(256*3),hex=scientificMaps[name];
 for(let i=0;i<256;i++){
  const t=i/255;
  table.set(hex?[0,1,2].map(c=>parseInt(hex.slice(i*6+c*2,i*6+c*2+2),16)):name==='plasma'?anchored(plasma,t):name==='turbo'?turboColor(t):name==='black-white'?[t*255,t*255,t*255].map(Math.round):name==='blue-red'?hsl(t<.5?220:0,.75,.95-.5*Math.abs(2*t-1)):hsl(240-240*t,.85,.55),i*3);
 }
 cache.set(name,table);return table;
}
export function colormapColor(name,t){
 const bytes=colormapBytes(name),p=Math.max(0,Math.min(1,t))*255,a=Math.floor(p),b=Math.min(255,a+1),f=p-a;
 return 'rgb('+[0,1,2].map(c=>Math.round(bytes[a*3+c]*(1-f)+bytes[b*3+c]*f)).join(',')+')';
}
const gradients=new Map();
export function colormapGradient(name){
 if(!gradients.has(name))gradients.set(name,'linear-gradient(to right,'+Array.from({length:33},(_,i)=>colormapColor(name,i/32)).join(',')+')');
 return gradients.get(name);
}
