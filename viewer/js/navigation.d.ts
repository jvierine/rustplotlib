export interface NavigableViewport {
 zoom(x:number,y:number,factor:number):void;
 zoom_box(x0:number,y0:number,x1:number,y1:number):boolean;
 pan(dx:number,dy:number):void;
 reset():void;
}
export function bindPlotNavigation(canvas:HTMLCanvasElement,viewport:NavigableViewport,redraw:()=>void,axes?:()=>{x:number;y:number;width:number;height:number}):()=>void;
