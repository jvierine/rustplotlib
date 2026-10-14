//! All fields in this gallery are synthetic or analytic.
use anyhow::Result;
use rustplotlib::*;
use std::{fs, path::Path};

fn save(f: &Figure, name: &str) -> Result<()> {
    for ext in ["pdf", "png", "jpg", "svg"] {
        f.save(Path::new("gallery").join(format!("{name}.{ext}")))?;
    }
    println!("{name}: PDF, PNG, JPEG, SVG");
    Ok(())
}
fn edges(n: usize, a: f64, b: f64) -> Vec<f64> {
    (0..=n).map(|i| a + (b - a) * i as f64 / n as f64).collect()
}
fn cell_values(x: &[f64], y: &[f64], sample: impl Fn(f64, f64) -> f64) -> Vec<f64> {
    let mut out = Vec::with_capacity((x.len() - 1) * (y.len() - 1));
    for yy in y.windows(2) {
        for xx in x.windows(2) {
            out.push(sample((xx[0] + xx[1]) * 0.5, (yy[0] + yy[1]) * 0.5));
        }
    }
    out
}

fn keogram() -> Result<()> {
    let mut f = Figure::new(IntendedUse::ArticleWide);
    f.height_mm = 82.0;
    let scale = f.scale(Scale::new(
        [-0.05, 0.05],
        Normalization::Asinh { width: 0.004 },
        ColorMap::diverging(),
    )?);
    let t = edges(1200, 0.0, 60.0);
    let d = edges(720, 0.0, 1500.0);
    for (k, title) in ["Complete field", "Gapped sampling"]
        .into_iter()
        .enumerate()
    {
        let rect = Rect::new(18.0 + k as f64 * 84.0, 12.0, 59.0, 36.0);
        let mut axes = Axes::new(
            [0.0, 60.0],
            [0.0, 1500.0],
            r"$t\;[\mathrm{min}]$",
            r"$d\;[\mathrm{km}]$",
        );
        axes.xticks = vec![0.0, 20.0, 40.0, 60.0];
        axes.yticks = vec![0.0, 500.0, 1000.0, 1500.0];
        let mut p = Panel::new(rect, axes);
        let data = cell_values(&t, &d, |t, d| {
            if k == 1
                && ((23.0..27.0).contains(&t)
                    || ((400.0..750.0).contains(&d) && (42.0..50.0).contains(&t)))
            {
                return f64::NAN;
            }
            let tau = t * 60.0 - d / 0.7 - 480.0;
            0.045
                * (-(tau / 320.0).powi(2)).exp()
                * (tau * std::f64::consts::TAU / 160.0).cos()
                * (-d / 1800.0).exp()
        });
        p.pcolormesh(t.clone(), d.clone(), data, scale)?;
        let mut guide = Line::new(
            vec![[8.0, 0.0], [8.0 + 1500.0 / 0.7 / 60.0, 1500.0]],
            "#747743",
        );
        guide.width_pt = 0.65;
        guide.dash = Some("1.0 1.0".into());
        p.push(Layer::Line(guide));
        if k == 0 {
            let mut label = Label::new(
                r"$c=0.7\,\mathrm{km\,s^{-1}}$",
                rect.x + rect.width * 0.28,
                rect.y + rect.height * 0.14,
            );
            label.color = "#747743".into();
            p.labels.push(label);
        }
        f.panels.push(p);
        f.labels.push(Label::new(
            format!(
                "${})\\quad\\text{{{title}}}$",
                if k == 0 { "a" } else { "b" }
            ),
            rect.x + rect.width / 2.0,
            6.0,
        ));
    }
    let mut bar = Colorbar::new(
        Rect::new(18.0, 63.0, 143.0, 2.0),
        scale,
        r"$\delta\mathrm{TEC}\;[\mathrm{TECU}]$",
        vec![-0.05, -0.01, 0.0, 0.01, 0.05],
    );
    bar.horizontal = true;
    f.colorbars.push(bar);
    save(&f, "keogram")
}

