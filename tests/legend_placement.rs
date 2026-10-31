use rustplotlib::*;
fn legend_rect(svg: &str) -> Rect {
    let tail = svg.split("data-role=\"legend\"").nth(1).unwrap();
    let tag = tail
        .split("<rect ")
        .nth(1)
        .unwrap()
        .split("/>")
        .next()
        .unwrap();
    let read = |key: &str| {
        tag.split(&format!("{key}=\""))
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse::<f64>()
            .unwrap()
    };
    Rect::new(read("x"), read("y"), read("width"), read("height"))
}
fn contains(p: Rect, r: Rect) -> bool {
    r.x >= p.x && r.y >= p.y && r.x + r.width <= p.x + p.width && r.y + r.height <= p.y + p.height
}
fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}
fn figure() -> Figure {
    let mut f = Figure::new(IntendedUse::ArticleWide);
    f.height_mm = 160.;
    f.tight_export = false;
    f.panels.push(Panel::new(
        Rect::new(25., 45., 120., 85.),
        Axes::new([0., 1.], [0., 1.], "x", "y"),
    ));
    f
}
#[test]
fn legacy_above_center_now_uses_empty_axes_space() {
    let mut f = figure();
    let p = &mut f.panels[0];
    p.push(Layer::Line(Line::new(vec![[0., 0.1], [1., 0.1]], "blue")));
    let mut l = Legend::new(vec![LegendEntry::line("Model", "blue")]);
    l.position = LegendPosition::AboveCenter;
    p.legend = Some(l);
    let r = legend_rect(&f.svg().unwrap());
    assert!(contains(f.panels[0].rect, r));
}
#[test]
fn wide_preferred_columns_reflow_inside_before_using_outside_space() {
    let mut f = figure();
    f.panels[0].rect.width = 70.;
    let mut l = Legend::new(vec![
        LegendEntry::line("Horizontal component", "blue"),
        LegendEntry::line("Vertical component", "red"),
        LegendEntry::line("Total field", "green"),
    ]);
    l.columns = 3;
    f.panels[0].legend = Some(l);
    let r = legend_rect(&f.svg().unwrap());
    assert!(contains(f.panels[0].rect, r));
    assert!(r.height > 15.);
}
#[test]
fn inside_legend_avoids_annotation_background() {
    let mut f = figure();
    let p = &mut f.panels[0];
    p.label_at("Important observation", [0.8, 0.9]);
    p.legend = Some(Legend::new(vec![LegendEntry::line("Model", "blue")]));
    let annotation = [p.labels[0].x, p.labels[0].y];
    let r = legend_rect(&f.svg().unwrap());
    assert!(contains(f.panels[0].rect, r));
    assert!(!contains(
        r,
        Rect::new(annotation[0] - 0.1, annotation[1] - 0.1, 0.2, 0.2)
    ));
}
#[test]
fn full_axes_use_safe_external_space_away_from_secondary_labels() {
    let mut f = figure();
    let scale = f.scale(Scale::new([0., 1.], Normalization::Linear, ColorMap::ember()).unwrap());
    let p = &mut f.panels[0];
    p.axes.as_mut().unwrap().secondary_x =
        Some(SecondaryAxis::new("Secondary coordinate", vec![0., 0.5, 1.], |x| x).unwrap());
    p.push(Layer::Pcolormesh(
        Field::new(vec![0., 1.], vec![0., 1.], vec![0.5], scale).unwrap(),
    ));
    p.legend = Some(Legend::new(vec![LegendEntry::line("Model", "blue")]));
    let r = legend_rect(&f.svg().unwrap());
    assert!(!overlaps(f.panels[0].rect, r));
    assert!(contains(Rect::new(0., 0., f.width_mm, f.height_mm), r));
}
#[test]
fn explicit_outside_request_stays_outside() {
    let mut f = figure();
    let mut l = Legend::new(vec![LegendEntry::line("Model", "blue")]);
    l.position = LegendPosition::OutsideAboveCenter;
    f.panels[0].legend = Some(l);
    let r = legend_rect(&f.svg().unwrap());
    assert!(r.y + r.height < f.panels[0].rect.y);
}
