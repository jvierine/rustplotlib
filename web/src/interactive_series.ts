import init, {SeriesView,PlotViewport} from '../pkg/rustplotlib_web.js';

export type SeriesSpec={bounds:[number,number,number,number];xlabel:string;ylabel:string;utc_origin_seconds?:number;series:{name?:string;color:string;points:(number|null)[][];line?:boolean;radius?:number}[]};
let ready:Promise<unknown>|undefined;
/** Data and native axes are drawn in Rust/WASM. Interaction is enabled by default. */
export class InteractiveSeriesPlot {
  readonly viewport=new PlotViewport();
  readonly canvas=document.createElement('canvas');
  private view:SeriesView;
  private observer:ResizeObserver;
  private rect=[0,0,1,1];
  private box=document.createElement('div');
  private drag?:{x:number;y:number;lastX:number;lastY:number;pan:boolean};
  private pending=0;
  private specJSON:string;
  private constructor(private root:HTMLElement,private spec:SeriesSpec,private height:number){
    this.specJSON=JSON.stringify(spec);
    const toolbar=document.createElement('div'),reset=document.createElement('button'),help=document.createElement('span');
    toolbar.style.cssText='display:flex;gap:12px;align-items:center;flex-wrap:wrap;margin:0 6px 10px';
    reset.textContent='Reset view';reset.onclick=()=>this.reset();
    help.textContent='Scroll to zoom · drag to zoom · Shift-drag to pan · double-click to reset';help.style.cssText='font-size:.75rem;color:var(--muted,#526b7d)';toolbar.append(reset,help);
    const stage=document.createElement('div');stage.style.cssText='position:relative;background:white;';
    this.canvas.style.cssText='display:block;width:100%;touch-action:none';this.canvas.tabIndex=0;
    this.box.style.cssText='position:absolute;display:none;pointer-events:none;border:1px dashed #12669a;background:#12669a22';
    stage.append(this.canvas,this.box);root.replaceChildren(toolbar,stage);
    this.view=new SeriesView(this.canvas);this.observer=new ResizeObserver(()=>this.schedule());this.observer.observe(root);
    const point=(e:PointerEvent|WheelEvent)=>{const r=this.canvas.getBoundingClientRect();return [Math.max(0,Math.min(1,(e.clientX-r.left-this.rect[0])/this.rect[2])),Math.max(0,Math.min(1,1-(e.clientY-r.top-this.rect[1])/this.rect[3]))];};
    this.canvas.addEventListener('wheel',e=>{e.preventDefault();const [x,y]=point(e);this.viewport.zoom(x,y,Math.exp(Math.max(-1,Math.min(1,e.deltaY*.002))));this.draw();},{passive:false});
    this.canvas.ondblclick=()=>this.reset();
    this.canvas.onkeydown=e=>{if(e.key==='Escape'||e.key==='Home'){e.preventDefault();this.reset();}};
    this.canvas.onpointerdown=e=>{if(e.button!==0)return;e.preventDefault();const [x,y]=point(e);this.drag={x,y,lastX:x,lastY:y,pan:e.shiftKey};this.canvas.setPointerCapture(e.pointerId);};
    this.canvas.onpointermove=e=>{const d=this.drag;if(!d)return;const [x,y]=point(e);if(d.pan){this.viewport.pan(d.lastX-x,d.lastY-y);d.lastX=x;d.lastY=y;this.draw();}else Object.assign(this.box.style,{display:'block',left:this.rect[0]+Math.min(x,d.x)*this.rect[2]+'px',top:this.rect[1]+(1-Math.max(y,d.y))*this.rect[3]+'px',width:Math.abs(x-d.x)*this.rect[2]+'px',height:Math.abs(y-d.y)*this.rect[3]+'px'});};
    this.canvas.onpointerup=e=>{const d=this.drag;if(!d)return;this.drag=undefined;this.box.style.display='none';const [x,y]=point(e);if(!d.pan&&Math.abs(x-d.x)+Math.abs(y-d.y)>.015)this.viewport.zoom_box(d.x,d.y,x,y);this.draw();};
    this.canvas.onpointercancel=()=>{this.drag=undefined;this.box.style.display='none';};
    this.draw();
  }
  static async mount(root:HTMLElement,spec:SeriesSpec,options:{height?:number}={}){ready??=init();await ready;return new InteractiveSeriesPlot(root,spec,options.height||300);}
  setSpec(spec:SeriesSpec){this.spec=spec;this.specJSON=JSON.stringify(spec);this.draw();}
  reset(){this.viewport.reset();this.draw();}
  private schedule(){cancelAnimationFrame(this.pending);this.pending=requestAnimationFrame(()=>this.draw());}
  private draw(){
    const style=getComputedStyle(this.root);
    const width=Math.max(280,Math.min(1200,this.root.clientWidth-parseFloat(style.paddingLeft)-parseFloat(style.paddingRight)));
    this.canvas.style.width=width+'px';this.canvas.style.height=this.height+'px';
    this.rect=Array.from(this.view.render_plot(this.specJSON,width,this.height,Math.min(devicePixelRatio||1,2),this.viewport.bounds()));
    this.canvas.setAttribute('aria-label',`${this.spec.ylabel} versus ${this.spec.xlabel}`);
    this.root.dataset.renderer='rustplotlib-wasm-series';
    this.root.dataset.viewBounds=JSON.stringify(Array.from(this.viewport.bounds()));
    this.root.dataset.seriesCount=String(this.spec.series.length);
    this.root.dataset.sampleCount=String(this.spec.series.reduce((n,s)=>n+s.points.length,0));
  }
  dispose(){this.observer.disconnect();cancelAnimationFrame(this.pending);this.view.free();this.viewport.free();this.root.replaceChildren();}
}
