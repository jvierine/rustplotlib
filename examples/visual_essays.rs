//! Data-led visual essays: aligned margins, small multiples, inline diagnostics.
use anyhow::{Result, ensure};
use rustplotlib::*;
use std::{f64::consts::TAU, fs};
const INK: &str = "#27353e";
const BLUE: &str = "#2a627b";
const RED: &str = "#a34d3f";
fn label(f: &mut Figure, text: impl Into<String>, x: f64, y: f64, color: &str) {
    let mut l = Label::new(text, x, y);
    l.color = color.into();
    f.labels.push(l);
}
fn panel(rect: Rect, x: [f64; 2], y: [f64; 2]) -> Panel {
    let mut p = Panel::new(rect, Axes::new(x, y, "", ""));
    p.axes_visible = false;
    p
}
fn line(p: &mut Panel, points: Vec<[f64; 2]>, color: &str, width: f64) {
    let mut l = Line::new(points, color);
    l.width_pt = width;
    p.push(Layer::Line(l));
}
fn save(f: &Figure, name: &str) -> Result<()> {
    for ext in ["pdf", "png", "svg", "jpg"] {
        f.save(format!("gallery/{name}.{ext}"))?;
    }
    Ok(())
}
fn binary(path: &str) -> Result<Vec<f64>> {
    let b = fs::read(path)?;
    ensure!(b.len() % 4 == 0, "Invalid render transport");
    Ok(b.chunks_exact(4)
        .map(|v| f32::from_le_bytes(v.try_into().unwrap()) as f64)
        .collect())
}
fn frame(data: &[f64], k: usize) -> &[f64] {
    &data[k * 128 * 128..(k + 1) * 128 * 128]
}
fn mixing_atlas() -> Result<()> {
    let data = binary("output/essays/khi.f32")?;
    ensure!(data.len() == 41 * 128 * 128, "Unexpected KHI dimensions");
    let meta: serde_json::Value = serde_json::from_slice(&fs::read("output/essays/khi.json")?)?;
    let mut f = Figure::new(IntendedUse::ArticleWide);
    f.height_mm = 132.;
    label(&mut f, "A shear layer becomes a mixing field", 89., 6., INK);
    let sc = f.scale(Scale::new(
        [0., 1.],
        Normalization::Linear,
        ColorMap::estuary(),
    )?);
    let field = frame(&data, 40);
    let edges: Vec<_> = (0..=128).map(|k| TAU * k as f64 / 128.).collect();
    let rect = Rect::new(16., 28., 66., 66.);
    let mut p = panel(rect, [0., TAU], [0., TAU]);
    let mut grid = Field::new(edges.clone(), edges, field.to_vec(), sc)?;
    grid.interpolate = true;
    p.push(Layer::Pcolormesh(grid));
    f.panels.push(p);
    label(&mut f, "$t=12$", 49., 20., INK);
    label(&mut f, "$x=0$", 16., 98., INK);
    label(&mut f, r"$2\pi$", 82., 98., INK);
    label(&mut f, "$y=0$", 7., 93., INK);
    label(&mut f, r"$2\pi$", 8., 28., INK);
    // The marginal shares the exact vertical extent and coordinate system.
    let mut margin = panel(Rect::new(87., 28., 15., 66.), [0., 1.], [0., TAU]);
    let initial = frame(&data, 0);
    for (values, color) in [(initial, "#bbc3c5"), (field, BLUE)] {
        let pts = (0..128)
            .map(|j| {
                [
                    values[j * 128..(j + 1) * 128].iter().sum::<f64>() / 128.,
                    (j as f64 + 0.5) * TAU / 128.,
                ]
            })
            .collect();
        line(
            &mut margin,
            pts,
            color,
            if color == BLUE { 1.2 } else { 0.8 },
        );
    }
    f.panels.push(margin);
    label(&mut f, r"$\langle c\rangle_x$", 94., 20., BLUE);
    label(&mut f, "$0$", 87., 98., INK);
    label(&mut f, "$1$", 102., 98., INK);
    // Compact numerical evidence belongs beside the image, not in another legend.
    label(&mut f, "Mixing history", 143., 20., INK);
    let times: Vec<f64> = meta["time"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    let mixed: Vec<_> = (0..41)
        .map(|k| 4. * frame(&data, k).iter().map(|c| c * (1. - c)).sum::<f64>() / (128. * 128.))
        .collect();
    let mut spark = panel(Rect::new(115., 29., 49., 20.), [0., 12.], [0., 0.5]);
    line(
        &mut spark,
        times.iter().zip(&mixed).map(|(t, m)| [*t, *m]).collect(),
        BLUE,
        1.1,
    );
    line(&mut spark, vec![[0., 0.], [12., 0.]], "#d8dddf", 0.5);
    f.panels.push(spark);
    label(&mut f, r"$M=4\langle c(1-c)\rangle$", 143., 55., BLUE);
    label(
        &mut f,
        format!("${:.2}\\;\\longrightarrow\\;{:.2}$", mixed[0], mixed[40]),
        143.,
        63.,
        INK,
    );
    let energy: Vec<_> = meta["energy"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    let enstrophy: Vec<_> = meta["enstrophy"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    let mut spark = panel(Rect::new(115., 71., 49., 17.), [0., 12.], [0.6, 1.02]);
    for (v, c) in [(&energy, BLUE), (&enstrophy, RED)] {
        line(
            &mut spark,
            times
                .iter()
                .zip(v)
                .map(|(t, vv)| [*t, *vv / v[0]])
                .collect(),
            c,
            1.0,
        );
    }
    f.panels.push(spark);
    label(
        &mut f,
        format!("Energy ${:.0}\\%$", 100. * energy[40] / energy[0]),
        142.,
        93.,
        BLUE,
    );
    label(
        &mut f,
        format!("Enstrophy ${:.0}\\%$", 100. * enstrophy[40] / enstrophy[0]),
        142.,
        101.,
        RED,
    );
    // Chronology in six equally scaled miniature fields.
    for (col, k) in [0, 8, 16, 24, 32, 40].into_iter().enumerate() {
        let r = Rect::new(16. + col as f64 * 25., 109., 19., 19.);
        let mut p = panel(r, [0., 1.], [0., 1.]);
        let edges = (0..=128).map(|i| i as f64 / 128.).collect::<Vec<_>>();
        let mut field = Field::new(edges.clone(), edges, frame(&data, k).to_vec(), sc)?;
        field.interpolate = true;
        p.push(Layer::Pcolormesh(field));
        f.panels.push(p);
        label(
            &mut f,
            format!("${:.1}$", times[k]),
            r.x + r.width / 2.,
            105.,
            INK,
        );
    }
    let mut bar = Colorbar::new(Rect::new(17., 15., 15., 1.), sc, "", vec![0., 1.]);
    bar.horizontal = true;
    f.colorbars.push(bar);
    label(&mut f, "Dye $c$", 44., 16., INK);
    save(&f, "tufte_mixing")
}
fn meteor_anatomy() -> Result<()> {
    let data = binary("output/essays/meteor.f32")?;
    ensure!(data.len() == 1000 * 800, "Unexpected meteor dimensions");
    let mut f = Figure::new(IntendedUse::ArticleWide);
    f.height_mm = 115.;
    label(&mut f, "The anatomy of a meteor wake", 89., 6., INK);
    let colors = include_bytes!("support/dimant_colormap.rgb")
        .chunks_exact(3)
        .map(|c| [c[0] as f64 / 255., c[1] as f64 / 255., c[2] as f64 / 255.])
        .collect();
    let sc = f.scale(Scale::new(
        [1e-3, 1e4],
        Normalization::Log,
        ColorMap::listed(colors)?,
    )?);
    let x = (0..=1000)
        .map(|i| -5. + 25. * i as f64 / 1000.)
        .collect::<Vec<_>>();
    let y = (0..=800)
        .map(|i| -10. + 20. * i as f64 / 800.)
        .collect::<Vec<_>>();
    let mut p = panel(Rect::new(19., 27., 90., 72.), [-5., 20.], [-10., 10.]);
    p.pcolormesh(x.clone(), y, data.clone(), sc)?;
    line(&mut p, vec![[-5., 0.], [20., 0.]], "#dce0e3", 0.4);
    f.panels.push(p);
    let column: Vec<_> = (0..1000)
        .map(|i| {
            data.iter()
                .skip(i)
                .step_by(1000)
                .filter(|x| x.is_finite())
                .sum::<f64>()
                * 0.025
        })
        .collect();
    let peak = column.iter().copied().fold(0.0, f64::max);
    let mut marginal = panel(Rect::new(19., 14., 90., 10.), [-5., 20.], [0., 1.05]);
    line(
        &mut marginal,
        (0..1000)
            .map(|i| [(x[i] + x[i + 1]) / 2., column[i] / peak])
            .collect(),
        INK,
        1.0,
    );
    f.panels.push(marginal);
    label(&mut f, r"$\int \mathcal N\,\mathrm{d}r$", 149., 18., INK);
    label(&mut f, "peak normalized", 149., 25., "#6d7478");
    // Encoded longitudinal quantiles of the displayed cross-section.
    let total = column.iter().sum::<f64>();
    let mut cumulative = 0.;
    let mut quantiles = vec![];
    let mut target = 0;
    for (i, c) in column.iter().enumerate() {
        cumulative += c;
        while target < 3 && cumulative / total >= [0.5, 0.8, 0.95][target] {
            quantiles.push((x[i] + x[i + 1]) / 2.);
            target += 1;
        }
    }
    label(&mut f, "Wake footprint", 149., 43., INK);
    let mut ledger = panel(Rect::new(131., 50., 24., 30.), [0., 20.], [0., 3.]);
    for (k, (&q, percentage)) in quantiles.iter().zip([50, 80, 95]).enumerate() {
        let yy = 2.5 - k as f64;
        line(&mut ledger, vec![[0., yy], [q.max(0.), yy]], BLUE, 1.8);
        label(
            &mut f,
            format!("${percentage}\\%$"),
            122.,
            50. + (3. - yy) / 3. * 30.,
            INK,
        );
        label(
            &mut f,
            format!("${q:.1}\\lambda_T$"),
            165.,
            50. + (3. - yy) / 3. * 30.,
            BLUE,
        );
    }
    f.panels.push(ledger);
    label(&mut f, "Cumulative density", 149., 89., INK);
    label(&mut f, r"within $|r|\leq10\lambda_T$", 149., 97., "#6d7478");
    for v in [-5., 0., 5., 10., 15., 20.] {
        label(
            &mut f,
            format!("${v}$"),
            19. + (v + 5.) / 25. * 90.,
            109.,
            INK,
        );
    }
    label(
        &mut f,
        r"$z/\lambda_T$; positive behind the meteor",
        69.,
        116. - 0.0,
        INK,
    );
    f.height_mm = 122.;
    for v in [-10., 0., 10.] {
        label(
            &mut f,
            format!("${v}$"),
            11.,
            27. + (10. - v) / 20. * 72.,
            INK,
        );
    }
    let mut l = Label::new(r"$r/\lambda_T$", 5., 66.);
    l.rotation = -90.;
    f.labels.push(l);
    f.height_mm = 145.;
    let mut bar = Colorbar::new(
        Rect::new(19., 127., 90., 2.5),
        sc,
        r"$\mathcal N=8\pi n_e/n_*$",
        vec![1e-3, 1., 1e4],
    );
    bar.horizontal = true;
    f.colorbars.push(bar);
    save(&f, "tufte_meteor")
}
fn wave_ledger() -> Result<()> {
    let mut f = Figure::new(IntendedUse::ArticleWide);
    f.height_mm = 115.;
    label(&mut f, "One wave, twelve clocks", 89., 6., INK);
    label(
        &mut f,
        r"Analytic pulse; $c=0.7\,\mathrm{km\,s^{-1}}$",
        89.,
        14.,
        "#66747a",
    );
    let rect = Rect::new(23., 25., 120., 72.);
    let mut p = panel(rect, [0., 42.], [-70., 1170.]);
    let mut ridge = Vec::new();
    for k in 0..12 {
        let d = k as f64 * 100.;
        let arrival = 8. + d / 0.7 / 60.;
        ridge.push([arrival, d]);
        let pts = (0..1600)
            .map(|i| {
                let t = i as f64 / 1599. * 42.;
                let tau = t - arrival;
                let wave = (-(tau / 2.4).powi(2)).exp() * (TAU * tau / 2.0).cos();
                [t, d + 42. * (-d / 1800.).exp() * wave]
            })
            .collect();
        line(&mut p, pts, if k % 3 == 0 { BLUE } else { "#56656c" }, 0.8);
        label(
            &mut f,
            format!("${d:.0}$"),
            14.,
            rect.y + (1170. - d) / 1240. * rect.height,
            INK,
        );
    }
    let mut l = Line::new(ridge, "#b77859");
    l.width_pt = 0.6;
    l.dash = Some("0.7 1.2".into());
    p.push(Layer::Line(l));
    f.panels.push(p);
    for t in [0., 10., 20., 30., 40.] {
        label(&mut f, format!("${t:.0}$"), 23. + t / 42. * 120., 102., INK);
    }
    label(&mut f, "Elapsed time [min]", 83., 110., INK);
    let mut l = Label::new("Receiver distance [km]", 5., 60.);
    l.rotation = -90.;
    f.labels.push(l);
    for (text, y, color) in [
        ("Peak", 55., "#b77859"),
        ("arrival", 61., "#b77859"),
        ("Traces", 75., INK),
        ("offset by", 81., INK),
        ("distance", 87., INK),
    ] {
        let mut l = Label::new(text, 159., y);
        l.color = color.into();
        f.labels.push(l);
    }
    save(&f, "tufte_wave")
}
fn main() -> Result<()> {
    mixing_atlas()?;
    meteor_anatomy()?;
    wave_ledger()?;
    Ok(())
}
