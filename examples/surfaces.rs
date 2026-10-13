use anyhow::Result;
use rustplotlib::*;
fn main() -> Result<()> {
    let mut f = Figure::new(IntendedUse::ArticleWide);
    f.height_mm = 100.0;
    let nodes: Vec<_> = (0..=160).map(|i| -3.0 + 6.0 * i as f64 / 160.0).collect();
    for (k, (title, formula, zlim)) in [
        ("Gaussian peak", r"$z=\exp[-(x^2+y^2)/2]$", [0.0, 1.0]),
        ("Saddle", r"$z=(x^2-y^2)/9$", [-1.0, 1.0]),
        ("Radial wave", r"$z=\sin(3r)/(1+r)$", [-0.5, 0.7]),
    ]
    .into_iter()
    .enumerate()
    {
        let mut values = vec![];
        for &y in &nodes {
            for &x in &nodes {
                values.push(match k {
                    0 => (-(x * x + y * y) / 2.0).exp(),
                    1 => (x * x - y * y) / 9.0,
                    _ => {
                        let r = x.hypot(y);
                        (3.0 * r).sin() / (1.0 + r)
                    }
                });
            }
        }
        let sc = f.scale(Scale::new(
            zlim,
            Normalization::Linear,
            if k == 0 {
                ColorMap::ember()
            } else {
                ColorMap::diverging()
            },
        )?);
        let surf = Surface::new(nodes.clone(), nodes.clone(), values, zlim, sc)?;
        let rect = Rect::new(3.0 + 59.0 * k as f64, 9.0, 55.0, 52.0);
        let mut p = Panel::new(rect, Axes::new([0., 1.], [0., 1.], "", ""));
        p.axes = None;
        // Lines underneath the surface are depth-occluded by the raster layer.
        for (a, b, label) in [
            ([-3., -3., zlim[0]], [3., -3., zlim[0]], "$x$"),
            ([-3., -3., zlim[0]], [-3., 3., zlim[0]], "$y$"),
            ([-3., -3., zlim[0]], [-3., -3., zlim[1]], "$z$"),
        ] {
            let aa = surf.project(a);
            let bb = surf.project(b);
            p.push(Layer::Line(Line::new(
                vec![[aa[0], 1. - aa[1]], [bb[0], 1. - bb[1]]],
                "#657078",
            )));
            p.labels.push(Label::new(
                label,
                rect.x + rect.width * bb[0] + 2.0,
                rect.y + rect.height * bb[1],
            ));
        }
        p.push(Layer::Surface(surf));
        f.panels.push(p);
        f.labels
            .push(Label::new(title, rect.x + rect.width / 2.0, 4.0));
        f.labels
            .push(Label::new(formula, rect.x + rect.width / 2.0, 83.0));
        let mut bar = Colorbar::new(
            Rect::new(rect.x + 6.0, 90.0, 43.0, 1.5),
            sc,
            "",
            vec![zlim[0], zlim[1]],
        );
        bar.horizontal = true;
        f.colorbars.push(bar);
    }
    for ext in ["pdf", "png", "svg", "jpg"] {
        f.save(format!("gallery/surfaces.{ext}"))?;
    }
    Ok(())
}
