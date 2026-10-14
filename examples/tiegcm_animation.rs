//! One-panel animation from all 49 saved TIE-GCM samples; no temporal interpolation.
use anyhow::{Result, ensure};
use rustplotlib::*;
use serde::Deserialize;
use std::fs;
#[derive(Deserialize)]
struct Meta {
    distance: Vec<f64>,
    altitude: Vec<f64>,
    time_s: Vec<f64>,
}
fn edges(v: &[f64]) -> Vec<f64> {
    let mut e = vec![v[0] - (v[1] - v[0]) / 2.];
    e.extend(v.windows(2).map(|w| (w[0] + w[1]) / 2.));
    e.push(v[v.len() - 1] + (v[v.len() - 1] - v[v.len() - 2]) / 2.);
    e
}
fn main() -> Result<()> {
    let m: Meta = serde_json::from_slice(&fs::read("output/tiegcm/geometry.json")?)?;
    let d = edges(&m.distance);
    let z = edges(&m.altitude);
    let mid = (d[0] + d[d.len() - 1]) / 2.;
    fs::create_dir_all("output/tiegcm-animation")?;
    for (frame, &time) in m.time_s.iter().enumerate() {
        let raw = fs::read(format!("output/tiegcm/{frame}.f32"))?;
        let vals = raw
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes(b.try_into().unwrap()) as f64)
            .collect::<Vec<_>>();
        ensure!(
            vals.len() == m.distance.len() * m.altitude.len(),
            "Section shape mismatch"
        );
        let mut f = Figure::new(IntendedUse::ArticleWide);
        f.height_mm = 82.;
        f.tight_export = false;
        f.raster_dpi = 180.;
        let sc = f.scale(Scale::new(
            [-12.5, 12.5],
            Normalization::Linear,
            ColorMap::diverging(),
        )?);
        f.labels
            .push(Label::new("TIE-GCM: heating pulse over Tromsø", 89., 5.));
        f.labels.push(Label::new(
            r"Meridian $20^\circ$E; pulse minus control",
            89.,
            13.,
        ));
        let mut p = Panel::new(
            Rect::new(15., 23., 150., 32.8125),
            Axes::new([-4000., 4000.], [-1200., 550.], "", ""),
        );
        p.axes_visible = false;
        p.push(Layer::QuadMesh(cartography::curved_section(
            6371., &d, &z, vals, sc,
        )?));
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
        for lat in [80., 70., 60., 50., 40., 30.] {
            let a = ((69.65 - lat) * std::f64::consts::PI / 180. * 6371. - mid) / 6371.;
            let radial = |height: f64| {
                [
                    (6371. + height) * a.sin(),
                    (6371. + height) * a.cos() - 6371.,
                ]
            };
            let mut l = Line::new(vec![radial(0.), radial(-65.)], "#61727c");
            l.width_pt = 0.55;
            p.push(Layer::Line(l));
            let q = radial(-190.);
            f.labels.push(Label::new(
                format!(r"${lat:.0}^\circ\mathrm{{N}}$"),
                15. + (q[0] + 4000.) / 8000. * 150.,
                23. + (550. - q[1]) / 1750. * 32.8125,
            ));
        }
        f.labels
            .push(Label::new(format!("{:.0} min", time / 60.), 166., 24.));
        f.labels.push(Label::new("100–500 km", 89., 59.));
        let mut bar = Colorbar::new(
            Rect::new(40., 66., 98., 2.5),
            sc,
            r"$\Delta T\;[\mathrm K]$",
            vec![-12.5, 0., 12.5],
        );
        bar.horizontal = true;
        f.colorbars.push(bar);
        f.panels.push(p);
        f.save(format!("output/tiegcm-animation/{frame:04}.png"))?;
        if frame == 24 {
            f.save("gallery/tiegcm_animation.pdf")?;
            f.save("gallery/tiegcm_animation.png")?;
        }
    }
    fs::write(
        "output/tiegcm-animation/provenance.json",
        serde_json::to_string_pretty(
            &serde_json::json!({"source":"gallery/tiegcm/section.h5","frames":m.time_s.len(),"time_s":m.time_s,"temporal_interpolation":false,"renderer":"Rustplotlib","limits_K":[-12.5,12.5]}),
        )?,
    )?;
    Ok(())
}
