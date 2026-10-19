//! Real TeX regression coverage for the default AMS preamble and shared glyph
//! pipeline. Cache directories are fresh so an old SVG cannot hide a failure.
use rustplotlib::Latex;
use std::{path::PathBuf, time::SystemTime};

fn fresh_latex(name: &str) -> Latex {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    Latex {
        cache: PathBuf::from(format!(
            "target/ams-latex/{name}-{}-{nonce}",
            std::process::id()
        )),
        ..Latex::default()
    }
}

#[test]
fn default_preamble_compiles_ams_equations_and_symbols() {
    let latex = fresh_latex("environments");
    assert!(latex.preamble.contains(r"\usepackage{amsmath,amssymb}"));
    let labels = [
        r"$\begin{aligned}\partial_t n+\nabla\cdot(n\boldsymbol{u})&=S-L\\ \rho D_t\boldsymbol{u}&=-\nabla p+\boldsymbol{f}\end{aligned}$",
        r"$\begin{pmatrix}u\\v\\w\end{pmatrix}\in\mathbb{R}^{3},\quad\begin{bmatrix}1&0\\0&1\end{bmatrix}=\mathbf{I}$",
        r"$S(t)=\begin{cases}S_0\exp(-t/\tau),&t\geq0,\\0,&t<0.\end{cases}$",
        r"$\begin{gathered}\boldsymbol{u}=\nabla\phi\\\nabla\times\boldsymbol{u}=\boldsymbol{0}\end{gathered}$",
        r"$\lVert\boldsymbol{u}\rVert_2\leq U_{\max},\quad\operatorname{Re}\lambda\leq0$",
        r"$\mathbb{R}\supsetneq\mathbb{Q},\quad\varnothing\neq\Omega,\quad\nexists x\in\partial\Omega,\quad\mathfrak{g}$",
        r"\begin{align*}\nabla\cdot\boldsymbol{B}&=0\\\nabla\times\boldsymbol{E}&=-\partial_t\boldsymbol{B}\end{align*}",
    ];
    let glyphs = latex.typeset(labels.map(String::from)).unwrap();
    assert_eq!(glyphs.len(), labels.len());
    for label in labels {
        let glyph = &glyphs[label];
        assert!(glyph.width_pt.is_finite() && glyph.width_pt > 0.);
        assert!(glyph.height_pt.is_finite() && glyph.height_pt > 0.);
        assert!(glyph.body.contains("<path"), "No outlined glyphs: {label}");
        assert!(!glyph.body.contains("<text"));
    }
    assert_eq!(latex.cached_label_count(), labels.len());
    let cached = latex.typeset(labels.map(String::from)).unwrap();
    for label in labels {
        assert_eq!(glyphs[label].body, cached[label].body);
    }
    assert_eq!(latex.cached_label_count(), labels.len());
}

#[test]
fn appended_macros_retain_ams_and_cache_keys_include_preamble_and_size() {
    let mut latex = fresh_latex("custom-preamble");
    latex
        .preamble
        .push_str(r"\newcommand{\wind}{\boldsymbol{u}}");
    let label = r"$\wind\in\mathbb{R}^{3}$".to_string();
    let original = latex.typeset([label.clone()]).unwrap();
    assert_eq!(latex.cached_label_count(), 1);
    latex.font_pt = 14.;
    let larger = latex.typeset([label.clone()]).unwrap();
    assert!(larger[&label].width_pt > original[&label].width_pt);
    assert_eq!(latex.cached_label_count(), 2);
    latex.preamble = latex.preamble.replace(
        r"\newcommand{\wind}{\boldsymbol{u}}",
        r"\newcommand{\wind}{\boldsymbol{U}_{\mathrm{north}}}",
    );
    let revised = latex.typeset([label.clone()]).unwrap();
    assert_ne!(revised[&label].body, larger[&label].body);
    assert_eq!(latex.cached_label_count(), 3);
}

#[cfg(feature = "export")]
#[test]
fn ams_labels_survive_native_export_and_web_sized_figures() {
    use rustplotlib::{Figure, IntendedUse, Label, WebLayout};
    let latex = fresh_latex("exports");
    let root = latex.cache.parent().unwrap().join(
        latex
            .cache
            .file_name()
            .unwrap()
            .to_string_lossy()
            .replace("exports", "export-files"),
    );
    std::fs::create_dir_all(&root).unwrap();
    let label = r"$\begin{aligned}\boldsymbol{u}&\in\mathbb{R}^{3}\\S(t)&=\begin{cases}S_0,&t\geq0,\\0,&t<0.\end{cases}\end{aligned}$";
    let mut figure = Figure::new(IntendedUse::ArticleColumn);
    figure.latex = latex.clone();
    figure.labels.push(Label::new(label, 40., 25.));
    for extension in ["svg", "pdf", "png"] {
        figure
            .save(root.join(format!("equation.{extension}")))
            .unwrap();
    }
    let svg = std::fs::read_to_string(root.join("equation.svg")).unwrap();
    assert!(svg.contains("<path"));
    assert!(!svg.contains("<text"));
    assert!(
        std::fs::read(root.join("equation.pdf"))
            .unwrap()
            .starts_with(b"%PDF")
    );
    let image = image::open(root.join("equation.png")).unwrap().to_rgb8();
    assert!(
        image
            .pixels()
            .filter(|p| p.0.iter().any(|c| *c < 100))
            .count()
            > 100
    );
    let mut web = WebLayout::new(640., 16.).unwrap().figure(200.).unwrap();
    web.latex.cache = latex.cache;
    web.labels.push(Label::new(label, 70., 25.));
    assert!(web.svg().unwrap().contains("<path"));
    assert!(
        web.html_embed("equation.svg", "Continuity equation")
            .is_ok()
    );
}
