use rustplotlib::*;
use std::path::PathBuf;
#[test]
fn real_latex_cache_and_all_export_formats() {
    let root = PathBuf::from("target/export-test");
    std::fs::create_dir_all(&root).unwrap();
    let mut f = Figure::new(IntendedUse::ArticleColumn);
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
