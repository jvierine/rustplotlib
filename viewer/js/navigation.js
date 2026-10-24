/** Standard DOM gesture bridge; all viewport transforms run in Rust/Wasm.
 * Ordinary wheel scroll belongs to the page. Cmd/Ctrl-drag selects a zoom box.
 * axes() returns the data rectangle in canvas CSS coordinates (top-left origin).
 */
export function bindPlotNavigation(canvas, viewport, redraw, axes = () => ({x:0,y:0,width:canvas.clientWidth,height:canvas.clientHeight})) {
 const box=document.createElement('div');
 Object.assign(box.style,{position:'fixed',pointerEvents:'none',display:'none',border:'1.5px dashed #176fa3',background:'rgba(45,137,200,.16)',boxSizing:'border-box',zIndex:'2147483647'});
 box.setAttribute('aria-hidden','true');document.body.append(box);
 const events=[];const on=(target,name,fn,options)=>{target.addEventListener(name,fn,options);events.push(()=>target.removeEventListener(name,fn,options));};
 const point=e=>{const r=canvas.getBoundingClientRect(),a=axes();return [(e.clientX-r.left-a.x)/a.width,1-(e.clientY-r.top-a.y)/a.height]};
 const clamp=p=>p.map(v=>Math.max(0,Math.min(1,v)));
 let drag=null,moved=false;
 const cancel=()=>{drag=null;box.style.display='none'};
 const show=(start,end)=>{const r=canvas.getBoundingClientRect(),a=axes();Object.assign(box.style,{display:'block',left:`${r.left+a.x+Math.min(start[0],end[0])*a.width}px`,top:`${r.top+a.y+(1-Math.max(start[1],end[1]))*a.height}px`,width:`${Math.abs(end[0]-start[0])*a.width}px`,height:`${Math.abs(end[1]-start[1])*a.height}px`})};
 on(canvas,'wheel',e=>{if(!e.metaKey&&!e.ctrlKey)return;const p=point(e);if(p.some(v=>v<0||v>1))return;e.preventDefault();viewport.zoom(p[0],p[1],Math.exp(Math.max(-1,Math.min(1,e.deltaY*.002))));redraw()},{passive:false});
 on(canvas,'contextmenu',e=>{e.preventDefault();cancel();viewport.reset();redraw()});
 on(canvas,'pointerdown',e=>{if(e.button!==0)return;const p=point(e);if(p.some(v=>v<0||v>1))return;e.preventDefault();drag={start:p,last:p,box:e.metaKey||e.ctrlKey};moved=false;canvas.setPointerCapture(e.pointerId);if(drag.box)show(p,p)});
 on(canvas,'pointermove',e=>{if(!drag)return;const p=clamp(point(e)),a=axes();if(Math.hypot((p[0]-drag.start[0])*a.width,(p[1]-drag.start[1])*a.height)>4)moved=true;if(drag.box)show(drag.start,p);else{viewport.pan(drag.last[0]-p[0],drag.last[1]-p[1]);redraw()}drag.last=p});
 on(canvas,'pointerup',e=>{if(!drag)return;const p=clamp(point(e)),a=axes();if(drag.box&&Math.abs(p[0]-drag.start[0])*a.width>=4&&Math.abs(p[1]-drag.start[1])*a.height>=4){viewport.zoom_box(...drag.start,...p);moved=true;redraw()}cancel()});
 on(canvas,'pointercancel',cancel);on(canvas,'lostpointercapture',cancel);on(window,'blur',cancel);on(window,'keydown',e=>{if(e.key==='Escape')cancel()});
 on(canvas,'click',e=>{if(moved){e.stopImmediatePropagation();moved=false}},true);
 return ()=>{events.forEach(remove=>remove());box.remove()};
}

/** Shared colorbar limit editor. Hit testing and scale application use the plot API.
 * Options: automaticLimits() -> [low,high], onChange(low,high,automatic).
 * The canvas may ignore pointer events; its parent still owns the colorbar hit region.
 */
export function bindColorbarEditor(canvas, plot, options = {}) {
  const host=canvas.parentElement||canvas;
  let dialog;
  const remove=()=>{dialog?.remove();dialog=null;};
  const open=event=>{
    const bounds=canvas.getBoundingClientRect();
    const x=event.clientX-bounds.left,y=event.clientY-bounds.top;
    const [left,top,width,height]=plot.colorbar_rect();
    if(x<left||x>left+width||y<top||y>top+height)return;
    event.preventDefault();event.stopPropagation();remove();
    dialog=document.createElement('dialog');
    Object.assign(dialog.style,{font:'16px system-ui',padding:'24px',border:'1px solid #aebfc6',borderRadius:'9px',width:'340px',maxWidth:'90vw',color:'#243746'});
    dialog.setAttribute('aria-label','Color limits');
    const heading=document.createElement('h2');heading.textContent='Color limits';heading.style.margin='0 0 10px';
    const unit=document.createElement('p');unit.textContent=plot.color_label();
    const inputs=Array.from(plot.scale_limits(),(value,i)=>{const label=document.createElement('label');label.textContent=i?'Upper limit':'Lower limit';label.style.display='block';label.style.margin='12px 0';const input=document.createElement('input');input.type='number';input.step='any';input.value=Number(value.toPrecision(7));input.setAttribute('aria-label',label.textContent);Object.assign(input.style,{font:'inherit',display:'block',width:'100%',padding:'8px',marginTop:'4px',boxSizing:'border-box'});label.append(input);return {label,input};});
    const error=document.createElement('p');error.setAttribute('role','alert');error.style.color='#a31e35';
    const actions=document.createElement('div');Object.assign(actions.style,{display:'flex',gap:'8px',justifyContent:'flex-end',marginTop:'18px'});
    const button=(text,fn)=>{const b=document.createElement('button');b.textContent=text;Object.assign(b.style,{font:'inherit',padding:'7px 10px',cursor:'pointer'});b.onclick=fn;actions.append(b);};
    button('Cancel',remove);
    if(options.automaticLimits)button('Automatic',()=>{const [a,b]=options.automaticLimits();plot.set_scale(a,b);options.onChange?.(a,b,true);remove();});
    button('Apply',()=>{const a=Number(inputs[0].input.value),b=Number(inputs[1].input.value);if(!inputs.every(p=>p.input.value.trim())||!Number.isFinite(a)||!Number.isFinite(b)||b<=a){error.textContent='Enter a lower limit smaller than the upper limit.';return;}plot.set_scale(a,b);options.onChange?.(a,b,false);remove();});
    dialog.append(heading,unit,...inputs.map(p=>p.label),error,actions);document.body.append(dialog);dialog.addEventListener('cancel',remove,{once:true});dialog.showModal();inputs[0].input.focus();inputs[0].input.select();
  };
  host.addEventListener('pointerdown',open,true);
  return ()=>{host.removeEventListener('pointerdown',open,true);remove();};
}
