use rustplotlib::*;

fn panel() -> Panel {
    Panel::new(
        Rect::new(20., 10., 60., 45.),
        Axes::new([0., 10.], [0., 10.], "", ""),
    )
}

#[test]
fn defaults_distinguish_series_and_keep_legends_honest() {
    let mut p = panel();
    p.push(Layer::Line(Line::new(vec![[0., 0.], [10., 0.]], "#aaaaaa")));
    for i in 0..6 {
        p.plot(vec![[0., i as f64], [3., i as f64]], format!("Series {i}"));
    }
    let lines: Vec<_> = p
        .layers
        .iter()
        .skip(1)
        .map(|l| match l {
            Layer::Line(l) => l,
            _ => unreachable!(),
        })
        .collect();
    for (i, line) in lines.iter().enumerate() {
        for other in lines.iter().take(i) {
            assert_ne!(line.color, other.color);
            assert_ne!(
                line.dash, other.dash,
                "Series must remain distinct in grayscale"
            );
        }
        match &p.legend.as_ref().unwrap().entries[i].symbol {
            LegendSymbol::Line {
                color,
                width_pt,
                dash,
            } => {
                assert_eq!(color, &line.color);
                assert_eq!(*width_pt, line.width_pt);
                assert_eq!(dash, &line.dash);
            }
            _ => panic!("Expected line legend"),
        }
    }
    assert_eq!(lines[0].color, "#0072B2");
    assert!(lines[0].dash.is_none());
    let mut second = panel();
    second.plot(vec![[0., 0.], [1., 1.]], "New panel");
    if let Layer::Line(line) = &second.layers[0] {
        assert_eq!(line.color, lines[0].color);
    }
    let explicit = Line::new(vec![], "#abcdef");
    assert_eq!(explicit.color, "#abcdef");
    assert!(explicit.dash.is_none());
}

#[test]
fn dashed_series_and_legend_export_with_the_same_pattern() {
    let mut f = Figure::new(IntendedUse::ArticleColumn);
    f.width_mm = 100.;
    f.height_mm = 80.;
    let mut p = panel();
    p.plot(vec![[0., 1.], [2., 1.]], "A")
        .plot(vec![[0., 2.], [2., 2.]], "B");
    f.panels.push(p);
    let svg = f.svg().unwrap();
    assert_eq!(svg.matches("stroke-dasharray=\"2.4 1.2\"").count(), 2);
    assert!(svg.matches("#D55E00").count() >= 2);
}
