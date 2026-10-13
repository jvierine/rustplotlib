import {chromium} from 'playwright';
import {writeFile,mkdir} from 'node:fs/promises';
await mkdir('gallery',{recursive:true});
const browser=await chromium.launch({executablePath:'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',args:['--use-gl=angle','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
try {
 const page=await browser.newPage({viewport:{width:1200,height:1000}});const errors=[];page.on('pageerror',e=>errors.push(String(e)));
 await page.goto('http://127.0.0.1:8791/heimdall.html');await page.waitForFunction(()=>window.heimdallExample?.ready);
 const data=await page.evaluate(()=>window.heimdallExample.scientificScene().toDataURL('image/png'));
 await mkdir('output',{recursive:true});await writeFile('output/heimdall-canvas.png',Buffer.from(data.split(',')[1],'base64'));
 const provenance=await page.evaluate(()=>window.heimdallExample.provenance);
 const palette=await page.evaluate(()=>window.heimdallExample.palette);
 await writeFile('output/heimdall-palette.json',JSON.stringify(palette));
 await writeFile('gallery/heimdall-scientific.json',JSON.stringify({script:'scripts/export_heimdall_scientific.mjs',figure:'examples/scientific_views.rs',renderer:'HEIMDALL WebGL2 + native LaTeX typography',width_mm:85,font_pt:10,provenance},null,2));
 if(errors.length)throw Error(errors.join('\n'));console.log('HEIMDALL scene extracted without page UI; typeset with examples/scientific_views.rs');
} finally {await browser.close();}