fn curvilinear() -> Result<()> {
    let mut f = Figure::new(IntendedUse::ArticleColumn);
    f.height_mm = 98.0;
    let scale = f.scale(Scale::new(
        [-1.0, 1.0],
        Normalization::Linear,
        ColorMap::diverging(),
    )?);
    let (nx, ny) = (96, 360);
    let r = edges(nx, 1.0, 3.0);
    let theta = edges(ny, -std::f64::consts::PI, std::f64::consts::PI);
    let mut corners = vec![];
    for t in &theta {
        for r in &r {
            corners.push([r * t.cos(), r * t.sin()]);
        }
    }
    let values = cell_values(&r, &theta, |r, t| (3.0 * t - 2.0 * r).sin());
    let rect = Rect::new(18.0, 12.0, 50.0, 50.0);
    let mut axes = Axes::new([-3.2, 3.2], [-3.2, 3.2], r"$x$", r"$y$");
    axes.xticks = vec![-3.0, 0.0, 3.0];
    axes.yticks = vec![-3.0, 0.0, 3.0];
    let mut p = Panel::new(rect, axes);
    p.push(Layer::QuadMesh(QuadMesh::new(
        nx, ny, corners, values, scale,
    )?));
    f.panels.push(p);
    f.labels.push(Label::new(r"Curvilinear mesh", 43.0, 6.0));
    let mut bar = Colorbar::new(
        Rect::new(18.0, 77.0, 50.0, 2.0),
        scale,
        r"$\varphi(r,\theta)=\sin(3\theta-2r)$",
        vec![-1.0, 0.0, 1.0],
    );
    bar.horizontal = true;
    f.colorbars.push(bar);
    save(&f, "curvilinear")
}

fn dispersion() -> Result<()> {
    let mut f = Figure::new(IntendedUse::ArticleWide);
    f.height_mm = 107.0;
    f.labels.push(Label::new(
        r"$\omega^4-\big[c_s^2(k_h^2+m^2)+\omega_a^2\big]\omega^2+c_s^2N^2k_h^2=0$",
        89.0,
        10.0,
    ));
    for (k, title, color) in [
        (0, "Gravity branch", "#245774"),
        (1, "Acoustic branch", "#8b3940"),
    ] {
        let rect = Rect::new(18.0 + k as f64 * 85.0, 34.0, 58.0, 36.0);
        let ylim = if k == 0 { [0.0, 1.1] } else { [1.4, 3.6] };
        let mut axes = Axes::new([0.0, 3.0], ylim, r"$c_sk_h/N$", r"$\omega/N$");
        axes.xticks = vec![0.0, 1.0, 2.0, 3.0];
        let mut p = Panel::new(rect, axes);
        let points = (0..401)
            .map(|i| {
                let h = i as f64 / 400.0 * 3.0;
                let s = h * h + 2.5;
                let sign = if k == 0 { -1.0 } else { 1.0 };
                [h, ((s + sign * (s * s - 4.0 * h * h).sqrt()) / 2.0).sqrt()]
            })
            .collect();
        p.push(Layer::Line(Line::new(points, color)));
        f.panels.push(p);
        f.labels.push(Label::new(
            format!(
                "${})\\quad\\text{{{title}}}$",
                if k == 0 { "a" } else { "b" }
            ),
            rect.x + rect.width / 2.0,
            27.0,
        ));
    }
    f.labels.push(Label::new(r"$\displaystyle\omega_{\pm}^{2}=\frac{S\pm\sqrt{S^{2}-4c_s^{2}N^{2}k_h^{2}}}{2},\qquad S=c_s^2(k_h^2+m^2)+\omega_a^2$",89.0,94.0));
    save(&f, "dispersion")
}

