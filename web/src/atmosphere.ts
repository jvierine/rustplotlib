import init,{VolumeView} from '../pkg/rustplotlib_web.js';
await init();const views:VolumeView[]=[];
for(const [id,lo,hi,palette] of [ ['temperature',250,1100,[.08,.12,.35,.20,.50,.70,.96,.80,.32,.75,.17,.15]],['electron_density',0,1e12,[.07,.08,.15,.18,.36,.30,.59,.56,.75,.98,.98,.96]],['zonal_wind',-200,200,[.12,.23,.61,.97,.96,.92,.69,.10,.16]] ] as [string,number,number,number[]][]){
 const canvas=document.querySelector<HTMLCanvasElement>('#'+id)!;
 const v=new VolumeView(canvas);v.set_auto_draw(false);views.push(v);const data=new Float32Array(await(await fetch(`data/atmosphere/${id}.f32`)).arrayBuffer());
 v.set_data(80,80,80,data);v.set_colormap(new Float32Array(palette));v.set_transfer(lo,hi,3,0);v.set_spherical_shell(.21,.47,true);v.set_camera(.7,.4);v.set_auto_draw(true);v.draw();
 let drag:[number,number]|null=null;let yaw=.7,pitch=.4;
 canvas.onpointerdown=e=>{drag=[e.clientX,e.clientY];canvas.setPointerCapture(e.pointerId);};canvas.onpointermove=e=>{if(!drag)return;yaw+=(e.clientX-drag[0])*.008;pitch+=(e.clientY-drag[1])*.008;drag=[e.clientX,e.clientY];v.set_camera(yaw,pitch);};canvas.onpointerup=()=>{drag=null;};
}
let spin=false,t=.7;document.querySelector('#rotate')!.addEventListener('click',()=>{spin=!spin;});setInterval(()=>{if(spin){t+=.02;views.forEach(v=>v.set_camera(t,.4));}},60);
(window as any).atmosphereExample={ready:true,views};
