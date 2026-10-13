// Display-only NESTED HEALPix geometry and a single ring of neighbour means.
// Face projection follows the HEALPix jrll/jpll face-coordinate convention.
const rings=[2,2,2,2,3,3,3,3,4,4,4,4],phases=[1,3,5,7,0,2,4,6,1,3,5,7];
const geometries=new Map();
export function nestedGeometry(nside){
 if(!Number.isInteger(nside)||nside<1||(nside&(nside-1)))throw Error('Invalid HEALPix nside');
 if(geometries.has(nside))return geometries.get(nside);
 const capacity=12*nside*nside,coords=new Float64Array(capacity*8),vertices=new Map(),adjacent=Array.from({length:capacity},()=>new Set());
 for(let pixel=0;pixel<capacity;pixel++){
  const face=Math.floor(pixel/(nside*nside));let bits=pixel%(nside*nside),x=0,y=0;
  for(let scale=1;scale<nside;scale*=2){x+=(bits&1)*scale;y+=((bits>>1)&1)*scale;bits>>=2;}
  for(const [i,dx,dy]of [[0,1,1],[1,0,1],[2,0,0],[3,1,0]]){
   const jr=rings[face]*nside-x-dx-y-dy,nr=Math.min(nside,jr,4*nside-jr);
   const z=jr<nside?1-nr*nr/(3*nside*nside):jr>3*nside?-1+nr*nr/(3*nside*nside):(2*nside-jr)*2/(3*nside);
   const lon=nr===0?0:((phases[face]*nr+x+dx-y-dy)*45/nr+360)%360,lat=Math.asin(Math.max(-1,Math.min(1,z)))*180/Math.PI;
   coords[pixel*8+i*2]=lon;coords[pixel*8+i*2+1]=lat;
   const key=nr===0?'pole:'+Math.sign(z):jr+':'+Math.round(lon*1e8);
   const touching=vertices.get(key)||[];
   for(const other of touching){adjacent[pixel].add(other);adjacent[other].add(pixel);}touching.push(pixel);vertices.set(key,touching);
  }
 }
 const neighbours=adjacent.map(s=>Uint32Array.from(s));
 const geometry={count:capacity,capacity,pixels:Uint32Array.from({length:capacity},(_,i)=>i),neighbours,corners(pixel){return Array.from({length:4},(_,i)=>[coords[pixel*8+i*2],coords[pixel*8+i*2+1]]);}};
 geometries.set(nside,geometry);return geometry;
}
export function fillNeighbours(cells,valueAt,geometry){
 const values=new Map();for(const cell of cells){const v=valueAt(cell);if(v!=null&&Number.isFinite(v))values.set(cell.pixel,v);}
 const sums=new Map();
 for(const [pixel,value]of values)for(const neighbour of geometry.neighbours[pixel]||[]){
  if(values.has(neighbour))continue;const total=sums.get(neighbour)||[0,0];total[0]+=value;total[1]++;sums.set(neighbour,total);
 }
 return [...values].map(([pixel,value])=>({pixel,value})).concat([...sums].map(([pixel,[sum,count]])=>({pixel,value:sum/count})));
}
export function createNeighbourRenderer(){
 let previousCells,previousGeometry,previousField,result;
 return (cells,geometry,field,valueAt)=>{
  if(!geometry?.capacity)return null;
  if(cells===previousCells&&geometry===previousGeometry&&field===previousField)return result;
  const nside=Math.sqrt(geometry.capacity/12);if(!Number.isInteger(nside))return null;
  const full=nestedGeometry(nside);
  result={geometry:full,cells:fillNeighbours(cells,valueAt,full)};
  previousCells=cells;previousGeometry=geometry;previousField=field;return result;
 };
}
