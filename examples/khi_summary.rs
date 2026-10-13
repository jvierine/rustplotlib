//! Small multiples and shared diagnostics from the actual Rust KHI run.
use anyhow::Result;
use rustplotlib::*;
use std::{f64::consts::TAU, fs};
fn main() -> Result<()> {
    let bytes = fs::read("output/khi/dye.f32")?;
    let data: Vec<f64> = bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes(b.try_into().unwrap()) as f64)
        .collect();
    let meta: serde_json::Value =
        serde_json::from_slice(&fs::read("output/khi/diagnostics.json")?)?;
    let samples = meta["samples"].as_array().unwrap();
    let mut f = Figure::new(IntendedUse::ArticleWide);
    f.height_mm = 115.0;
    let sc = f.scale(Scale::new(
        [0., 1.],
        Normalization::Linear,
        ColorMap::estuary(),
    )?);
    let edges: Vec<_> = (0..=128).map(|i| i as f64 / 128.0 * TAU).collect();
    for (col, k) in [0, 20, 40].into_iter().enumerate() {
        let rect = Rect::new(18. + col as f64 * 54., 12., 46., 46.);
        let mut axes = Axes::new(
            [0., TAU],
            [0., TAU],
            "$x$",
            if col == 0 { "$y$" } else { "" },
        );
        axes.xticks = vec![0., 3., 6.];
        axes.yticks = if col == 0 { vec![0., 3., 6.] } else { vec![] };
        let mut p = Panel::new(rect, axes);
        let mut field = Field::new(
            edges.clone(),
            edges.clone(),
            data[k * 128 * 128..(k + 1) * 128 * 128].to_vec(),
            sc,
        )?;
        field.interpolate = true;
        p.push(Layer::Pcolormesh(field));
        f.panels.push(p);
        f.labels.push(Label::new(
            format!(r"$t={:.1}$", samples[k]["time"].as_f64().unwrap()),
            rect.x + rect.width / 2.,
            5.,
        ));
    }
    let mut bar = Colorbar::new(Rect::new(18., 74., 137., 1.5), sc, "", vec![0., 0.5, 1.]);
    bar.horizontal = true;
    f.colorbars.push(bar);
    let rect = Rect::new(18., 85., 137., 20.);
    let mut axes = Axes::new([0., 12.], [0., 1.03], "$t$", "");
    axes.xticks = vec![0., 4., 8., 12.];
    axes.yticks = vec![0., 1.];
    let mut p = Panel::new(rect, axes);
    for (key, title, color) in [
        ("energy", "Energy", "#245774"),
        ("enstrophy", "Enstrophy", "#a3483d"),
    ] {
        let start = samples[0][key].as_f64().unwrap();
        let points = samples
            .iter()
            .map(|s| {
                [
                    s["time"].as_f64().unwrap(),
                    s[key].as_f64().unwrap() / start,
                ]
            })
            .collect();
        p.push(Layer::Line(Line::new(points, color)));
        let last = samples.last().unwrap()[key].as_f64().unwrap() / start;
        let mut label = Label::new(title, 165., rect.y + (1.03 - last) / 1.03 * rect.height);
        label.color = color.into();
        p.labels.push(label);
    }
    f.panels.push(p);
    for ext in ["pdf", "png", "svg"] {
        f.save(format!("gallery/khi_summary.{ext}"))?;
    }
    Ok(())
}
