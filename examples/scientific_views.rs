//! WebGL data views with native LaTeX labels; no browser interface in exports.
use anyhow::Result;
use rustplotlib::*;
use std::path::Path;
fn main() -> Result<()> {
    for name in ["smile", "currents"] {
        let mut f = Figure::new(if name == "smile" {
            IntendedUse::ArticleColumn
        } else {
            IntendedUse::ArticleWide
        });
        let w = if name == "smile" { 79. } else { 166. };
        let h = if name == "smile" {
            w * 0.8
        } else {
            w * 700. / 1200.
        };
        f.height_mm = h + 35.;
        let mut p = Panel::new(
            Rect::new(3., 20., w, h),
            Axes::new([0., 1.], [0., 1.], "", ""),
        );
        p.axes_visible = false;
        p.push(Layer::Image(
            image::open(format!("output/{name}-canvas.png"))?.to_rgba8(),
        ));
        f.panels.push(p);
        let (title, epoch) = if name == "smile" {
            ("SMILE UVI", "24 July 2026, 00:25:08 UTC")
        } else {
            ("GAMERA current paths", "9 August 2016, 20:47:00 UTC")
        };
        f.labels.push(Label::new(title, f.width_mm / 2., 5.));
        f.labels.push(Label::new(epoch, f.width_mm / 2., 13.));
        if name == "currents" {
            // Orthographic camera: 34 Earth radii per half-height, aspect 1200/700.
            let mut bar = Line::new(
                vec![[0.80, 0.10], [0.80 + 10. / (68. * 1200. / 700.), 0.10]],
                "#27353e",
            );
            bar.width_pt = 1.0;
            f.panels[0].push(Layer::Line(bar));
            f.labels.push(Label::new(
                r"$10R_E$",
                3. + w * (0.80 + 5. / (68. * 1200. / 700.)),
                20. + h * 0.9 - 3.,
            ));
            for (i, (title, color)) in [
                ("Dayside", "#198c52"),
                ("Tail", "#2961b8"),
                ("Ring region", "#a63ba6"),
                ("Region 1-like", "#cc4040"),
                ("Region 2-like", "#0f9494"),
            ]
            .iter()
            .enumerate()
            {
                let mut l = Label::new(*title, 20. + i as f64 * 34., h + 27.);
                l.color = (*color).into();
                f.labels.push(l);
            }
        }
        for ext in ["png", "pdf", "svg"] {
            f.save(Path::new("gallery").join(format!("{name}.{ext}")))?;
        }
    }
    if Path::new("output/heimdall-canvas.png").exists() {
        let mut f = Figure::new(IntendedUse::ArticleColumn);
        f.height_mm = 105.;
        let mut p = Panel::new(
            Rect::new(3., 10., 79., 79.),
            Axes::new([0., 1.], [0., 1.], "", ""),
        );
        p.axes_visible = false;
        p.push(Layer::Image(
            image::open("output/heimdall-canvas.png")?.to_rgba8(),
        ));
        f.panels.push(p);
        f.labels
            .push(Label::new("25 September 2025, 12:59 UTC", 42.5, 5.));
        let palette: Vec<u8> =
            serde_json::from_slice(&std::fs::read("output/heimdall-palette.json")?)?;
        let colors = palette
            .chunks_exact(3)
            .map(|p| [p[0] as f64 / 255., p[1] as f64 / 255., p[2] as f64 / 255.])
            .collect();
        let sc = f.scale(Scale::new(
            [1., 40.],
            Normalization::Log,
            ColorMap::listed(colors)?,
        )?);
        let mut b = Colorbar::new(
            Rect::new(8., 88., 69., 1.5),
            sc,
            r"Vertical TEC $[\mathrm{TECU}]$",
            vec![1., 2., 5., 10., 20., 40.],
        );
        b.horizontal = true;
        f.colorbars.push(b);
        for ext in ["png", "pdf", "svg"] {
            f.save(format!("gallery/heimdall.{ext}"))?;
        }
    }
    if Path::new("output/temperature-canvas.png").exists() {
        let mut f = Figure::new(IntendedUse::ArticleWide);
        f.height_mm = 88.;
        f.labels
            .push(Label::new("Spherical atmospheric cutaways", 89., 5.));
        let specs = [
            (
                "temperature",
                "Temperature",
                [250., 1100.],
                r"$T\;[\mathrm K]$",
                vec![
                    [0.08, 0.12, 0.35],
                    [0.20, 0.50, 0.70],
                    [0.96, 0.80, 0.32],
                    [0.75, 0.17, 0.15],
                ],
            ),
            (
                "electron_density",
                "Electron density",
                [0., 1e12],
                r"$N_e\;[\mathrm{m^{-3}}]$",
                vec![
                    [0.07, 0.08, 0.15],
                    [0.18, 0.36, 0.30],
                    [0.59, 0.56, 0.75],
                    [0.98, 0.98, 0.96],
                ],
            ),
            (
                "zonal_wind",
                "Zonal wind",
                [-200., 200.],
                r"$u_e\;[\mathrm{m\,s^{-1}}]$",
                vec![[0.12, 0.23, 0.61], [0.97, 0.96, 0.92], [0.69, 0.10, 0.16]],
            ),
        ];
        for (i, (name, title, limits, label, colors)) in specs.into_iter().enumerate() {
            let x = 3. + i as f64 * 59.;
            f.labels.push(Label::new(
                format!("{}) {title}", ['a', 'b', 'c'][i]),
                x + 26.,
                13.,
            ));
            let mut p = Panel::new(
                Rect::new(x, 18., 52., 52.),
                Axes::new([0., 1.], [0., 1.], "", ""),
            );
            p.axes_visible = false;
            p.push(Layer::Image(
                image::open(format!("output/{name}-canvas.png"))?.to_rgba8(),
            ));
            f.panels.push(p);
            let sc = f.scale(Scale::new(
                limits,
                Normalization::Linear,
                ColorMap::new(colors)?,
            )?);
            let mut bar = Colorbar::new(
                Rect::new(x + 5., 71., 42., 2.),
                sc,
                label,
                vec![limits[0], limits[1]],
            );
            bar.horizontal = true;
            f.colorbars.push(bar);
        }
        for ext in ["png", "pdf", "svg"] {
            f.save(format!("gallery/atmosphere.{ext}"))?;
        }
    }
    Ok(())
}
