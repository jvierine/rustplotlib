// Decode display data only. No fitting, mapping, or averaging in JavaScript.
export function decodeGeometryF16(buffer){
 const view=new DataView(buffer);
 if(buffer.byteLength<12||view.getUint32(0,true)!==0x31464748)throw Error('Invalid float16 geometry magic');
 const count=view.getUint32(4,true),capacity=view.getUint32(8,true);
 if(capacity>1e7||count>capacity||buffer.byteLength!==12+count*20)throw Error('Invalid geometry size');
 const offsets=new Int32Array(capacity);offsets.fill(-1);
 for(let i=0;i<count;i++){const offset=12+i*20,pixel=view.getUint32(offset,true);if(pixel>=capacity||offsets[pixel]!==-1)throw Error('Invalid geometry pixel');offsets[pixel]=offset+4;}
 const half=offset=>{const bits=view.getUint16(offset,true),sign=bits&32768?-1:1,e=(bits>>10)&31,m=bits&1023;return e===31?NaN:sign*(e?(1+m/1024)*2**(e-15):m*2**-24);};
 const pixels=Uint32Array.from({length:count},(_,i)=>view.getUint32(12+i*20,true));
 return {count,capacity,pixels,corners(pixel){const offset=offsets[pixel];if(offset===undefined||offset<0)return null;return Array.from({length:4},(_,i)=>[half(offset+i*4),half(offset+i*4+2)]);}};
}

export function decodeMapF16(buffer){
 const view=new DataView(buffer);
 if(buffer.byteLength<8||view.getUint32(0,true)!==0x314d4748)throw Error('Invalid float16 map magic');
 const length=view.getUint32(4,true),start=8+length;
 if(start>buffer.byteLength)throw Error('Truncated map metadata');
 const meta=JSON.parse(new TextDecoder().decode(new Uint8Array(buffer,8,length)));
 const n=meta.count,limit=meta.pixel_count??49152;
 if(meta.schema!=='heimdall.map-f16/v1'||!Number.isSafeInteger(limit)||limit<1||limit>1e7||!Number.isSafeInteger(n)||n<0||n>limit||buffer.byteLength!==start+n*14)throw Error('Invalid map frame size');
 const half=offset=>{const bits=view.getUint16(offset,true),sign=bits&32768?-1:1,e=(bits>>10)&31,m=bits&1023;return e===31?(m?NaN:sign*Infinity):sign*(e?(1+m/1024)*2**(e-15):m*2**-24);};
 const cells=new Array(n);
 for(let i=0;i<n;i++){
  const pixel=view.getUint32(start+i*4,true),offset=start+n*4+i*10;
  if(pixel>=limit)throw Error('Invalid map pixel');
  const values=[half(offset),half(offset+2),half(offset+4),half(offset+6)];
  cells[i]={pixel,values,horizontal_gradient_magnitude:half(offset+8)};
 }
 return {...meta,cells};
}

export function decodeDtecF16(buffer){
 const view=new DataView(buffer);
 if(buffer.byteLength<8||view.getUint32(0,true)!==0x31544448)throw Error('Invalid dTEC float16 map magic');
 const length=view.getUint32(4,true),start=8+length;
 if(start>buffer.byteLength)throw Error('Truncated dTEC map metadata');
 const meta=JSON.parse(new TextDecoder().decode(new Uint8Array(buffer,8,length)));
 const n=meta.count,limit=meta.pixel_count??49152;
 const v2=meta.schema==='heimdall.dtec-f16/v2',stride=v2?10:4;
 if((!v2&&meta.schema!=='heimdall.dtec-f16/v1')||!Number.isSafeInteger(limit)||limit<1||limit>1e7||!Number.isSafeInteger(n)||n<0||n>limit||buffer.byteLength!==start+n*(4+stride))throw Error('Invalid dTEC map frame size');
 const half=offset=>{const bits=view.getUint16(offset,true),sign=bits&32768?-1:1,e=(bits>>10)&31,m=bits&1023;return e===31?(m?NaN:sign*Infinity):sign*(e?(1+m/1024)*2**(e-15):m*2**-24);};
 const cells=new Array(n);
 for(let i=0;i<n;i++){
  const pixel=view.getUint32(start+i*4,true),offset=start+n*4+i*stride;
  if(pixel>=limit)throw Error('Invalid dTEC map pixel');
  cells[i]={pixel,values:v2?[half(offset),half(offset+2),half(offset+4),view.getUint32(offset+6,true)]:[half(offset),half(offset+2)]};
 }
 return {...meta,cells};
}
