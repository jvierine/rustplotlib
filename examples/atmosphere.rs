//! Analytic atmospheric fields for the spherical cutaway renderer.
use anyhow::Result;
use rustplotlib::*;
use std::fs;
fn main() -> Result<()> {
    fs::create_dir_all("web/data/atmosphere")?;
    let n = 80;
    let mut fields = [Vec::new(), Vec::new(), Vec::new()];
    for k in 0..n {
        for j in 0..n {
            for i in 0..n {
                let x = i as f64 / (n - 1) as f64 - 0.5;
                let y = j as f64 / (n - 1) as f64 - 0.5;
                let z = k as f64 / (n - 1) as f64 - 0.5;
                let r = (x * x + y * y + z * z).sqrt();
                let altitude = (r - 0.21) / 0.26 * 500.0;
                let day = (x / r.max(1e-9)).clamp(-1.0, 1.0);
                let lat = (y / r.max(1e-9)).clamp(-1.0, 1.0).asin();
                let lon = z.atan2(x);
                fields[0].push(
                    (250.0
                        + 750.0 * (1.0 - (-altitude.max(0.0) / 100.0).exp()) * (0.85 + 0.15 * day))
                        as f32,
                );
                fields[1].push(
                    (1e12
                        * (-((altitude - 290.0) / 85.0).powi(2)).exp()
                        * (0.4 + 0.6 * (day + 1.0) * 0.5)
                        * (-0.6 * lat.powi(2)).exp()) as f32,
                );
                fields[2].push(
                    (180.0 * (3.0 * lon + 2.0 * lat).sin() * (altitude.max(0.0) / 200.0).min(1.0))
                        as f32,
                );
            }
        }
    }
    let specs = [
        ("temperature", 250.0, 1100.0, r"$T\;[\mathrm{K}]$"),
        ("electron_density", 0.0, 1e12, r"$N_e\;[\mathrm{m^{-3}}]$"),
        (
            "zonal_wind",
            -200.0,
            200.0,
            r"$u_{\mathrm{east}}\;[\mathrm{m\,s^{-1}}]$",
        ),
    ];
    for (values, (name, lo, hi, label)) in fields.iter().zip(specs) {
        fs::write(
            format!("web/data/atmosphere/{name}.f32"),
            values
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        )?;
        let mut f = Figure::new(IntendedUse::Mobile);
        f.width_mm = 100.0;
        f.height_mm = 20.0;
        let scale = f.scale(Scale::new(
            [lo, hi],
            Normalization::Linear,
            ColorMap::new(if name == "temperature" {
                vec![
                    [0.08, 0.12, 0.35],
                    [0.20, 0.50, 0.70],
                    [0.96, 0.80, 0.32],
                    [0.75, 0.17, 0.15],
                ]
            } else if name == "electron_density" {
                vec![
                    [0.07, 0.08, 0.15],
                    [0.18, 0.36, 0.30],
                    [0.59, 0.56, 0.75],
                    [0.98, 0.98, 0.96],
                ]
            } else {
                vec![[0.12, 0.23, 0.61], [0.97, 0.96, 0.92], [0.69, 0.10, 0.16]]
            })?,
        )?);
        let mut b = Colorbar::new(
            Rect::new(12.0, 3.0, 76.0, 2.0),
            scale,
            label,
            vec![lo, (lo + hi) / 2.0, hi],
        );
        b.horizontal = true;
        f.colorbars.push(b);
        f.save(format!("web/data/atmosphere/{name}.svg"))?;
    }
    fs::write(
        "web/data/atmosphere/manifest.json",
        serde_json::to_vec_pretty(
            &serde_json::json!({"shape":[n,n,n],"geometry":"Cartesian sampling of spherical shells; inner radius 0.21, outer 0.47", "description":"Analytic demonstration fields, not observations or a dynamical simulation"}),
        )?,
    )?;
    Ok(())
}
