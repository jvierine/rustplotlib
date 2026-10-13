import {chromium} from 'playwright';
import {mkdir} from 'node:fs/promises';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import {existsSync} from 'node:fs';
await mkdir('output/volume-frames',{recursive:true});
const native='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const browser=await chromium.launch({executablePath:process.env.CHROME||(existsSync(native)?native:chromium.executablePath()),args:['--enable-webgl','--use-gl=angle','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
const page=await browser.newPage({viewport:{width:940,height:950},deviceScaleFactor:1});const errors=[];page.on('pageerror',e=>errors.push(String(e)));
try{await page.goto('http://127.0.0.1:8791/');await page.waitForFunction(()=>window.rustplotlib?.ready);await page.evaluate(()=>window.rustplotlib.volume.set_transfer(0,1,5,.08));for(let i=0;i<24;i++){await page.evaluate(i=>{document.getElementById('volume-time').value=String(i);window.rustplotlib.volumeFrame();window.rustplotlib.volume.set_camera(.45+i*.018,.75);},i);await page.locator('main>section:nth-child(2)>figure').screenshot({path:`output/volume-frames/${String(i).padStart(4,'0')}.png`});}if(errors.length)throw Error(errors.join('\n'));}finally{await browser.close();}
const ff=spawn('ffmpeg',['-v','error','-y','-framerate','8','-i','output/volume-frames/%04d.png','-filter_complex','fps=8,scale=420:-1:flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse=dither=bayer:bayer_scale=3','-loop','0','gallery/volume.gif'],{stdio:'inherit'});const [code]=await once(ff,'close');if(code)throw Error('GIF encoding failed');
