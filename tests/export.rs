use rustplotlib::*;
use std::path::PathBuf;
#[test]
fn listed_colorbars_avoid_large_pdf_stitching_functions() {
    let mut f = Figure::new(IntendedUse::ArticleWide);
    let scale = f.scale(Scale::new([0., 1.], Normalization::Linear, ColorMap::inferno()).unwrap());
    let mut bar = Colorbar::new(
        Rect::new(20., 15., 100., 3.),
        scale,
        "Intensity",
        vec![0., 1.],
    );
    bar.horizontal = true;
    f.colorbars.push(bar);
    let svg = f.svg().unwrap();
    assert!(!svg.contains("linearGradient"));
    assert!(svg.contains("data:image/png;base64,"));
    f.save("target/listed-colorbar.pdf").unwrap();
}
#[test]
fn routed_connector_and_calendar_ticks_export() {
    let mut f = Figure::new(IntendedUse::ArticleWide);
    f.tight_export = false;
    let mut a = Axes::new([0., 24.], [0., 1.], "UT", "");
    a.xticks = vec![0., 12., 24.];
    a.xticklabels = Some(vec!["06".into(), "18".into(), "06".into()]);
    f.panels.push(Panel::new(Rect::new(20., 15., 50., 25.), a));
    f.panels.push(Panel::new(
        Rect::new(100., 65., 50., 25.),
        Axes::new([0., 1.], [0., 1.], "", ""),
    ));
    f.connectors.push(Connector {
        from_panel: 0,
        from: [24., 1.],
        to_panel: 1,
        to: [0., 1.],
        via: vec![[85., 15.], [85., 65.]],
        color: "#008080".into(),
        width_pt: 0.5,
    });
    let s = f.svg().unwrap();
    assert!(s.contains("data-role=\"panel-connector\" d=\"M 70 15 L 85 15 L 85 65 L 100 65\""));
    f.save("target/connector-test.pdf").unwrap();
    f.save("target/connector-test.png").unwrap();
    f.connectors[0].from_panel = 4;
    assert!(
        f.svg()
            .unwrap_err()
            .to_string()
            .contains("Unknown connector panel")
    );
}
#[test]
fn real_latex_cache_and_all_export_formats() {
    let root = PathBuf::from("target/export-test");
    std::fs::create_dir_all(&root).unwrap();
    let mut f = Figure::new(IntendedUse::ArticleColumn);
    f.tight_export = false;
    f.latex.cache = root.join("latex");
    let ids = f.grid(1, 1).unwrap();
    f.panels[ids[0]].axes = Some(Axes::new(
        [0.0, 2.0],
        [-1.0, 1.0],
        r"$t\,[\mathrm{s}]$",
        r"$\frac{\partial p}{\partial t}$",
    ));
    f.panels[0].push(Layer::Line(Line::new(
        vec![
            [0.0, 0.0],
            [1.0, 1.0],
            [f64::NAN, f64::NAN],
            [1.5, -0.5],
            [2.0, 0.0],
        ],
        "#245774",
    )));
    for ext in ["svg", "pdf", "png", "jpg"] {
        f.save(root.join(format!("math.{ext}"))).unwrap();
    }
    let svg = std::fs::read_to_string(root.join("math.svg")).unwrap();
    assert!(svg.contains("<path"));
    assert!(!svg.contains("<text"));
    assert!(
        std::fs::read(root.join("math.pdf"))
            .unwrap()
            .starts_with(b"%PDF")
    );
    let pdf = String::from_utf8_lossy(&std::fs::read(root.join("math.pdf")).unwrap()).into_owned();
    let media = pdf
        .split("/MediaBox")
        .nth(1)
        .unwrap()
        .split('[')
        .nth(1)
        .unwrap()
        .split(']')
        .next()
        .unwrap();
    let dimensions: Vec<f64> = media
        .split_whitespace()
        .map(|v| v.parse().unwrap())
        .collect();
    assert!(
        (dimensions[2] - 85.0 / 25.4 * 72.0).abs() < 0.01,
        "PDF must preserve intended physical width"
    );
    let n = f.latex.cached_label_count();
    assert!(n > 4);
    f.svg().unwrap();
    assert_eq!(n, f.latex.cached_label_count());
    let png = image::open(root.join("math.png")).unwrap();
    let jpg = image::open(root.join("math.jpg")).unwrap();
    assert_eq!((png.width(), png.height()), (jpg.width(), jpg.height()));
    assert!(png.width() > 900);
}

#[test]
fn cropped_and_overlapping_labels_are_rejected_by_default() {
    let mut f = Figure::new(IntendedUse::ArticleColumn);
    f.tight_export = false;
    f.latex.cache = PathBuf::from("target/layout-test-latex");
    f.labels.push(Label::new(r"$\omega$", 0.0, 0.0));
    assert!(f.svg().unwrap_err().to_string().contains("cropped"));
    f.labels.clear();
    f.labels.push(Label::new(r"$\omega$", 30.0, 20.0));
    f.labels.push(Label::new(r"$p$", 30.0, 20.0));
    assert!(f.svg().unwrap_err().to_string().contains("overlap"));
    f.check_label_overlap = false;
    assert!(f.svg().is_ok());
}

