use anyhow::Result;
use rustplotlib::*;
use std::{fs, path::Path, time::Instant};

fn main() -> Result<()> {
    let dir = std::env::args().nth(1).unwrap_or("gallery".into());
    fs::create_dir_all(&dir)?;
    let mut f = Figure::new(IntendedUse::ArticleWide);
    f.height_mm = 140.0;
    let signed = f.scale(Scale::new(
        [-1.0, 1.0],
        Normalization::Linear,
        ColorMap::diverging(),
    )?);
    let logarithmic = f.scale(Scale::new(
        [0.01, 10.0],
        Normalization::Log,
        ColorMap::sequential(),
    )?);
    let x: Vec<_> = (0..=120).map(|i| -4.0 + i as f64 / 15.0).collect();
    let y: Vec<_> = (0..=80)
        .map(|j| 6.0 * (j as f64 / 80.0).powf(1.35))
        .collect();
    let wave =
        |x: f64, y: f64| (1.8 * x - 1.2 * y).sin() * (-0.035 * (x * x + (y - 3.0).powi(2))).exp();
    let cell: Vec<_> = y
        .windows(2)
        .flat_map(|ys| {
            x.windows(2)
                .map(move |xs| wave((xs[0] + xs[1]) * 0.5, (ys[0] + ys[1]) * 0.5))
        })
        .collect();
    let nodal: Vec<_> = y
        .iter()
        .flat_map(|yy| x.iter().map(move |xx| wave(*xx, *yy)))
        .collect();
    let levels = vec![-1.0, -0.75, -0.5, -0.25, 0.0, 0.25, 0.5, 0.75, 1.0];
    let axes = || {
        Axes::new(
            [-4.0, 4.0],
            [0.0, 6.0],
            r"$x\;[\mathrm{km}]$",
            r"$z\;[\mathrm{km}]$",
        )
    };
    let mut a = Panel::new(Rect::new(18.0, 13.0, 58.0, 34.0), axes());
    a.push(Layer::Pcolormesh(Field::new(
        x.clone(),
        y.clone(),
        cell,
        signed,
    )?));
    f.panels.push(a);
    f.labels
        .push(Label::new(r"$a)\quad\text{Cell-centred field}$", 47.0, 7.0));
    let mut b = Panel::new(Rect::new(103.0, 13.0, 58.0, 34.0), axes());
    b.push(Layer::Contour(Contour::new(
        x.clone(),
        y.clone(),
        nodal.clone(),
        levels.clone(),
        signed,
        true,
    )?));
    f.panels.push(b);
    f.labels
        .push(Label::new(r"$b)\quad\text{Filled contours}$", 132.0, 7.0));
    let mut cb = Colorbar::new(
        Rect::new(18.0, 60.0, 143.0, 2.0),
        signed,
        r"$p'/p_0\;[\%]$",
        vec![-1.0, -0.5, 0.0, 0.5, 1.0],
    );
    cb.horizontal = true;
    f.colorbars.push(cb);
    let mut c = Panel::new(Rect::new(18.0, 86.0, 58.0, 34.0), axes());
    c.push(Layer::Contour(Contour::new(
        x.clone(),
        y.clone(),
        nodal,
        levels,
        signed,
        false,
    )?));
    f.panels.push(c);
    f.labels
        .push(Label::new(r"$c)\quad\text{Contour lines}$", 47.0, 79.0));
    let positive: Vec<_> = y
        .windows(2)
        .flat_map(|ys| {
            x.windows(2).map(move |xs| {
                10f64.powf(
                    -2.0 + 3.0
                        * ((xs[0] + xs[1]) / 8.0 + 0.5)
                        * (-0.025 * ((ys[0] + ys[1]) * 0.5 - 3.0).powi(2)).exp(),
                )
            })
        })
        .collect();
    let mut d = Panel::new(Rect::new(103.0, 86.0, 45.0, 34.0), axes());
    d.push(Layer::Pcolormesh(Field::new(x, y, positive, logarithmic)?));
    f.panels.push(d);
    f.labels.push(Label::new(
        r"$d)\quad\text{Independent logarithmic scale}$",
        132.0,
        79.0,
    ));
    f.colorbars.push(Colorbar::new(
        Rect::new(151.0, 86.0, 2.0, 34.0),
        logarithmic,
        r"$Q\;[\mathrm{W\,m^{-3}}]$",
        vec![0.01, 0.1, 1.0, 10.0],
    ));
    let start = Instant::now();
    f.save(Path::new(&dir).join("fields.pdf"))?;
    let cold = start.elapsed();
    let start = Instant::now();
    f.save(Path::new(&dir).join("fields.png"))?;
    let warm = start.elapsed();
    f.save(Path::new(&dir).join("fields.svg"))?;
    f.save(Path::new(&dir).join("fields.jpg"))?;
    println!("PDF including first typesetting: {cold:?}; cached-label PNG: {warm:?}");
    // A genuinely single-column figure: 85 mm wide and 10 pt labels.
    let mut one = Figure::new(IntendedUse::ArticleColumn);
    let ids = one.grid(1, 1)?;
    let panel = &mut one.panels[ids[0]];
    panel.axes = Some(Axes::new(
        [0.0, 12.0],
        [-1.1, 1.1],
        r"$t\;[\mathrm{s}]$",
        r"$\omega/\omega_0$",
    ));
    for (tau, color) in [(5.0, "#245774"), (8.0, "#8b3940")] {
        panel.push(Layer::Line(Line::new(
            (0..601)
                .map(|i| {
                    let t = i as f64 * 0.02;
                    [t, (-t / tau).exp() * (2.0 * t).cos()]
                })
                .collect(),
            color,
        )));
    }
    let mut label = Label::new(r"$\tau=8\,\mathrm{s}$", 65.0, 11.0);
    label.color = "#8b3940".into();
    panel.labels.push(label);
    let mut label = Label::new(r"$\tau=5\,\mathrm{s}$", 65.0, 17.0);
    label.color = "#245774".into();
    panel.labels.push(label);
    one.save(Path::new(&dir).join("single_column.pdf"))?;
    one.save(Path::new(&dir).join("single_column.png"))?;
    one.save(Path::new(&dir).join("single_column.svg"))?;
    Ok(())
}
