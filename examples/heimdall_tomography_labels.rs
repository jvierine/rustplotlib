//! Live-figure labels exported through rustplotlib's CSS-sized web SVG contract.
use anyhow::Result;
use rustplotlib::*;
fn main()->Result<()>{
 let dir=std::env::args().nth(1).unwrap_or("output/heimdall-tomography".into());std::fs::create_dir_all(&dir)?;
 for (name,text) in [
 ("density",r"$\log_{10}(n_e/\mathrm{m}^{-3})$"),
 ("ratio",r"$\log_{10}(n_e/n_{e,\mathrm{IRI}})$"),
 ("coverage",r"Ray quadrature samples"),
 ("vtec",r"Vertical TEC $[\mathrm{TECU}]$"),
 ("latitude",r"Latitude $[{}^\circ\mathrm{N}]$"),
 ("longitude",r"Longitude $[{}^\circ\mathrm{E}]$"),
 ("altitude",r"Altitude $[\mathrm{km}]$")]{
  let mut f=WebLayout::new(320.,18.)?.figure(36.)?;
  f.labels.push(Label::new(text,f.width_mm/2.,6.));
  f.save_for_web(format!("{dir}/{name}.svg"),&format!("{name}.svg"),text)?;
 }
 Ok(())
}
