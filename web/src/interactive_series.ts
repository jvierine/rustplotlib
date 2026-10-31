import init, {SeriesView,PlotViewport} from '../pkg/rustplotlib_web.js';
import {PlotControlGroup,bindPlotNavigation} from './plot_controls';
export {PlotControlGroup} from './plot_controls';

export type SeriesSpec={bounds:[number,number,number,number];xlabel:string;ylabel:string;compact_axes?:boolean;hide_x_labels?:boolean;panel_label?:string;utc_origin_seconds?:number;categories?:string[];bars?:boolean;equal_aspect?:boolean;sky?:{paper_style?:boolean;lonlat:number[][];nside?:number;central_longitude?:number;west_positive?:boolean;limits?:[number,number]};field?:{x:number[];y:number[];values:(number|null)[];limits:[number,number];palette?:string;label?:string;missing_value?:number};series:{name?:string;color:string;points:(number|null)[][];line?:boolean;radius?:number;errors?:(number|null)[];line_width?:number;fill?:string}[]};
let ready:Promise<unknown>|undefined;
let fontReady:Promise<unknown>|undefined;
function loadPlotFont(){return fontReady??=new FontFace('RustplotlibPagella',`url("${new URL('../fonts/texgyrepagella-regular.otf',import.meta.url)}")`).load().then(font=>{document.fonts.add(font);}).catch(error=>{fontReady=undefined;throw error;});}
/** Data and native axes are drawn in Rust/WASM. Interaction is enabled by default. */
export class InteractiveSeriesPlot {
  readonly viewport=new PlotViewport();
  readonly canvas=document.createElement('canvas');
  private view:SeriesView;
  private observer:ResizeObserver;
  private rect=[0,0,1,1];
  private box=document.createElement('div');
  private unbind:()=>void;
  private pending=0;
  private specJSON:string;
  private colorButton=document.createElement('button');
  private editor?:HTMLDivElement;
  private colorOverride?:[number,number];
  private constructor(private root:HTMLElement,private spec:SeriesSpec,private height:number,controls?:PlotControlGroup){
    this.specJSON=JSON.stringify(spec);
    const group=controls||new PlotControlGroup();
    const stage=document.createElement('div');stage.style.cssText='position:relative;background:white;';
    this.canvas.style.cssText='display:block;width:100%';this.canvas.tabIndex=0;
    this.box.style.cssText='position:absolute;display:none;pointer-events:none;border:1px dashed #12669a;background:#12669a22';
    this.colorButton.type='button';this.colorButton.setAttribute('aria-label','Adjust color limits');this.colorButton.title='Adjust color limits';
    this.colorButton.style.cssText='position:absolute;z-index:2;border:0;background:transparent;padding:0;cursor:pointer';
    this.colorButton.onclick=()=>this.editColors(stage);
    stage.append(this.canvas,this.box,this.colorButton);root.replaceChildren(...controls?[stage]:[group.element,stage]);
    this.view=new SeriesView(this.canvas);this.observer=new ResizeObserver(()=>this.schedule());this.observer.observe(root);
    const point=(e:MouseEvent)=>{const r=this.canvas.getBoundingClientRect();return [Math.max(0,Math.min(1,(e.clientX-r.left-this.rect[0])/this.rect[2])),Math.max(0,Math.min(1,1-(e.clientY-r.top-this.rect[1])/this.rect[3]))];};
    this.unbind=bindPlotNavigation({canvas:this.canvas,viewport:this.viewport,controls:group,point,changed:()=>this.draw(),box:b=>{if(!b){this.box.style.display='none';return;}const[x0,y0,x,y]=b;Object.assign(this.box.style,{display:'block',left:this.rect[0]+Math.min(x,x0)*this.rect[2]+'px',top:this.rect[1]+(1-Math.max(y,y0))*this.rect[3]+'px',width:Math.abs(x-x0)*this.rect[2]+'px',height:Math.abs(y-y0)*this.rect[3]+'px'});}});
    this.draw();
  }
  static async mount(root:HTMLElement,spec:SeriesSpec,options:{height?:number;compact?:boolean;controls?:PlotControlGroup}={}){ready??=init().catch(error=>{ready=undefined;throw error;});await Promise.all([ready,loadPlotFont()]);return new InteractiveSeriesPlot(root,spec,options.height||300,options.controls);}
  setHeight(height:number){this.height=height;this.schedule();}
  setSpec(spec:SeriesSpec){if(this.colorOverride){spec={...spec,...spec.field?{field:{...spec.field,limits:this.colorOverride}}:{},...spec.sky?{sky:{...spec.sky,limits:this.colorOverride}}:{}};}const next=JSON.stringify(spec);if(next===this.specJSON)return;this.spec=spec;this.specJSON=next;this.draw();}
  private editColors(stage:HTMLDivElement){
    this.editor?.remove();const editor=document.createElement('div');this.editor=editor;
    editor.setAttribute('role','dialog');editor.setAttribute('aria-label','Color limits');
    editor.style.cssText='position:absolute;right:10px;top:20px;z-index:5;display:grid;gap:10px;padding:14px;background:white;color:#172134;border:1px solid #9baab5;border-radius:5px;box-shadow:0 3px 15px #0003;font:14px system-ui';
    const limits=Array.from(this.view.color_limits()),inputs:HTMLInputElement[]=[],outputs:HTMLOutputElement[]=[];
    for(const [i,name]of ['Minimum value','Maximum value'].entries()){
      const label=document.createElement('label'),input=document.createElement('input'),output=document.createElement('output');
      label.textContent=name+' ';input.type='range';input.setAttribute('aria-label',name);
      const logarithmic=this.spec.sky?.paper_style;
      input.min=String(logarithmic?limits[0]/10:Math.min(0,limits[0]*2));input.max=String(logarithmic?limits[1]*2:Math.max(30,limits[1]*2));input.step=logarithmic?String(limits[0]/10):'0.1';input.value=String(limits[i]);output.value=String(limits[i]);
      input.style.display='block';label.append(input,output);editor.append(label);inputs.push(input);outputs.push(output);
    }
    const change=()=>{const low=Number(inputs[0].value),high=Number(inputs[1].value);outputs[0].value=String(low);outputs[1].value=String(high);if(high>low){this.colorOverride=[low,high];this.setSpec(this.spec);}};
    inputs.forEach(input=>input.oninput=change);
    const done=document.createElement('button');done.textContent='Done';done.onclick=()=>{editor.remove();this.editor=undefined;};editor.append(done);stage.append(editor);inputs[1].focus();
  }
  reset(){this.viewport.reset();this.draw();}
  private schedule(){cancelAnimationFrame(this.pending);this.pending=requestAnimationFrame(()=>this.draw());}
  private draw(){
    const style=getComputedStyle(this.root);
    const width=Math.max(this.spec.compact_axes?150:260,Math.min(1600,this.root.clientWidth-parseFloat(style.paddingLeft)-parseFloat(style.paddingRight)));
    this.canvas.style.width=width+'px';this.canvas.style.height=this.height+'px';
    this.rect=Array.from(this.view.render_plot(this.specJSON,width,this.height,Math.min(devicePixelRatio||1,2),this.viewport.bounds()));
    this.colorButton.hidden=!this.spec.field&&!this.spec.sky;
    Object.assign(this.colorButton.style,{left:this.rect[0]+this.rect[2]+8+'px',top:this.rect[1]+'px',width:'28px',height:this.rect[3]+'px'});
    this.canvas.setAttribute('aria-label',`${this.spec.ylabel} versus ${this.spec.xlabel}`);
    this.root.dataset.renderer='rustplotlib-wasm-series';
    this.root.dataset.viewBounds=JSON.stringify(Array.from(this.viewport.bounds()));
    this.root.dataset.seriesCount=String(this.spec.series.length);
    this.root.dataset.sampleCount=String(this.spec.series.reduce((n,s)=>n+s.points.length,0));
  }
  dispose(){this.unbind();this.editor?.remove();this.observer.disconnect();cancelAnimationFrame(this.pending);this.view.free();this.viewport.free();this.root.replaceChildren();}
}
