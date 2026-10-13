//! Generate synthetic wave data and analytic vortex rings, not a fluid solver.
use anyhow::Result;
use rustplotlib::{volume::Volume, *};
use std::{fs, path::Path};
fn write_f32(path: impl AsRef<Path>, data: &[f32]) -> Result<()> {
    fs::write(
        path,
        data.iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
    )?;
    Ok(())
}
fn main() -> Result<()> {
    fs::create_dir_all("web/data")?;
    let (nx, ny) = (192, 112);
    let mut frames = vec![];
    for k in 0..32 {
        let phase = k as f64 / 31.0 * std::f64::consts::TAU;
        for j in 0..ny {
            for i in 0..nx {
                let x = i as f64 / (nx - 1) as f64 * 8.0 - 4.0;
                let y = j as f64 / (ny - 1) as f64 * 6.0;
                frames.push(
                    ((1.8 * x - 1.2 * y - phase).sin()
                        * (-0.035 * (x * x + (y - 3.0).powi(2))).exp()) as f32,
                );
            }
        }
    }
    write_f32("web/data/waves.f32", &frames)?;
    let n = 48;
    let mut volumes = vec![];
    for frame in 0..24 {
        let t = frame as f64 / 23.0;
        let mut values = vec![];
        for k in 0..n {
            for j in 0..n {
                for i in 0..n {
                    let x = i as f64 / (n - 1) as f64 * 2.0 - 1.0;
                    let y = j as f64 / (n - 1) as f64 * 2.0 - 1.0;
                    let z = k as f64 / (n - 1) as f64 * 2.0 - 1.0;
                    let r = (x * x + y * y).sqrt();
                    let sep = 0.42 * (1.0 - t);
                    let radius = 0.46 + 0.08 * t;
                    let core = 0.095;
                    let q = ((-((r - radius).powi(2) + (z - sep).powi(2)) / (2.0 * core * core))
                        .exp()
                        + (-((r - radius).powi(2) + (z + sep).powi(2)) / (2.0 * core * core))
                            .exp())
                        * 0.5;
                    values.push(q as f32);
                }
            }
        }
        let v = Volume::new([n, n, n], [[-1.0, 1.0]; 3], values)?;
        volumes.extend(v.values);
    }
    write_f32("web/data/rings.f32", &volumes)?;
    let spec = serde_json::json!({"field":{"shape":[nx,ny],"frames":32,"bounds":[[-4,4],[0,6]],"path":"data/waves.f32"},"volume":{"shape":[n,n,n],"frames":24,"path":"data/rings.f32","description":"Analytic vortex-ring fields"}});
    fs::write("web/data/manifest.json", serde_json::to_vec_pretty(&spec)?)?;
    // Genuine LaTeX is precompiled to paths for the web, never approximated by HTML math.
    let mut labels = Figure::new(IntendedUse::Mobile);
    labels.width_mm = 100.0;
    labels.height_mm = 15.0;
    labels.labels.push(Label::new(
        r"$p'/p_0\;[\%]\qquad -1\quad 0\quad 1$",
        50.0,
        6.0,
    ));
    labels.save("web/data/field_label.svg")?;
    labels.labels.clear();
    labels.labels.push(Label::new(
        r"$|\boldsymbol{\omega}|/\omega_{\max}$",
        50.0,
        6.0,
    ));
    labels.save("web/data/volume_label.svg")?;
    println!("Prepared Rust-generated field and volume data");
    Ok(())
}
