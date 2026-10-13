import {createServer} from 'node:http';
import {readFile} from 'node:fs/promises';
import path from 'node:path';
const root=path.resolve('web'),port=Number(process.env.PORT||8791);
const types={'.html':'text/html','.js':'text/javascript','.wasm':'application/wasm','.svg':'image/svg+xml','.json':'application/json','.pdf':'application/pdf','.png':'image/png','.jpg':'image/jpeg'};
createServer(async(req,res)=>{try{const url=new URL(req.url,'http://localhost'),file=path.resolve(root,'.'+(url.pathname==='/'?'/index.html':decodeURIComponent(url.pathname)));if(!file.startsWith(root+path.sep))throw Error('Invalid path');const data=await readFile(file);res.writeHead(200,{'Content-Type':types[path.extname(file)]||'application/octet-stream'});res.end(data);}catch{res.writeHead(404);res.end('Not found');}}).listen(port,'127.0.0.1',()=>console.log(`http://127.0.0.1:${port}`));
