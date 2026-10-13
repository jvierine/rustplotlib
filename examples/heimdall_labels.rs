use anyhow::Result;
use rustplotlib::*;
fn main() -> Result<()> {
    let mut f = Figure::new(IntendedUse::Mobile);
    f.width_mm = 100.;
    f.height_mm = 22.;
    let colors = include_bytes!("support/heimdall_rainbow.rgb")
        .chunks_exact(3)
        .map(|c| [c[0] as f64 / 255., c[1] as f64 / 255., c[2] as f64 / 255.])
        .collect();
    let sc = f.scale(Scale::new(
        [1., 10f64.powf(1.6)],
        Normalization::Log,
        ColorMap::new(colors)?,
    )?);
    let mut bar = Colorbar::new(
        Rect::new(6., 3., 84., 2.),
        sc,
        r"Vertical TEC $[\mathrm{TECU}]$",
        vec![1., 2., 5., 10., 20., 10f64.powf(1.6)],
    );
    bar.horizontal = true;
    f.colorbars.push(bar);
    f.save("web/data/heimdall_colorbar.svg")?;
    Ok(())
}