fn normalizations() -> Result<()> {
    let mut f = Figure::new(IntendedUse::ArticleWide);
    f.height_mm = 82.0;
    let x = edges(180, 0.0, 6.0);
    let y = edges(110, 0.0, 4.0);
    let values = cell_values(&x, &y, |x, y| {
        0.08 * (4.0 * x - 3.0 * y).sin()
            + 8.0 * (-((x - 1.8).powi(2) + (y - 2.0).powi(2)) / 0.15).exp()
            - 8.0 * (-((x - 4.2).powi(2) + (y - 2.0).powi(2)) / 0.15).exp()
    });
    for (k, title, norm, limits, ticks) in [
        (
            0,
            "Linear",
            Normalization::Linear,
            [-10.0, 10.0],
            vec![-10.0, 0.0, 10.0],
        ),
        (
            1,
            "Signed asinh",
            Normalization::Asinh { width: 0.05 },
            [-10.0, 10.0],
            vec![-10.0, 0.0, 10.0],
        ),
        (
            2,
            "Log magnitude",
            Normalization::Log,
            [0.001, 10.0],
            vec![0.001, 0.1, 10.0],
        ),
    ] {
        let scale = f.scale(Scale::new(
            limits,
            norm,
            if k == 2 {
                ColorMap::sequential()
            } else {
                ColorMap::diverging()
            },
        )?);
        let rect = Rect::new(16.0 + k as f64 * 57.0, 14.0, 42.0, 27.0);
        let mut axes = Axes::new([0.0, 6.0], [0.0, 4.0], r"$x$", r"$y$");
        axes.xticks = vec![0.0, 3.0, 6.0];
        axes.yticks = vec![0.0, 2.0, 4.0];
        let mut p = Panel::new(rect, axes);
        let field = if k == 2 {
            values.iter().map(|v| v.abs()).collect()
        } else {
            values.clone()
        };
        p.pcolormesh(x.clone(), y.clone(), field, scale)?;
        f.panels.push(p);
        f.labels
            .push(Label::new(title, rect.x + rect.width / 2.0, 7.0));
        let mut bar = Colorbar::new(
            Rect::new(rect.x, 56.0, rect.width, 2.0),
            scale,
            if k == 2 { r"$|\varphi|$" } else { r"$\varphi$" },
            ticks,
        );
        bar.horizontal = true;
        f.colorbars.push(bar);
    }
    save(&f, "normalizations")
}

fn profile(use_case: IntendedUse, name: &str, title: &str) -> Result<()> {
    let mut f = Figure::new(use_case);
    let ids = f.grid(1, 1)?;
    let rect = f.panels[ids[0]].rect;
    let p = &mut f.panels[ids[0]];
    p.axes = Some(Axes::new([-4.0, 4.0], [-0.05, 1.1], r"$x$", r"$A/A_0$"));
    for (sigma, color, level) in [(1.0, "#245774", 0.88), (1.8, "#8b3940", 0.66)] {
        p.push(Layer::Line(Line::new(
            (0..501)
                .map(|i| {
                    let x = -4.0 + i as f64 / 500.0 * 8.0;
                    [x, (-0.5 * (x / sigma).powi(2)).exp()]
                })
                .collect(),
            color,
        )));
        let mut label = Label::new(
            format!("$\\sigma={sigma}$"),
            rect.x + rect.width * 0.79,
            rect.y + rect.height * (1.1 - level) / 1.15,
        );
        label.color = color.into();
        p.labels.push(label);
    }
    f.labels.push(Label::new(title, f.width_mm / 2.0, 4.8));
    save(&f, name)
}

