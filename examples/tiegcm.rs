//! Curved meridian sections from a completed paired TIE-GCM heating experiment.
use anyhow::{Result, ensure};
use rustplotlib::*;
use serde::Deserialize;
use std::fs;
#[derive(Deserialize)]
struct Meta {
    distance: Vec<f64>,
    altitude: Vec<f64>,
    time_s: Vec<f64>,
    frames: Vec<usize>,
}
fn edges(v: &[f64]) -> Vec<f64> {
    let mut e = vec![v[0] - (v[1] - v[0]) / 2.];
    e.extend(v.windows(2).map(|w| (w[0] + w[1]) / 2.));
    e.push(v[v.len() - 1] + (v[v.len() - 1] - v[v.len() - 2]) / 2.);
    e
}
fn main() -> Result<()> {
    let m: Meta = serde_json::from_slice(&fs::read("output/tiegcm/geometry.json")?)?;
    let mut f = Figure::new(IntendedUse::ArticleWide);
    f.height_mm = 144.;
    f.labels
        .push(Label::new("TIE-GCM: a heating pulse over Tromsø", 89., 5.));
    f.labels.push(Label::new(
        r"Meridian $20^\circ$E; pulse minus control",
        89.,
        13.,
    ));
    let sc = f.scale(Scale::new(
        [-12.5, 12.5],
        Normalization::Linear,
        ColorMap::diverging(),
    )?);
    let d = edges(&m.distance);
    let z = edges(&m.altitude);
    for (k, &frame) in m.frames.iter().enumerate() {
        let raw = fs::read(format!("output/tiegcm/{frame}.f32"))?;
        let vals = raw
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes(b.try_into().unwrap()) as f64)
            .collect::<Vec<_>>();
        ensure!(
            vals.len() == m.distance.len() * m.altitude.len(),
            "Section shape mismatch"
        );
        // Transparent curved sections can nest without changing their physical scale.
        let y = 21. + k as f64 * 22.;
        let mut p = Panel::new(
            Rect::new(15., y, 150., 32.8125),
            Axes::new([-4000., 4000.], [-1200., 550.], "", ""),
        );
        p.axes_visible = false;
        p.push(Layer::QuadMesh(cartography::curved_section(
            6371., &d, &z, vals, sc,
        )?));
        let mid = (d[0] + d[d.len() - 1]) / 2.;
        for alt in [0., 100., 300., 500.] {
            let mut l = Line::new(
                (0..301)
                    .map(|i| {
                        let a = (d[0] + (d[d.len() - 1] - d[0]) * i as f64 / 300. - mid) / 6371.;
                        [(6371. + alt) * a.sin(), (6371. + alt) * a.cos() - 6371.]
                    })
                    .collect(),
                "#61727c",
            );
            l.width_pt = if alt == 0. { 0.65 } else { 0.3 };
            p.push(Layer::Line(l));
        }
        if k == 0 {
            let latitude = 80.;
            let distance = (69.65 - latitude) * std::f64::consts::PI / 180. * 6371.;
            let angle = (distance - mid) / 6371.;
            for (altitude, label_y) in [(300., y + 15.), (100., y + 23.)] {
                let q = [
                    (6371. + altitude) * angle.sin(),
                    (6371. + altitude) * angle.cos() - 6371.,
                ];
                let elbow = [
                    (23. - 15.) / 150. * 8000. - 4000.,
                    550. - (label_y - y) / 32.8125 * 1750.,
                ];
                let end = [(18. - 15.) / 150. * 8000. - 4000., elbow[1]];
                let mut leader = Line::new(vec![q, elbow, end], "#61727c");
                leader.width_pt = 0.5;
                p.push(Layer::Line(leader));
                let mut label = Label::new(format!("{altitude:.0} km"), 9., label_y);
                label.color = "#465660".into();
                f.labels.push(label);
            }
        }
        for latitude in [80., 70., 60., 50., 40., 30.] {
            let distance = (69.65 - latitude) * std::f64::consts::PI / 180. * 6371.;
            if distance < d[0] || distance > d[d.len() - 1] {
                continue;
            }
            let a = (distance - mid) / 6371.;
            let radial = |height: f64| {
                [
                    (6371. + height) * a.sin(),
                    (6371. + height) * a.cos() - 6371.,
                ]
            };
            let mut tick = Line::new(vec![radial(0.), radial(-65.)], "#61727c");
            tick.width_pt = 0.55;
            p.push(Layer::Line(tick));
            if k == m.frames.len() - 1 {
                let q = radial(-190.);
                f.labels.push(Label::new(
                    format!(r"${latitude:.0}^\circ\mathrm{{N}}$"),
                    15. + (q[0] + 4000.) / 8000. * 150.,
                    y + (550. - q[1]) / 1750. * 32.8125,
                ));
            }
        }
        f.labels.push(Label::new(
            format!("{} min", (m.time_s[frame] / 60.) as i32),
            170.5,
            y + 3.,
        ));
        f.panels.push(p);
    }
    f.labels.push(Label::new("North", 18., 121.));
    f.labels.push(Label::new("South", 159., 121.));
    f.labels.push(Label::new(
        "Heights 100–500 km; spherical geometry",
        89.,
        121.,
    ));
    let mut b = Colorbar::new(
        Rect::new(40., 128., 98., 2.5),
        sc,
        r"$\Delta T\;[\mathrm K]$",
        vec![-12.5, 0., 12.5],
    );
    b.horizontal = true;
    f.colorbars.push(b);
    for ext in ["png", "pdf", "svg"] {
        f.save(format!("gallery/tiegcm.{ext}"))?;
    }
    Ok(())
}
