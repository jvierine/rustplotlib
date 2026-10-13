// Geometry is uploaded once per publication; only scalar values change per frame.
export class MapGpu {
 constructor(){
  this.canvas=document.createElement('canvas');
  const gl=this.gl=this.canvas.getContext('webgl2',{alpha:true,antialias:true,premultipliedAlpha:true,preserveDrawingBuffer:true});
  if(!gl)throw Error('WebGL2 unavailable');
  const shader=(type,source)=>{const s=gl.createShader(type);gl.shaderSource(s,source);gl.compileShader(s);if(!gl.getShaderParameter(s,gl.COMPILE_STATUS))throw Error(gl.getShaderInfoLog(s));return s;};
  const vertex=shader(gl.VERTEX_SHADER,`#version 300 es
   precision highp float;
   in vec3 position; in vec2 lonlat; in float pixel;
   uniform vec3 rightAxis,upAxis,frontAxis;
   uniform vec2 radius,mapOffset;
   uniform float mapZoom,shift;
   uniform bool globe;
   flat out int cell; out float depth;
   void main(){
    cell=int(pixel);depth=1.0;
    if(globe){depth=dot(position,frontAxis);gl_Position=vec4(dot(position,rightAxis)*radius.x,dot(position,upAxis)*radius.y,0.0,1.0);}
    else gl_Position=vec4((lonlat.x+shift)/180.0*mapZoom+2.0*mapOffset.x,lonlat.y/90.0*mapZoom-2.0*mapOffset.y,0.0,1.0);
   }`);
  const fragment=shader(gl.FRAGMENT_SHADER,`#version 300 es
   precision highp float;
   uniform highp sampler2D values,previousValues,palette;
   uniform float blend;
   uniform vec2 limits;
   uniform bool signedScale,logScale;
   flat in int cell; in float depth; out vec4 color;
   vec3 hsl(float h,float s,float l){vec3 k=mod(vec3(0.0,8.0,4.0)+h/30.0,12.0);return l-s*min(l,1.0-l)*clamp(min(k-3.0,9.0-k),-1.0,1.0);}
   void main(){
    if(depth<0.0)discard;
    float v=texelFetch(values,ivec2(cell%256,cell/256),0).r;
    if(blend<1.0){float previous=texelFetch(previousValues,ivec2(cell%256,cell/256),0).r;if(blend<=0.0)v=previous;else{if(isnan(previous)||isinf(previous)||isnan(v)||isinf(v))discard;v=mix(previous,v,blend);}}
    if(isnan(v)||isinf(v)||(logScale&&v<=0.0))discard;
    if(logScale)v=log(v)/log(10.0);
    float t=clamp((v-limits.x)/max(1e-20,limits.y-limits.x),0.0,1.0);
    color=vec4(texture(palette,vec2((t*255.0+.5)/256.0,.5)).rgb,1.0);
   }`);
  const program=this.program=gl.createProgram();gl.attachShader(program,vertex);gl.attachShader(program,fragment);gl.linkProgram(program);
  if(!gl.getProgramParameter(program,gl.LINK_STATUS))throw Error(gl.getProgramInfoLog(program));
  gl.deleteShader(vertex);gl.deleteShader(fragment);gl.useProgram(program);
  this.paletteUniform=gl.getUniformLocation(program,'palette');
  this.paletteTexture=gl.createTexture();
  this.uniforms=Object.fromEntries(['rightAxis','upAxis','frontAxis','radius','mapOffset','mapZoom','shift','globe','limits','signedScale','logScale','values','previousValues','blend'].map(name=>[name,gl.getUniformLocation(program,name)]));
  this.vao=gl.createVertexArray();gl.bindVertexArray(this.vao);
  this.vertices=gl.createBuffer();gl.bindBuffer(gl.ARRAY_BUFFER,this.vertices);
  for(const [name,size,offset]of [['position',3,0],['lonlat',2,12],['pixel',1,20]]){const a=gl.getAttribLocation(program,name);gl.enableVertexAttribArray(a);gl.vertexAttribPointer(a,size,gl.FLOAT,false,24,offset);}
  this.indices=gl.createBuffer();gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,this.indices);
  this.texture=gl.createTexture();this.previousTexture=gl.createTexture();
  for(const texture of [this.texture,this.previousTexture]){gl.bindTexture(gl.TEXTURE_2D,texture);
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.NEAREST);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.CLAMP_TO_EDGE);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.CLAMP_TO_EDGE);
  }
 }
 geometry(geometry){
  if(this.currentGeometry===geometry)return;
  const gl=this.gl,vertices=new Float32Array(geometry.count*24),indices=new Uint32Array(geometry.count*6),r=Math.PI/180;
  geometry.pixels.forEach((pixel,i)=>{
   const corners=geometry.corners(pixel),ref=corners[0][0];
   corners.forEach(([lon,lat],j)=>{const p=(i*4+j)*6,c=Math.cos(lat*r);vertices.set([c*Math.cos(lon*r),c*Math.sin(lon*r),Math.sin(lat*r),ref+((lon-ref+540)%360-180),lat,pixel],p);});
   indices.set([i*4,i*4+1,i*4+2,i*4,i*4+2,i*4+3],i*6);
  });
  gl.bindBuffer(gl.ARRAY_BUFFER,this.vertices);gl.bufferData(gl.ARRAY_BUFFER,vertices,gl.STATIC_DRAW);
  gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,this.indices);gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,indices,gl.STATIC_DRAW);
  this.count=indices.length;this.height=Math.ceil(geometry.capacity/256);this.values=new Float32Array(this.height*256);
  for(const texture of [this.texture,this.previousTexture]){gl.bindTexture(gl.TEXTURE_2D,texture);gl.texImage2D(gl.TEXTURE_2D,0,gl.R32F,256,this.height,0,gl.RED,gl.FLOAT,null);}
  this.currentGeometry=geometry;this.currentCells=null;
 }
 draw(ctx,geometry,cells,valueAt,options){
  const gl=this.gl,u=this.uniforms;
  if(gl.isContextLost()||!geometry.pixels)return false;
  gl.useProgram(this.program);gl.bindVertexArray(this.vao);this.geometry(geometry);
  const {width,height}=ctx.canvas;
  if(this.canvas.width!==width)this.canvas.width=width;if(this.canvas.height!==height)this.canvas.height=height;
  gl.viewport(0,0,width,height);gl.clearColor(0,0,0,0);gl.clear(gl.COLOR_BUFFER_BIT);
  gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,this.texture);
  if(this.currentCells!==cells||this.currentField!==options.field){
   [this.texture,this.previousTexture]=[this.previousTexture,this.texture];gl.bindTexture(gl.TEXTURE_2D,this.texture);
   this.values.fill(NaN);for(const cell of cells){const value=valueAt(cell);this.values[cell.pixel]=value==null?NaN:value;}
   gl.texSubImage2D(gl.TEXTURE_2D,0,0,0,256,this.height,gl.RED,gl.FLOAT,this.values);
   this.currentCells=cells;this.currentField=options.field;
  }
  gl.activeTexture(gl.TEXTURE1);gl.bindTexture(gl.TEXTURE_2D,this.previousTexture);gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,this.texture);
  gl.uniform1i(u.previousValues,1);gl.uniform1f(u.blend,options.blend??1);
  gl.activeTexture(gl.TEXTURE2);gl.bindTexture(gl.TEXTURE_2D,this.paletteTexture);
  if(this.currentPalette!==options.palette){
   gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
   gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.CLAMP_TO_EDGE);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.CLAMP_TO_EDGE);
   gl.texImage2D(gl.TEXTURE_2D,0,gl.RGB8,256,1,0,gl.RGB,gl.UNSIGNED_BYTE,options.palette);this.currentPalette=options.palette;
  }
  gl.uniform1i(this.paletteUniform,2);gl.activeTexture(gl.TEXTURE0);
  gl.uniform1i(u.values,0);gl.uniform1i(u.globe,options.globe);gl.uniform1i(u.signedScale,options.signed);gl.uniform1i(u.logScale,options.log);
  gl.uniform2fv(u.limits,options.limits);gl.uniform2f(u.radius,2*options.radius/width,2*options.radius/height);
  const camera=options.camera||{right:[1,0,0],up:[0,1,0],front:[0,0,1]};
  gl.uniform3fv(u.rightAxis,camera.right);gl.uniform3fv(u.upAxis,camera.up);gl.uniform3fv(u.frontAxis,camera.front);
  gl.uniform2f(u.mapOffset,options.mapX,options.mapY);gl.uniform1f(u.mapZoom,options.mapZoom);
  for(const shift of options.globe?[0]:[-360,0,360]){gl.uniform1f(u.shift,shift);gl.drawElements(gl.TRIANGLES,this.count,gl.UNSIGNED_INT,0);}
  ctx.drawImage(this.canvas,0,0);return true;
 }
}
