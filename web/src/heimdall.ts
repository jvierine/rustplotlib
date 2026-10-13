// Reuse the deployed HEIMDALL geometry, projection, palette and WebGL renderer.
// @ts-ignore Existing HEIMDALL module.
import {MapGpu} from '../vendor/heimdall/map-gpu.mjs';
// @ts-ignore Existing HEIMDALL module.
import {decodeGeometryF16,decodeMapF16} from '../vendor/heimdall/map-f16.mjs';
// @ts-ignore Existing HEIMDALL module.
import {cameraBasis,project} from '../vendor/heimdall/frames.mjs';
// @ts-ignore Existing HEIMDALL module.
import {solarFrame,latitudeRing} from '../vendor/heimdall/solar.mjs';
// @ts-ignore Existing HEIMDALL module.
import {colormapBytes,colormapColor} from '../vendor/heimdall/colormaps.mjs';
// @ts-ignore Existing HEIMDALL module.
import {createNeighbourRenderer} from '../vendor/heimdall/neighbour-fill.mjs';
type Point=[number,number];
const canvas=document.querySelector<HTMLCanvasElement>('#map')!,ctx=canvas.getContext('2d')!;
const slider=document.querySelector<HTMLInputElement>('#time')!,label=document.querySelector<HTMLElement>('#epoch')!;
const projection=document.querySelector<HTMLSelectElement>('#projection')!;
const read=async (name:string)=>{const r=await fetch(`gallery/heimdall/${name}`);if(!r.ok)throw Error(`${name}: ${r.status}`);return r;};
const provenance=await (await read('provenance.json')).json();
const geometry=decodeGeometryF16(await (await read('geometry.bin')).arrayBuffer());
const frames=await Promise.all(provenance.minutes.map(async(m:number)=>decodeMapF16(await(await read(`${m}.bin`)).arrayBuffer())));
const world=await(await read('world.geojson')).json(),magnetic=await(await read('magnetic.json')).json();
const coasts:Point[][]=[];for(const f of world.features){if(f.geometry.type==='Polygon')coasts.push(...f.geometry.coordinates);if(f.geometry.type==='MultiPolygon')for(const p of f.geometry.coordinates)coasts.push(...p);}
const gpu=new MapGpu(),fill=createNeighbourRenderer(),palette=colormapBytes('rainbow');
let yaw=1.0826666,pitch=.7733333,zoom=1,index=4,playing=false;
slider.max=String(frames.length-1);slider.value=String(index);
function draw(){ render(canvas,false); }
function render(canvas:HTMLCanvasElement,scientific:boolean){
 const ctx=canvas.getContext('2d')!;
 const dpr=scientific?2:Math.min(devicePixelRatio||1,2);
 if(!scientific){canvas.width=Math.round(canvas.clientWidth*dpr);canvas.height=Math.round(canvas.clientHeight*dpr);}
 const w=canvas.width,h=canvas.height,r=Math.min(w,h)*.46*(scientific?1:zoom),globe=projection.value==='globe';
 const camera=cameraBasis({x:[1,0,0],y:[0,1,0],z:[0,0,1]},yaw,pitch);
 ctx.fillStyle='white';ctx.fillRect(0,0,w,h);
 if(globe){ctx.beginPath();ctx.arc(w/2,h/2,r,0,Math.PI*2);ctx.fillStyle='#edf0f2';ctx.fill();}
 const cells=frames[index].cells;
 const filled=document.querySelector<HTMLInputElement>('#fill')!.checked?fill(cells,geometry,0,(c:any)=>c.values[0]):null;
 gpu.draw(ctx,filled?.geometry||geometry,filled?.cells||cells,filled?(c:any)=>c.value:(c:any)=>c.values[0],{field:0,globe,camera,radius:r,mapZoom:1,mapX:0,mapY:0,limits:[0,Math.log10(40)],signed:false,log:true,palette});
 const point=(q:Point)=>{const lon=q[0]*Math.PI/180,lat=q[1]*Math.PI/180;const v=project([Math.cos(lat)*Math.cos(lon),Math.cos(lat)*Math.sin(lon),Math.sin(lat)],camera);return {p:globe?[w/2+r*v[0],h/2-r*v[1]]:[w*(.5+q[0]/360),h*(.5-q[1]/180)],visible:!globe||v[2]>=0};};
 const line=(ring:Point[],color:string,width:number)=>{ctx.strokeStyle=color;ctx.lineWidth=width*dpr;ctx.beginPath();let prior:number[]|null=null;for(const q of ring){const {p,visible}=point(q);if(!visible){prior=null;continue;}if(!prior||Math.hypot(p[0]-prior[0],p[1]-prior[1])>w/3)ctx.moveTo(p[0],p[1]);else ctx.lineTo(p[0],p[1]);prior=p;}ctx.stroke();};
 if(document.querySelector<HTMLInputElement>('#grid')!.checked){for(let lat=-60;lat<=60;lat+=30)line(Array.from({length:721},(_,k)=>[k*.5-180,lat] as Point),'#6d778055',.4);for(let lon=-180;lon<180;lon+=30)line(Array.from({length:361},(_,k)=>[lon,k*.5-90] as Point),'#6d778055',.4);}
 for(const ring of coasts)line(ring,'#303b45',.55);
 if(document.querySelector<HTMLInputElement>('#magnetic')!.checked)for(const contour of magnetic.contours)line(contour.points,'#73588b',.55);
 const minute=provenance.minutes[index],epoch=Date.parse('2025-09-25T00:00:00Z')+minute*60000;
 if(document.querySelector<HTMLInputElement>('#terminator')!.checked)line(latitudeRing(solarFrame(epoch).sun,0),'#42494c',1);
 label.textContent=`2025-09-25 · ${String(Math.floor(minute/60)).padStart(2,'0')}:${String(minute%60).padStart(2,'0')} UTC`;
 canvas.dataset.renderer='heimdall-webgl2';
}
slider.oninput=()=>{index=Number(slider.value);draw();};
for(const el of Array.from(document.querySelectorAll('input[type=checkbox],select')))el.addEventListener('change',draw);
let drag:Point|null=null;canvas.onpointerdown=e=>{drag=[e.clientX,e.clientY];canvas.setPointerCapture(e.pointerId);};canvas.onpointermove=e=>{if(!drag)return;yaw+=(e.clientX-drag[0])*.005;pitch=Math.max(-Math.PI/2,Math.min(Math.PI/2,pitch+(e.clientY-drag[1])*.005));drag=[e.clientX,e.clientY];draw();};canvas.onpointerup=()=>{drag=null;};canvas.onwheel=e=>{e.preventDefault();zoom=Math.max(.5,Math.min(1.7,zoom*Math.exp(-e.deltaY*.001)));draw();};
document.querySelector('#play')!.addEventListener('click',()=>{playing=!playing;document.querySelector('#play')!.textContent=playing?'Pause':'Play';});setInterval(()=>{if(playing){index=(index+1)%frames.length;slider.value=String(index);draw();}},500);
document.querySelector('#reset')!.addEventListener('click',()=>{yaw=1.0826666;pitch=.7733333;zoom=1;draw();});
function scientificFigure(widthMm=85,fontPt=10){
 const fontPx=fontPt*25.4/72*2400/widthMm;
 const scene=document.createElement('canvas');scene.width=2400;scene.height=2400;render(scene,true);
 const figure=document.createElement('canvas');figure.width=2400;figure.height=2840;
 const c=figure.getContext('2d')!;c.fillStyle='white';c.fillRect(0,0,2400,2840);c.drawImage(scene,0,40);
 c.fillStyle='#172029';c.font=`${fontPx}px Georgia`;c.fillText(label.textContent||'',150,95);
 const x=300,y=2500,width=1800,height=32;
 const grad=c.createLinearGradient(x,0,x+width,0);
 for(let k=0;k<palette.length/3;k++){const i=k*3;grad.addColorStop(k/(palette.length/3-1),`rgb(${palette[i]},${palette[i+1]},${palette[i+2]})`);}
 c.fillStyle=grad;c.fillRect(x,y,width,height);c.fillStyle='#172029';c.textAlign='center';c.font=`${fontPx}px Georgia`;
 for(const tick of [1,2,5,10,20,40]){const px=x+Math.log10(tick)/Math.log10(40)*width;c.fillText(String(tick),px,y+135);}
 c.font=`${fontPx}px Georgia`;c.fillText('Vertical TEC [TECU]',1200,2780);
 return figure;
}
document.querySelector('#save')!.addEventListener('click',()=>{const a=document.createElement('a');a.download='heimdall-vtec-scientific.png';a.href=scientificFigure().toDataURL();a.click();});
window.addEventListener('resize',draw);draw();(window as any).heimdallExample={ready:true,draw,scientificFigure,scientificScene:()=>{const c=document.createElement('canvas');c.width=2400;c.height=2400;render(c,true);return c;},palette:Array.from(palette),provenance};
