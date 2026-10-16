//! Native rustplotlib WebLayout frames over live WebGL data, with LaTeX glyph atlas.
use anyhow::Result;
use rustplotlib::*;
fn main()->Result<()>{
 let out=std::env::args().nth(1).unwrap();std::fs::create_dir_all(&out)?;
 for (name,width,height,pw,ph,xlabel,ylabel,label,cmap) in [
 ("ns",640.,390.,540.,235.,r"Latitude $[{}^\circ\mathrm{N}]$",r"Altitude $[\mathrm{km}]$",r"$\log_{10}(n_e/\mathrm{m}^{-3})$",ColorMap::ember()),
 ("ew",640.,390.,540.,235.,r"Longitude $[{}^\circ\mathrm{E}]$",r"Altitude $[\mathrm{km}]$",r"$\log_{10}(n_e/\mathrm{m}^{-3})$",ColorMap::ember()),
 ("vtec",640.,390.,540.,235.,r"Longitude $[{}^\circ\mathrm{E}]$",r"Latitude $[{}^\circ\mathrm{N}]$",r"Vertical TEC $[\mathrm{TECU}]$",ColorMap::ember())]{
  for mode in ["density","ratio","coverage"]{
   if name=="vtec"&&mode!="density"{continue;}
   let mut f=WebLayout::new(width,18.)?.figure(height)?;let mm=25.4/96.;
   let mut ax=Axes::new([0.,1.],[0.,1.],xlabel,ylabel);ax.xticks=vec![];ax.yticks=vec![];
   f.panels.push(Panel::new(Rect::new(80.*mm,20.*mm,pw*mm,ph*mm),ax));
   let sc=f.scale(Scale::new([0.,1.],Normalization::Linear,if mode=="ratio"{ColorMap::diverging()}else{cmap.clone()})?);
   let text=if mode=="ratio"{r"$\log_{10}(n_e/n_{e,\mathrm{IRI}})$"}else if mode=="coverage"{"Ray quadrature samples"}else{label};
   let mut bar=Colorbar::new(Rect::new(80.*mm,(height-80.)*mm,pw*mm,8.*mm),sc,text,vec![]);bar.horizontal=true;f.colorbars.push(bar);
   let file=format!("{out}/{name}-{mode}.svg");f.save_for_web(&file,&file,"Interactive tomography scientific frame")?;
   let svg=std::fs::read_to_string(&file)?.replace("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>","");std::fs::write(file,svg)?;
  }
 }
 let mut latex=Latex::default();latex.font_pt=18.*72.27/96.;
 let strings:Vec<String>="0123456789.-".chars().map(|c|format!("${c}$")).collect();let atlas=latex.typeset(strings.clone())?;
 let mut entries=serde_json::Map::new();for (key,g) in atlas{let character=key.trim_matches('$');entries.insert(character.into(),serde_json::json!({"body":g.body,"view_box":g.view_box,"width":g.width_pt*96./72.27,"height":g.height_pt*96./72.27}));}
 std::fs::write(format!("{out}/digits.json"),serde_json::to_vec(&entries)?)?;Ok(())
}
