//! Built-in colormap reference, low values at left and high values at right.
use anyhow::Result;
use rustplotlib::*;

fn main() -> Result<()> {
    let maps = [
        ("storm", ColorMap::storm()),
        ("turbo", ColorMap::turbo()),
        ("plasma", ColorMap::plasma()),
        ("inferno", ColorMap::inferno()),
        (r"nipy\_spectral", ColorMap::nipy_spectral()),
        ("diverging", ColorMap::diverging()),
        ("seismic", ColorMap::seismic()),
        ("sequential", ColorMap::sequential()),
        ("ember", ColorMap::ember()),
        ("estuary", ColorMap::estuary()),
    ];
    let mut figure = Figure::new(IntendedUse::ArticleWide);
    figure.height_mm = 142.;
    for (i, (name, colors)) in maps.into_iter().enumerate() {
        let y = 8. + i as f64 * 12.;
        let scale = figure.scale(Scale::new([0., 1.], Normalization::Linear, colors)?);
        let mut bar = Colorbar::new(Rect::new(44., y, 124., 6.), scale, "", vec![]);
        bar.horizontal = true;
        if i == 9 {
            bar.ticks = vec![0., 0.5, 1.];
            bar.label = "Normalized value".into();
        }
        figure.colorbars.push(bar);
        figure
            .labels
            .push(Label::new(format!(r"\texttt{{{name}}}"), 23., y + 3.));
    }
    for ext in ["svg", "pdf", "png", "jpg"] {
        figure.save(format!("gallery/colormaps.{ext}"))?;
    }
    Ok(())
}
