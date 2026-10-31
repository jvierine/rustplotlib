import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {mkdtemp,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {createServer} from 'node:http';
const {chromium}=await import(process.env.PLAYWRIGHT_MODULE||'playwright');
const directory=await mkdtemp(join(tmpdir(),'rustplotlib-controls-test-'));
execFileSync(process.env.ESBUILD_BIN||'node_modules/.bin/esbuild',['web/src/plot_controls.ts','--bundle','--format=esm',`--outfile=${directory}/controls.js`]);
const js=await readFile(join(directory,'controls.js'));
const server=createServer((req,res)=>{if(req.url==='/controls.js'){res.setHeader('Content-Type','text/javascript');res.end(js);return;}res.setHeader('Content-Type','text/html');res.end('<!doctype html><html><body><canvas width="400" height="250" tabindex="0"></canvas></body></html>');}).listen(0,'127.0.0.1');
await new Promise(resolve=>server.once('listening',resolve));
const browser=await chromium.launch({executablePath:process.env.CHROME,args:['--no-sandbox']});
try{
 const page=await browser.newPage();await page.goto(`http://127.0.0.1:${server.address().port}`);
 await page.evaluate(async()=>{
  const {PlotControlGroup,bindPlotNavigation}=await import('/controls.js'),canvas=document.querySelector('canvas');
  const calls=[],viewport={reset:()=>calls.push('reset'),zoom:()=>calls.push('zoom'),pan:()=>calls.push('pan'),zoom_box:()=>calls.push('box')};
  const controls=new PlotControlGroup();document.body.prepend(controls.element);
  const unbind=bindPlotNavigation({canvas,viewport,controls,point:e=>[e.offsetX/400,1-e.offsetY/250],changed:()=>calls.push('draw'),box:b=>canvas.dataset.selection=JSON.stringify(b||null)});
  window.test={calls,controls,unbind};
 });
 const canvas=page.locator('canvas');
 const wheel=()=>canvas.evaluate(c=>{const e=new WheelEvent('wheel',{cancelable:true,deltaY:20});c.dispatchEvent(e);return e.defaultPrevented;});
 assert.equal(await wheel(),false);assert.deepEqual(await page.evaluate(()=>test.calls),[]);
 assert.equal(await canvas.evaluate(c=>c.style.touchAction),'pan-y pinch-zoom');
 await page.getByRole('button',{name:'Zoom',exact:true}).click();assert.equal(await wheel(),true);
 await page.mouse.move(100,110);await page.mouse.down();await page.mouse.move(220,190);await page.mouse.up();
 assert.ok((await page.evaluate(()=>test.calls)).includes('box'));
 await page.getByRole('button',{name:'Pan',exact:true}).click();await page.mouse.move(100,110);await page.mouse.down();await page.mouse.move(140,130);await page.mouse.up();
 assert.ok((await page.evaluate(()=>test.calls)).includes('pan'));
 await page.getByRole('button',{name:'Zoom',exact:true}).click();await page.mouse.move(100,110);await page.mouse.down();await page.mouse.move(200,190);
 await canvas.dispatchEvent('pointercancel',{pointerId:1});assert.equal(await canvas.getAttribute('data-selection'),'null');await page.mouse.up();
 await canvas.focus();await page.keyboard.press('Escape');assert.equal(await page.getByRole('button',{name:'Scroll',exact:true}).getAttribute('aria-pressed'),'true');assert.equal(await wheel(),false);
 await page.getByRole('button',{name:'Zoom in',exact:true}).click();await page.getByRole('button',{name:'Zoom out',exact:true}).click();
 await page.getByRole('button',{name:'Reset view',exact:true}).click();assert.ok((await page.evaluate(()=>test.calls)).includes('reset'));
 await page.evaluate(()=>{test.unbind();test.calls.length=0;});await page.getByRole('button',{name:'Zoom in',exact:true}).click();assert.deepEqual(await page.evaluate(()=>test.calls),[]);
 console.log('PASS shared navigation: scroll default, explicit wheel/box zoom, pan, cancellation, keyboard reset, buttons and disposal');
}finally{await browser.close();server.close();await rm(directory,{recursive:true});}
