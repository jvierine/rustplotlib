use anyhow::Result;
use rustplotlib::*;

fn main() -> Result<()> {
    let mut f = Figure::new(IntendedUse::ArticleWide);
    f.width_mm = 180.;
    f.height_mm = 85.;
    for (i, background) in ["#ffffff", "#24152f"].iter().enumerate() {
        let mut p = Panel::new(
            Rect::new(18. + 86. * i as f64, 10., 66., 56.),
            Axes::new([0., 4.], [0., 3.], "Time", "Height"),
        );
        let mut bg = Line::new(
            vec![[0., 0.], [4., 0.], [4., 3.], [0., 3.], [0., 0.]],
            *background,
        );
        bg.fill = Some((*background).into());
        bg.width_pt = 0.;
        bg.protect_from_legend = false;
        p.push(Layer::Line(bg));
        let mut legend = p.marker_groups(&[
            MarkerGroup {
                label: "A",
                points: &[[1., 1.5], [2., 2.], [3., 1.5]],
                style: MarkerStyle {
                    shape: MarkerShape::Circle,
                    size_pt: 9.,
                    color: [0., 0.45, 0.7],
                },
            },
            MarkerGroup {
                label: "B",
                points: &[[1., 1.5], [2.08, 2.03], [3.5, 1.5]],
                style: MarkerStyle {
                    shape: MarkerShape::Diamond,
                    size_pt: 9.,
                    color: [0.84, 0.37, 0.],
                },
            },
        ])?;
        legend.position = LegendPosition::LowerLeft;
        p.legend = Some(legend);
        f.panels.push(p);
    }
    for ext in ["svg", "png", "pdf"] {
        f.save(format!("target/overlapping-markers.{ext}"))?;
    }
    Ok(())
}