fn index() -> Result<()> {
    let examples = [
        (
            "fields",
            "Field primitives",
            "pcolormesh, contourf, contour and independent color scales.",
            "examples/publication.rs",
        ),
        (
            "single_column",
            "Article column",
            "85 mm wide with 10 pt LaTeX type.",
            "examples/publication.rs",
        ),
        (
            "keogram",
            "Keogram with gaps",
            "Shared signed-asinh scale, masked samples and a propagation guide.",
            "examples/gallery.rs",
        ),
        (
            "curvilinear",
            "Curvilinear mesh",
            "Physical quadrilateral corners, with equal x/y scale.",
            "examples/gallery.rs",
        ),
        (
            "dispersion",
            "Mathematical annotations",
            "Acoustic and gravity branches of an analytic dispersion relation.",
            "examples/gallery.rs",
        ),
        (
            "normalizations",
            "Color normalization",
            "Linear, signed-asinh and logarithmic magnitude views.",
            "examples/gallery.rs",
        ),
        (
            "mobile",
            "Mobile preset",
            "13 pt type at mobile-page width.",
            "examples/gallery.rs",
        ),
        (
            "desktop",
            "Desktop half-window",
            "12 pt type for a wider on-screen figure.",
            "examples/gallery.rs",
        ),
    ];
    let mut html = String::from(
        r#"<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>rustplotlib gallery</title><style>body{font:18px Georgia,serif;max-width:1200px;margin:30px auto;padding:0 22px;color:#212329}h1,h2{font-weight:normal}header{display:flex;justify-content:space-between;align-items:baseline}a{color:#315769;text-decoration:none}main{display:grid;grid-template-columns:1fr 1fr;gap:36px}article{min-width:0}img{width:100%;height:auto}p{line-height:1.45;color:#50545a}.links{font:15px system-ui;display:flex;gap:15px;flex-wrap:wrap}@media(max-width:720px){main{grid-template-columns:1fr}}</style><header><h1>rustplotlib gallery</h1><a href="../">Interactive viewer</a></header><p>LaTeX typography. Rust rendering. Native figures, numerical simulations and measured HEIMDALL data.</p><main>"#,
    );
    let mut md = String::from(
        "# Gallery\n\nGenerate the additional examples with `cargo run --release --example gallery`.\nNative figures are exported at 300 dpi and their intended physical size. HEIMDALL uses measured VTEC; the atmospheric cutaways use analytic fields.\n\n",
    );
    for (name, title, description, source) in examples {
        html.push_str(&format!(r#"<article><h2>{title}</h2><a href="{name}.pdf"><img loading="lazy" src="{name}.png" alt="{title}"></a><p>{description}</p><div class="links"><a href="{name}.pdf">PDF</a><a href="{name}.png">PNG</a><a href="{name}.svg">SVG</a><a href="https://github.com/jvierine/rustplotlib/blob/main/{source}">Rust source</a></div></article>"#));
        md.push_str(&format!("## {title}\n\n{description}\n\n![{title}]({name}.png)\n\n[PDF]({name}.pdf) · [PNG]({name}.png) · [SVG]({name}.svg) · [Source](../{source})\n\n"));
    }
    for (name, title, description, url, source) in [
        (
            "secondary_axis",
            "Reciprocal secondary axis",
            "An analytic buoyancy profile with height vertically and period on the top axis; not an MSIS atmosphere.",
            "secondary_axis.pdf",
            "examples/secondary_axis.rs",
        ),
        (
            "khi_summary",
            "Small multiples and diagnostics",
            "KHI evolution with directly labelled energy and enstrophy.",
            "khi_summary.pdf",
            "examples/khi_summary.rs",
        ),
        (
            "surfaces",
            "Functions of two variables",
            "Depth-tested, shaded 3-D surfaces.",
            "surfaces.pdf",
            "examples/surfaces.rs",
        ),
        (
            "dimant_oppenheim",
            "Dimant--Oppenheim density",
            "Logarithmic density cross-section and directly labelled profiles.",
            "dimant_oppenheim.pdf",
            "examples/dimant_oppenheim.rs",
        ),
        (
            "dimant_surface",
            "Meteor density surface",
            "Log-height surface of the same density function.",
            "dimant_surface.pdf",
            "examples/dimant_oppenheim.rs",
        ),
        (
            "khi",
            "Kelvin--Helmholtz instability",
            "Periodic viscous Navier--Stokes with passive dye.",
            "khi.gif",
            "examples/khi.rs",
        ),
        (
            "heimdall",
            "HEIMDALL cartography",
            "Measured VTEC; shared HEIMDALL WebGL2 renderer, globe and flat-map views.",
            "../heimdall.html",
            "web/src/heimdall.ts",
        ),
        (
            "currents",
            "GAMERA current paths",
            "Traced current-density paths with a scientific PNG/PDF export.",
            "../currents.html",
            "examples/scientific_views.rs",
        ),
        (
            "smile",
            "SMILE aurora on Earth",
            "Projected UVI above a textured Earth; rotate the interactive globe.",
            "../smile.html",
            "examples/scientific_views.rs",
        ),
        (
            "tiegcm",
            "Curved atmospheric sections",
            "Completed TIE-GCM heating-pulse experiment, compared with its control at common geometric heights.",
            "tiegcm.pdf",
            "examples/tiegcm.rs",
        ),
        (
            "tufte_mixing",
            "Mixing atlas",
            "KHI snapshots, aligned profiles, small multiples and inline diagnostics.",
            "tufte_mixing.pdf",
            "examples/visual_essays.rs",
        ),
        (
            "tufte_meteor",
            "Meteor wake anatomy",
            "Density, marginal profile and cumulative footprint of the Dimant–Oppenheim function.",
            "tufte_meteor.pdf",
            "examples/visual_essays.rs",
        ),
        (
            "tufte_wave",
            "One wave, twelve clocks",
            "Stacked analytic receiver traces with a shared arrival guide.",
            "tufte_wave.pdf",
            "examples/visual_essays.rs",
        ),
        (
            "atmosphere",
            "Spherical cutaways",
            "Interactive temperature, density and wind demonstration fields.",
            "../atmosphere.html",
            "examples/atmosphere.rs",
        ),
    ] {
        let image = if name == "khi" {
            "khi.gif".to_string()
        } else {
            format!("{name}.png")
        };
        let download = if ["heimdall", "atmosphere", "smile", "currents"].contains(&name) {
            format!(r#"<a href="{name}.pdf">Scientific PDF</a>"#)
        } else {
            String::new()
        };
        html.push_str(&format!(r#"<article><h2>{title}</h2><a href="{url}"><img src="{image}" alt="{title}"></a><p>{description}</p><div class="links"><a href="{url}">Open example</a>{download}<a href="https://github.com/jvierine/rustplotlib/blob/main/{source}">Source</a></div></article>"#));
        md.push_str(&format!(
            "## {title}\n\n![{title}]({image})\n\n{description} [Source](../{source})\n\n"
        ));
    }
    for (name, title, description) in [
        (
            "syisr_khi_density",
            "KHI electron-density evolution",
            "Two-hour Ri=0.08 model, following the mean zonal drift; no field clipping.",
        ),
        (
            "syisr_khi_ion_up",
            "Ion drift and background shear",
            "The 60-minute Ri=0.08 snapshot: upward magnetic ion drift with an equal-height background-wind profile, LaTeX labels and a signed color scale. Animation below. Simulated, not an observed fit.",
        ),
    ] {
        let snapshot = if name == "syisr_khi_ion_up" {
            format!(
                r#"<a href="{name}.pdf"><img src="{name}.png" alt="Upward ion drift at 60 minutes and aligned background wind"></a>"#
            )
        } else {
            String::new()
        };
        html.push_str(&format!(r#"<article><h2>{title}</h2>{snapshot}<video controls playsinline preload="metadata" poster="{name}.png" style="width:100%;height:auto"><source src="{name}.mp4" type="video/mp4"></video><p>{description}</p><div class="links"><a href="{name}.mp4">MP4</a><a href="{name}.pdf">Scientific PDF</a><a href="https://github.com/jvierine/rustplotlib/blob/main/examples/syisr_khi.rs">Rust source</a></div></article>"#));
        md.push_str(&format!("## {title}\n\n![{title}]({name}.png)\n\n{description} [MP4]({name}.mp4) · [PDF]({name}.pdf) · [Rust source](../examples/syisr_khi.rs)\n\n"));
    }
    html.push_str("</main></html>");
    fs::write("gallery/index.html", html)?;
    fs::write("gallery/README.md", md.trim_end().to_owned() + "\n")?;
    Ok(())
}

fn main() -> Result<()> {
    fs::create_dir_all("gallery")?;
    keogram()?;
    curvilinear()?;
    dispersion()?;
    normalizations()?;
    profile(IntendedUse::Mobile, "mobile", "Mobile: 13 pt")?;
    profile(
        IntendedUse::DesktopHalf,
        "desktop",
        "Desktop half-window: 12 pt",
    )?;
    index()?;
    Ok(())
}
