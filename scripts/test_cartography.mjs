import {chromium} from 'playwright';
import {mkdir,writeFile} from 'node:fs/promises';
const browser=await chromium.launch({executablePath:process.platform==='darwin'?'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome':chromium.executablePath(),args:['--use-gl=angle','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
await mkdir('validation',{recursive:true});const page=await browser.newPage({viewport:{width:1200,height:1000}});const errors=[];page.on('pageerror',e=>errors.push(String(e)));
try{
await page.goto('http://127.0.0.1:8791/heimdall.html');await page.waitForFunction(()=>window.heimdallExample?.ready,null,{timeout:90000});await page.waitForFunction(()=>Array.from(document.images).every(img=>img.complete&&img.naturalWidth>0));await page.screenshot({path:'gallery/heimdall.png',fullPage:true});
const before=await page.locator('#map').screenshot();await page.locator('#time').fill('6');await page.locator('#time').dispatchEvent('input');const after=await page.locator('#map').screenshot();if(before.equals(after))throw Error('HEIMDALL frame unchanged');
await page.locator('#projection').selectOption('map');await page.screenshot({path:'validation/heimdall-map.png',fullPage:true});await page.setViewportSize({width:390,height:844});if(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth))throw Error('Map mobile overflow');
await page.goto('http://127.0.0.1:8791/atmosphere.html');await page.waitForFunction(()=>window.atmosphereExample?.ready);await page.waitForFunction(()=>Array.from(document.images).every(img=>img.complete&&img.naturalWidth>0));await page.setViewportSize({width:1500,height:850});await page.evaluate(()=>window.atmosphereExample.views.forEach(v=>v.draw()));await page.screenshot({path:'gallery/atmosphere.png',fullPage:true});
if(errors.length)throw Error(errors.join('\n'));await writeFile('validation/cartography.json',JSON.stringify({passed:true,errors,realFrameChanged:!before.equals(after)},null,2));console.log('HEIMDALL frame playback, projections, mobile and atmospheric shells passed');
}finally{await browser.close();}
