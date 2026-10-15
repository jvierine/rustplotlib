//! Same body-sized labels in desktop and mobile layouts, without scaling.
use rustplotlib::*;
use std::{fs, path::PathBuf};

fn main() -> anyhow::Result<()> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "output/web-typography".into()),
    );
    fs::create_dir_all(&out)?;
    let mut figures = Vec::new();
    for (name, width) in [("desktop", 720.), ("mobile", 340.)] {
        let mut f = WebLayout::new(width, 16.)?.figure(300.)?;
        f.latex.cache = out.join("latex-cache");
        let ids = f.grid(1, 1)?;
        let p = &mut f.panels[ids[0]];
        p.axes = Some(Axes::new([0., 6.], [-1., 1.], "Time (s)", "Amplitude"));
        p.axes.as_mut().unwrap().xticks = vec![0., 2., 4., 6.];
        p.axes.as_mut().unwrap().yticks = vec![-1., 0., 1.];
        p.push(Layer::Line(Line::new(
            (0..200)
                .map(|i| {
                    let t = i as f64 * 6. / 199.;
                    [t, t.sin()]
                })
                .collect(),
            "#236b8e",
        )));
        f.save_for_web(
            out.join(format!("{name}.svg")),
            &format!("{name}.svg"),
            "Amplitude versus time",
        )?;
        figures.push(f.html_embed(&format!("{name}.svg"), "Amplitude versus time")?);
    }
    fs::write(
        out.join("index.html"),
        format!(
            r#"<!doctype html>
<html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Rustplotlib web typography</title><style>
body {{ margin:24px; font:16px Georgia,serif; }}
.mobile {{ display:none; }}
@media(max-width:600px) {{ .desktop {{ display:none; }} .mobile {{ display:block; }} }}
</style><h1>Web typography</h1><p>Amplitude as a function of time.</p>
<section class="desktop">{}</section><section class="mobile">{}</section></html>"#,
            figures[0], figures[1]
        ),
    )?;
    Ok(())
}