#[test]
fn pdf_colorbar_matches_raster_colorbar() {
    // Poppler is optional locally, and installed by the CI workflow.
    if std::process::Command::new("pdftoppm")
        .arg("-v")
        .output()
        .is_err()
    {
        return;
    }
    let root = PathBuf::from("target/colorbar-test");
    std::fs::create_dir_all(&root).unwrap();
    let mut f = Figure::new(IntendedUse::ArticleColumn);
    f.tight_export = false;
    let scale =
        f.scale(Scale::new([-1.0, 1.0], Normalization::Linear, ColorMap::diverging()).unwrap());
    let mut bar = Colorbar::new(Rect::new(10.0, 10.0, 60.0, 3.0), scale, "", vec![]);
    bar.horizontal = true;
    f.colorbars.push(bar);
    f.save(root.join("bar.pdf")).unwrap();
    f.save(root.join("bar.png")).unwrap();
    let status = std::process::Command::new("pdftoppm")
        .args(["-r", "300", "-png", "-singlefile"])
        .arg(root.join("bar.pdf"))
        .arg(root.join("pdf"))
        .status()
        .unwrap();
    assert!(status.success());
    let a = image::open(root.join("bar.png")).unwrap().to_rgb8();
    let b = image::open(root.join("pdf.png")).unwrap().to_rgb8();
    for t in [0.01, 0.25, 0.5, 0.75, 0.99] {
        let sample = |im: &image::RgbImage| {
            *im.get_pixel(
                ((10.0 + 60.0 * t) / f.width_mm * im.width() as f64).round() as u32,
                (11.5 / f.height_mm * im.height() as f64).round() as u32,
            )
        };
        let (x, y) = (sample(&a), sample(&b));
        for k in 0..3 {
            assert!(
                (x[k] as i16 - y[k] as i16).abs() <= 3,
                "PDF changed the color scale: {t}: {x:?} vs {y:?}"
            );
        }
    }
}

#[test]
fn axis_labels_and_colorbars_cannot_overlap_unless_forced() {
    let root = PathBuf::from("target/geometry-layout-test");
    std::fs::create_dir_all(&root).unwrap();
    let mut f = Figure::new(IntendedUse::ArticleColumn);
    f.tight_export = false;
    f.height_mm = 110.;
    let sc = f.scale(Scale::new([0., 1.], Normalization::Linear, ColorMap::estuary()).unwrap());
    f.panels.push(Panel::new(
        Rect::new(15., 12., 60., 60.),
        Axes::new([0., 6.], [0., 6.], "$x$", "$y$"),
    ));
    let mut bar = Colorbar::new(Rect::new(15., 79., 60., 1.5), sc, "", vec![]);
    bar.horizontal = true;
    f.colorbars.push(bar);
    let err = f.save(root.join("bad.png")).unwrap_err().to_string();
    assert!(err.contains("$x$") && err.contains("overlaps colorbar"));
    assert!(
        !root.join("bad.png").exists(),
        "An invalid layout must not write an export"
    );
    f.check_label_geometry = false;
    assert!(
        f.svg().is_ok(),
        "Explicit override must allow intentional overlap"
    );
    f.check_label_geometry = true;
    f.colorbars[0].rect.y = 89.;
    assert!(f.svg().is_ok());
    f.panels.push(Panel::new(
        Rect::new(35., 78., 20., 8.),
        Axes::new([0., 1.], [0., 1.], "", ""),
    ));
    let err = f.svg().unwrap_err().to_string();
    assert!(err.contains("Axis label") && err.contains("overlaps data panel"));
}

#[test]
fn tight_export_is_default_and_preserves_colorbar_labels() {
    let root = PathBuf::from("target/tight-test");
    std::fs::create_dir_all(&root).unwrap();
    let mut f = Figure::new(IntendedUse::ArticleColumn);
    assert!(f.tight_export);
    let sc = f.scale(Scale::new([0., 1.], Normalization::Linear, ColorMap::ember()).unwrap());
    let mut bar = Colorbar::new(Rect::new(25., 20., 35., 2.), sc, "Density", vec![0., 1.]);
    bar.horizontal = true;
    f.colorbars.push(bar);
    f.save(root.join("tight.png")).unwrap();
    f.save(root.join("tight.svg")).unwrap();
    let tight = image::open(root.join("tight.png")).unwrap();
    f.tight_export = false;
    f.save(root.join("full.png")).unwrap();
    let full = image::open(root.join("full.png")).unwrap();
    assert!(tight.width() < full.width());
    assert!(tight.height() < full.height() / 2);
    let s = std::fs::read_to_string(root.join("tight.svg")).unwrap();
    assert!(s.contains("viewBox="));
    assert!(s.contains("<path"));
}

#[test]
fn raster_layer_keeps_native_typesetting_and_rejects_empty_images() {
    let mut f = Figure::new(IntendedUse::ArticleColumn);
    let mut p = Panel::new(
        Rect::new(10., 10., 60., 30.),
        Axes::new([0., 1.], [0., 1.], "", ""),
    );
    p.axes_visible = false;
    p.push(Layer::Image(image::RgbaImage::from_pixel(
        2,
        2,
        image::Rgba([120, 40, 20, 255]),
    )));
    f.panels.push(p);
    f.labels.push(Label::new(r"$n_e$", 40., 5.));
    let svg = f.svg().unwrap();
    assert!(svg.contains("data:image/png;base64,"));
    assert!(svg.contains("<path"));
    f.panels[0].layers[0] = Layer::Image(image::RgbaImage::new(0, 0));
    assert!(f.svg().unwrap_err().to_string().contains("Empty raster"));
}
