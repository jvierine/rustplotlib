import init,{PathView} from '../pkg/rustplotlib_web.js';
await init();type Path={group:string,points:[number,number,number][]};
const canvas=document.querySelector<HTMLCanvasElement>('#currents')!,view=new PathView(canvas);
const manifest=await(await fetch('gallery/currents/manifest.json')).json();
const records=await Promise.all(manifest.map(async(row:any)=>(await fetch('gallery/currents/'+row.file)).json()));
const colors:Record<string,number[]>={dayside:[.1,.55,.32],tail:[.16,.38,.72],ring:[.65,.23,.65],r1:[.8,.25,.25],r2:[.06,.58,.58]};
const names:Record<string,string>={dayside:'Dayside',tail:'Tail',ring:'Ring-region',r1:'Region 1-like',r2:'Region 2-like'};
const legend=document.querySelector('#groups')!;for(const [key,color] of Object.entries(colors)){const label=document.createElement('label');label.style.color=`rgb(${color.map(c=>c*255).join(',')})`;label.innerHTML=`<input type="checkbox" checked data-group="${key}">${names[key]}`;legend.append(label);}
let index=1,yaw=-.9,pitch=.45,extent=34;
function draw(){const vertices:number[]=[];const enabled=new Set(Array.from(document.querySelectorAll<HTMLInputElement>('[data-group]:checked')).map(e=>e.dataset.group));
 for(const path of records[index].paths as Path[]){if(!enabled.has(path.group))continue;const color=colors[path.group];for(let i=1;i<path.points.length;i++)vertices.push(...path.points[i-1],...color,...path.points[i],...color);}
 view.set_paths(new Float32Array(vertices));view.set_camera(yaw,pitch,extent);document.querySelector('#epoch')!.textContent=records[index].utc.replace('T',' · ').replace('Z',' UTC');
}
const slider=document.querySelector<HTMLInputElement>('#time')!;slider.max=String(records.length-1);slider.value='1';slider.oninput=()=>{index=Number(slider.value);draw();};legend.addEventListener('change',draw);
let drag:[number,number]|null=null;canvas.onpointerdown=e=>{drag=[e.clientX,e.clientY];canvas.setPointerCapture(e.pointerId);};canvas.onpointermove=e=>{if(!drag)return;yaw+=(e.clientX-drag[0])*.006;pitch+=(e.clientY-drag[1])*.006;drag=[e.clientX,e.clientY];view.set_camera(yaw,pitch,extent);};canvas.onpointerup=()=>drag=null;canvas.onwheel=e=>{e.preventDefault();extent=Math.max(4,Math.min(100,extent*Math.exp(e.deltaY*.001)));view.set_camera(yaw,pitch,extent);};
let playing=false;document.querySelector('#play')!.addEventListener('click',()=>{playing=!playing;});setInterval(()=>{if(playing){index=(index+1)%records.length;slider.value=String(index);draw();}},1200);
draw();(window as any).currentsExample={ready:true,view,draw};
