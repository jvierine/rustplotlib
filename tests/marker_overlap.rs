use rustplotlib::*;

#[test]
fn coincident_categories_both_survive_raster_export_and_have_open_legend_keys() {
    let mut f = Figure::new(IntendedUse::ArticleWide);
    f.width_mm = 100.;
    f.height_mm = 80.;
    f.tight_export = false;
    f.raster_dpi = 144.;
    let mut p = Panel::new(
        Rect::new(20., 10., 60., 50.),
        Axes::new([0., 2.], [0., 2.], "x", "y"),
    );
    let mut legend = p
        .marker_groups(&[
            MarkerGroup {
                label: "Circle",
                points: &[[1., 1.]],
                style: MarkerStyle {
                    shape: MarkerShape::Circle,
                    size_pt: 12.,
                    color: [0., 0., 1.],
                },
            },
            MarkerGroup {
                label: "Diamond",
                points: &[[1., 1.]],
                style: MarkerStyle {
                    shape: MarkerShape::Diamond,
                    size_pt: 12.,
                    color: [1., 0., 0.],
                },
            },
        ])
        .unwrap();
    legend.position = LegendPosition::LowerLeft;
    p.legend = Some(legend);
    f.panels.push(p);
    let svg = f.svg().unwrap();
    let key = svg.split("data-role=\"legend\"").nth(1).unwrap();
    assert!(key.contains("fill=\"none\" stroke=\"#0000ff\""));
    assert!(key.contains("fill=\"none\" stroke=\"#ff0000\""));
    let path = "target/coincident-markers.png";
    f.save(path).unwrap();
    let im = image::open(path).unwrap().to_rgb8();
    let cx = (50. * 144. / 25.4) as u32;
    let cy = (35. * 144. / 25.4) as u32;
    let (mut red, mut blue) = (0, 0);
    for y in cy - 18..cy + 19 {
        for x in cx - 18..cx + 19 {
            let p = im.get_pixel(x, y).0;
            if p[0] > 180 && p[1] < 100 && p[2] < 100 {
                red += 1;
            }
            if p[2] > 180 && p[0] < 100 && p[1] < 100 {
                blue += 1;
            }
        }
    }
    assert!(
        red > 20 && blue > 20,
        "A category disappeared: red={red}, blue={blue}"
    );
    assert_eq!(
        im.get_pixel(cx, cy).0,
        [255, 255, 255],
        "Marker interiors must be transparent"
    );
}
