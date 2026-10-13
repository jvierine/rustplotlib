import {mkdir,writeFile,readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
const root='gallery/heimdall',base='https://pikos.org/heimdall/';
await mkdir(root,{recursive:true});
const sources=[];
async function get(url,file){const r=await fetch(url);if(!r.ok)throw Error(`${r.status}: ${url}`);const data=Buffer.from(await r.arrayBuffer());await writeFile(`${root}/${file}`,data);sources.push({url,file,sha256:createHash('sha256').update(data).digest('hex')});return data;}
const meta=JSON.parse(await get(base+'pair-overlays/flow-grid-2025-09-25/latest.json','metadata.json'));
await get(base+'pair-overlays/flow-grid-2025-09-25/'+meta.geometry,'geometry.bin');
for(const minute of [720,735,750,765,779,795,810,825,840])await get(base+`pair-overlays/flow-grid-2025-09-25/${meta.generation}/${minute}.bin`,`${minute}.bin`);
await get(base+'api/v1/magnetic/2025.json','magnetic.json');
await writeFile(root+'/provenance.json',JSON.stringify({date:meta.date,generation:meta.generation,minutes:[720,735,750,765,779,795,810,825,840],requestedMinute:779,sources},null,2));
