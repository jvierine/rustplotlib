use anyhow::Result;
use rustplotlib::*;

fn main() -> Result<()> {
    let mut f = ArticleLayout::new(177., 10.)?.figure(1., 88.)?;
    let ids = f.grid(1, 2)?;
    f.width_mm = 177.;
    f.height_mm = 88.;
    for (j, &p) in ids.iter().enumerate() {
        f.panels[p].rect = Rect::new(20. + j as f64 * 88., 6., 64., 65.);
        f.panels[p].axes = Some(Axes::new([0., 11.], [0., 7.], "$x$", "$y$"));
        for i in 0..6 {
            let points = (0..200)
                .map(|k| {
                    let x = k as f64 * 6. / 199.;
                    [x, 6. - i as f64 + 0.3 * (1.5 * x).sin()]
                })
                .collect();
            let label = format!("{}", i + 1);
            if j == 0 {
                f.panels[p].plot(points, label);
            } else {
                let mut line = Line::series(points, i);
                line.color = "#333333".into();
                let key = line.legend(label);
                f.panels[p].push(Layer::Line(line));
                if let Some(legend) = &mut f.panels[p].legend {
                    legend.entries.push(key);
                } else {
                    f.panels[p].legend = Some(Legend::new(vec![key]));
                }
            }
        }
        f.panels[p].legend.as_mut().unwrap().position = LegendPosition::CenterRight;
    }
    for ext in ["svg", "png"] {
        f.save(format!("gallery/default_line_styles.{ext}"))?;
    }
    Ok(())
}
