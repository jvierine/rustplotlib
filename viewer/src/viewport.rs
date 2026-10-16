//! Shared normalized plot navigation for WebGL fields and curve axes.
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub struct PlotViewport { bounds: [f64;4] }
#[wasm_bindgen]
impl PlotViewport {
 #[wasm_bindgen(constructor)] pub fn new()->Self {Self{bounds:[0.,0.,1.,1.]}}
 pub fn bounds(&self)->Vec<f64>{self.bounds.to_vec()}
 pub fn reset(&mut self){self.bounds=[0.,0.,1.,1.]}
 /// Zoom around pointer coordinates measured from the lower-left of the axes.
 pub fn zoom(&mut self,x:f64,y:f64,factor:f64){
  if ![x,y,factor].iter().all(|v|v.is_finite())||factor<=0.{return}
  for (a,p) in [(0,x.clamp(0.,1.)),(1,y.clamp(0.,1.))]{
   let span=self.bounds[a+2]-self.bounds[a];let size=(span*factor).clamp(0.001,1.);
   let anchor=self.bounds[a]+p*span;let lo=(anchor-p*size).clamp(0.,1.-size);
   self.bounds[a]=lo;self.bounds[a+2]=lo+size;
  }
 }
 /// Pan by fractions of the currently visible extent.
 pub fn pan(&mut self,dx:f64,dy:f64){
  if ![dx,dy].iter().all(|v|v.is_finite()){return}
  for (a,d) in [(0,dx),(1,dy)]{let size=self.bounds[a+2]-self.bounds[a];let lo=(self.bounds[a]+d*size).clamp(0.,1.-size);self.bounds[a]=lo;self.bounds[a+2]=lo+size;}
 }
}
#[cfg(test)] mod tests {use super::*;
 #[test] fn anchored_zoom_pan_reset(){let mut v=PlotViewport::new();v.zoom(0.25,0.75,0.5);assert_eq!(v.bounds,[0.125,0.375,0.625,0.875]);v.pan(99.,-99.);assert_eq!(v.bounds,[0.5,0.,1.,0.5]);v.reset();assert_eq!(v.bounds,[0.,0.,1.,1.]);}
 #[test] fn invalid_zoom_ignored(){let mut v=PlotViewport::new();v.zoom(f64::NAN,0.,0.5);v.zoom(0.,0.,-1.);assert_eq!(v.bounds,[0.,0.,1.,1.]);}
}
