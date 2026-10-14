//! Analytic demonstration of reciprocal coordinates, not an MSIS profile.
use anyhow::Result;
use rustplotlib::*;
fn main() -> Result<()> {
    let mut f = Figure::new(IntendedUse::ArticleColumn);
    f.height_mm = 100.;
    let mut ax = Axes::new(
        [0.01, 0.035],
        [70., 150.],
        r"$N\,[\mathrm{s}^{-1}]$",
        r"Altitude $z\,[\mathrm{km}]$",
    );
    ax.xticks = vec![0.01, 0.02, 0.03];
    ax.yticks = vec![70., 90., 110., 130., 150.];
    ax.secondary_x = Some(SecondaryAxis::new(
        r"Buoyancy period [min]",
        vec![3., 4., 5., 7., 10.],
        |period| std::f64::consts::TAU / (60. * period),
    )?);
    let mut p = Panel::new(Rect::new(18., 23., 63., 65.), ax);
    p.push(Layer::Line(Line::new(
        (0..161)
            .map(|i| {
                let z = 70. + i as f64 * 0.5;
                let n = 0.017 + 0.011 * (-((z - 110.) / 23.).powi(2)).exp();
                [n, z]
            })
            .collect(),
        "#245b78",
    )));
    f.panels.push(p);
    f.save("gallery/secondary_axis.pdf")?;
    f.save("gallery/secondary_axis.png")?;
    Ok(())
}
