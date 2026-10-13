mod support {
    pub mod dimant;
}
use anyhow::Result;
use rustplotlib::*;
use std::{f64::consts::PI, fs};
use support::dimant::Density;
fn main() -> Result<()> {
    let model = Density::new(64);
    let refined = Density::new(128);
    let points = [
        [-5.0, 0.0],
        [5.0, 0.0],
        [0.0, 1.0],
        [1.0, 1.0],
        [-1.0, 1.0],
        [0.01, 0.01],
        [15.0, 8.0],
        [2.0, 0.0001],
    ];
    let checks:Vec<_>=points.iter().map(|&[z,r]|serde_json::json!({"z":z,"r":r,"rust":model.at(z,r),"refined":refined.at(z,r)})).collect();
    fs::create_dir_all("output/dimant")?;
    fs::write(
        "output/dimant/checks.json",
        serde_json::to_vec_pretty(&checks)?,
    )?;
    let (nx, ny) = (1000, 800);
    let x: Vec<_> = (0..=nx)
        .map(|i| -5.0 + 25.0 * i as f64 / nx as f64)
        .collect();
    let y: Vec<_> = (0..=ny)
        .map(|i| -10.0 + 20.0 * i as f64 / ny as f64)
        .collect();
    let mut values = vec![];
    for j in 0..ny {
        let r = (y[j] + y[j + 1]) / 2.0;
        for i in 0..nx {
            let z = (x[i] + x[i + 1]) / 2.0;
            values.push(if z.hypot(r) > 20.0 {
                f64::NAN
            } else {
                8.0 * PI * model.at(z, r)
            });
        }
    }
    fs::write(
        "output/dimant/density.f32",
        values
            .iter()
            .flat_map(|v| (*v as f32).to_le_bytes())
            .collect::<Vec<_>>(),
    )?;
    let mut f = Figure::new(IntendedUse::ArticleWide);
    f.height_mm = 122.0;
    let sc = f.scale(Scale::new(
        [1e-3, 1e4],
        Normalization::Log,
        ColorMap::listed(
            include_bytes!("support/dimant_colormap.rgb")
                .chunks_exact(3)
                .map(|rgb| {
                    [
                        rgb[0] as f64 / 255.,
                        rgb[1] as f64 / 255.,
                        rgb[2] as f64 / 255.,
                    ]
                })
                .collect(),
        )?,
    )?);
    let rect = Rect::new(15.0, 12.0, 86.0, 72.0);
    let mut axes = Axes::new(
        [-5.0, 20.0],
        [-10.0, 10.0],
        r"$z/\lambda_T$",
        r"$r/\lambda_T$",
    );
    axes.xticks = vec![-5., 0., 5., 10., 15., 20.];
    axes.yticks = vec![-10., -5., 0., 5., 10.];
    let mut p = Panel::new(rect, axes);
    p.pcolormesh(x, y, values, sc)?;
    let mut axis = Line::new(vec![[-5., 0.], [20., 0.]], "#c8d3d7");
    axis.width_pt = 0.45;
    axis.dash = Some("1.2 1.2".into());
    p.push(Layer::Line(axis));
    p.labels
        .push(Label::new(r"$z>0$: behind meteor", 73.0, 18.0));
    f.panels.push(p);
    f.labels
        .push(Label::new(r"Dimant--Oppenheim density", 58., 5.));
    let mut bar = Colorbar::new(
        Rect::new(15., 99., 86., 2.),
        sc,
        r"$\mathcal N\equiv 8\pi n_e/n_*$",
        vec![1e-3, 1e-1, 10., 10000.],
    );
    bar.horizontal = true;
    f.colorbars.push(bar);
    f.labels.push(Label::new(
        r"$\mathcal N$: dimensionless density; $n_*$: reference density scale",
        89.,
        118.,
    ));
    // Matched-height panels with golden-ratio widths; log ordinate is explicit.
    let rect = Rect::new(121., 12., 86. / 1.618033988749895, 72.);
    let mut axes = Axes::new(
        [0., 20.],
        [-3., 4.],
        r"$z/\lambda_T$",
        r"$\log_{10}\mathcal N$",
    );
    axes.xticks = vec![0., 10., 20.];
    axes.yticks = vec![-3., 0., 3.];
    let mut p = Panel::new(rect, axes);
    for (r, color, offset) in [
        (0.0, "#ad3b3f", 0.0),
        (1.0, "#245774", 7.0),
        (5.0, "#718447", 14.0),
    ] {
        let points = (1..=1000)
            .map(|i| {
                let z = i as f64 / 50.0;
                [z, (8.0 * PI * model.at(z, r)).log10()]
            })
            .collect();
        p.push(Layer::Line(Line::new(points, color)));
        let mut label = Label::new(format!(r"$r/\lambda_T={r}$"), 148., 18. + offset);
        label.color = color.into();
        p.labels.push(label);
    }
    f.panels.push(p);
    for ext in ["pdf", "png", "jpg", "svg"] {
        f.save(format!("gallery/dimant_oppenheim.{ext}"))?;
    }
    // Smooth log-height surface of exactly the same density function.
    let xs: Vec<_> = (0..=160).map(|i| -5.0 + 25.0 * i as f64 / 160.).collect();
    let ys: Vec<_> = (0..=128).map(|i| -10.0 + 20.0 * i as f64 / 128.).collect();
    let mut z = vec![];
    for &r in &ys {
        for &x in &xs {
            z.push(if x.hypot(r) > 20.0 {
                f64::NAN
            } else {
                (8. * PI * model.at(x, r)).log10()
            });
        }
    }
    let mut f = Figure::new(IntendedUse::ArticleColumn);
    f.height_mm = 68.;
    let sc = f.scale(Scale::new(
        [-3., 4.],
        Normalization::Linear,
        ColorMap::listed(
            include_bytes!("support/dimant_colormap.rgb")
                .chunks_exact(3)
                .map(|rgb| {
                    [
                        rgb[0] as f64 / 255.,
                        rgb[1] as f64 / 255.,
                        rgb[2] as f64 / 255.,
                    ]
                })
                .collect(),
        )?,
    )?);
    let s = Surface::new(xs, ys, z, [-3., 4.], sc)?;
    let mut p = Panel::new(
        Rect::new(5., 12., 75., 40.),
        Axes::new([0., 1.], [0., 1.], "", ""),
    );
    p.axes = None;
    p.push(Layer::Surface(s));
    f.panels.push(p);
    f.labels.push(Label::new(r"Dimant--Oppenheim", 42., 5.));
    let mut bar = Colorbar::new(
        Rect::new(13., 55., 59., 2.),
        sc,
        r"$\log_{10}(8\pi n_e/n_*)$",
        vec![-3., -1., 1., 3., 4.],
    );
    bar.horizontal = true;
    f.colorbars.push(bar);
    f.save("gallery/dimant_surface.pdf")?;
    f.save("gallery/dimant_surface.png")?;
    Ok(())
}
