import {chromium} from 'playwright';
import {mkdir,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {existsSync} from 'node:fs';
await mkdir('validation',{recursive:true});
const browser=await chromium.launch({executablePath:process.env.CHROME||(existsSync('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome')?'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome':chromium.executablePath()),args:['--enable-webgl','--use-gl=angle','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
const page=await browser.newPage({viewport:{width:1280,height:1000},deviceScaleFactor:1});const errors=[];page.on('pageerror',e=>errors.push(String(e)));
const hash=async id=>createHash('sha256').update(await page.locator(id).screenshot()).digest('hex');
try{
 await page.goto('http://127.0.0.1:8791/');await page.waitForFunction(()=>window.rustplotlib?.ready,null,{timeout:120000});
 const nonuniform=await page.evaluate(()=>{
  const f=window.rustplotlib.field;f.set_scale(-1,1,0,.1,0);
  f.set_rectilinear_data(new Float32Array([0,1,4]),new Float32Array([0,1]),new Float32Array([-1,1]));
  const blue=Array.from(f.pixel(72,100)),red=Array.from(f.pixel(288,100));
  f.set_colormap(new Float32Array([0,0,0,1,1,1]));const custom=Array.from(f.pixel(72,100));f.set_scale(-1,1,0,.1,0);
  f.set_data(2,1,new Float32Array([NaN,1]));const missing=Array.from(f.pixel(72,100));
  window.rustplotlib.fieldFrame();return {blue,red,missing,custom};
 });
 const close=(a,b)=>a.every((v,i)=>Math.abs(v-b[i])<=1);
 if(!close(nonuniform.blue,[33,66,115,255])||!close(nonuniform.red,[125,33,46,255])||!close(nonuniform.missing,[255,255,255,255])||!close(nonuniform.custom,[0,0,0,255]))throw Error('Nonuniform cells, CPU colormap parity, custom colormap or missing values failed: '+JSON.stringify(nonuniform));
 const fieldBefore=await hash('#field');await page.locator('#field-time').fill('12');await page.locator('#field-time').dispatchEvent('input');const fieldAfter=await hash('#field');if(fieldBefore===fieldAfter)throw Error('Field frame does not update');
 await page.locator('#normalization').selectOption('2');const nonlinear=await hash('#field');if(nonlinear===fieldAfter)throw Error('Normalization does not update');
 await page.locator('#volume').evaluate(el=>el.closest('section').hidden=false);
 const volumeBefore=await hash('#volume');const box=await page.locator('#volume').boundingBox();await page.mouse.move(box.x+box.width*.5,box.y+box.height*.5);await page.mouse.down();await page.mouse.move(box.x+box.width*.7,box.y+box.height*.6,{steps:8});await page.mouse.up();const volumeAfter=await hash('#volume');if(volumeBefore===volumeAfter)throw Error('Volume cannot rotate');
 await page.locator('#volume-time').fill('20');await page.locator('#volume-time').dispatchEvent('input');const changedVolume=await hash('#volume');if(changedVolume===volumeAfter)throw Error('Volume frame does not update');
 await page.locator('#play-field').click();await page.waitForTimeout(350);await page.locator('#play-field').click();if(await page.locator('#field-time').inputValue()==='12')throw Error('Play does not animate');
 const t=await page.evaluate(()=>{const v=window.rustplotlib;const times=[];for(let i=0;i<30;i++){const t=performance.now();v.field.draw();times.push(performance.now()-t);}return times;});
 await page.locator('#reset-camera').click();await page.locator('#volume').evaluate(el=>el.closest('section').hidden=true);await page.screenshot({path:'validation/desktop.png',fullPage:true});await page.setViewportSize({width:390,height:844});await page.evaluate(()=>{window.rustplotlib.field.draw();window.rustplotlib.volume.draw();});await page.waitForTimeout(200);await page.screenshot({path:'validation/mobile.png',fullPage:true});
 await page.locator('#volume').evaluate(el=>el.closest('section').hidden=false);
 const downloadPromise=page.waitForEvent('download');await page.locator('#save-volume').click();const download=await downloadPromise;await download.saveAs('validation/volume-download.png');
 if(errors.length)throw Error(errors.join('\n'));
 await writeFile('validation/web.json',JSON.stringify({passed:true,errors,nonuniform,fieldBefore,fieldAfter,nonlinear,volumeBefore,volumeAfter,changedVolume,drawSubmissionMs:t,renderer:await page.locator('#status').textContent()},null,2));
 await page.setViewportSize({width:1280,height:1000});await page.goto('http://127.0.0.1:8791/gallery/index.html');
 await page.evaluate(()=>document.querySelectorAll('img').forEach(img=>img.loading='eager'));
 await page.waitForFunction(()=>Array.from(document.querySelectorAll('img')).every(img=>img.complete&&img.naturalWidth>0));
 const cards=await page.locator('main article').count();if(cards!==21)throw Error('Gallery must contain twenty-one examples');
 const hrefs=await page.locator('a').evaluateAll(links=>links.map(a=>a.href).filter(url=>url.startsWith(location.origin)&&/\.(pdf|png|svg)$/.test(url)));
 for(const url of hrefs){const response=await page.request.get(url);if(!response.ok())throw Error('Broken gallery download: '+url);}
 await page.screenshot({path:'validation/gallery-desktop.png',fullPage:true});await page.setViewportSize({width:390,height:844});
 if(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth))throw Error('Mobile gallery overflows horizontally');
 await page.screenshot({path:'validation/gallery-mobile.png',fullPage:true});
 await writeFile('validation/gallery-web.json',JSON.stringify({passed:true,cards,downloads:hrefs.length,errors},null,2));
 console.log('WebGL2 field, normalization, volume rotation, frame playback, PNG download and mobile layout passed');
}finally{await browser.close();}
