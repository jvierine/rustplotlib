/** Shared navigation: ordinary wheel/touch gestures belong to the page.
 * Plots opt into drag navigation through an explicit, visible mode switch. */
export type NavigationMode='scroll'|'zoom'|'pan';
type Viewport={reset():void;zoom(x:number,y:number,factor:number):unknown;pan(x:number,y:number):unknown;zoom_box(x0:number,y0:number,x1:number,y1:number):unknown};
type Member={viewport:Viewport;changed():void;modeChanged(mode:NavigationMode):void};
export class PlotControlGroup {
  readonly element=document.createElement('div');
  private members=new Set<Member>();
  private buttons=new Map<NavigationMode,HTMLButtonElement>();
  mode:NavigationMode='scroll';
  constructor(){
    this.element.className='rustplotlib-controls';this.element.setAttribute('role','toolbar');this.element.setAttribute('aria-label','Plot navigation');
    this.element.style.cssText='display:flex;gap:5px;align-items:center;flex-wrap:wrap;margin:5px 0;font:12px system-ui';
    const button=(text:string,action:()=>void)=>{const b=document.createElement('button');b.type='button';b.textContent=text;b.onclick=action;this.element.append(b);return b;};
    for(const [mode,label]of [['scroll','Scroll'],['zoom','Zoom'],['pan','Pan']]as const)this.buttons.set(mode,button(label,()=>this.setMode(mode)));
    button('+',()=>this.zoom(.7)).setAttribute('aria-label','Zoom in');
    button('−',()=>this.zoom(1/.7)).setAttribute('aria-label','Zoom out');
    button('Reset view',()=>this.reset());
    const help=document.createElement('span');help.textContent='Scroll page normally · select Zoom/Pan to drag';help.style.cssText='color:var(--muted,#526b7d);font-size:11px';this.element.append(help);
    this.setMode('scroll');
  }
  add(member:Member){this.members.add(member);member.modeChanged(this.mode);return()=>this.members.delete(member);}
  setMode(mode:NavigationMode){this.mode=mode;for(const [key,b]of this.buttons)b.setAttribute('aria-pressed',String(key===mode));this.members.forEach(m=>m.modeChanged(mode));}
  reset(){this.setMode('scroll');this.members.forEach(m=>{m.viewport.reset();m.changed();});}
  private zoom(factor:number){this.members.forEach(m=>{m.viewport.zoom(.5,.5,factor);m.changed();});}
}
export function bindPlotNavigation(options:{canvas:HTMLCanvasElement;viewport:Viewport;controls:PlotControlGroup;point(e:MouseEvent):number[];changed():void;box?(bounds?:number[]):void;select?(x:number,y:number):void}){
  const {canvas,viewport,controls,point,changed,box}=options,abort=new AbortController(),signal=abort.signal;
  let drag:{id:number;x:number;y:number;lastX:number;lastY:number;pan:boolean}|undefined;
  const cancel=()=>{if(drag&&canvas.hasPointerCapture(drag.id))canvas.releasePointerCapture(drag.id);drag=undefined;box?.();};
  const remove=controls.add({viewport,changed,modeChanged(mode){cancel();canvas.style.touchAction=mode==='scroll'?'pan-y pinch-zoom':'none';canvas.style.cursor=mode==='pan'?'grab':mode==='zoom'?'crosshair':'auto';}});
  canvas.addEventListener('wheel',e=>{if(controls.mode==='scroll')return;e.preventDefault();const[x,y]=point(e);viewport.zoom(x,y,Math.exp(Math.max(-1,Math.min(1,e.deltaY*.002))));changed();},{passive:false,signal});
  canvas.addEventListener('dblclick',e=>{e.preventDefault();controls.reset();},{signal});
  canvas.addEventListener('keydown',e=>{if(e.key==='Escape'||e.key==='Home'){e.preventDefault();controls.reset();}},{signal});
  canvas.addEventListener('pointerdown',e=>{if(e.button!==0||controls.mode==='scroll')return;e.preventDefault();const[x,y]=point(e);drag={id:e.pointerId,x,y,lastX:x,lastY:y,pan:controls.mode==='pan'||e.shiftKey};canvas.setPointerCapture(e.pointerId);},{signal});
  canvas.addEventListener('pointermove',e=>{const d=drag;if(!d||d.id!==e.pointerId)return;const[x,y]=point(e);if(d.pan){viewport.pan(d.lastX-x,d.lastY-y);d.lastX=x;d.lastY=y;changed();}else box?.([d.x,d.y,x,y]);},{signal});
  canvas.addEventListener('pointerup',e=>{const d=drag;if(!d||d.id!==e.pointerId)return;const[x,y]=point(e);cancel();if(!d.pan){if(Math.abs(x-d.x)+Math.abs(y-d.y)>.015){viewport.zoom_box(d.x,d.y,x,y);changed();}else options.select?.(x,y);}},{signal});
  canvas.addEventListener('pointercancel',cancel,{signal});canvas.addEventListener('lostpointercapture',cancel,{signal});
  return()=>{cancel();remove();abort.abort();};
}
