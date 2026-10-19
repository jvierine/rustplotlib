//! AMS mathematics export regression. Generate with
//! `cargo run --example ams_math`; outputs live under output/pdf/ by default.
//! Script provenance stays in source comments, never in the rendered figure.
use anyhow::Result;
use rustplotlib::{Figure, IntendedUse, Label};
use std::{fs, path::PathBuf};

fn main() -> Result<()> {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("output/pdf"));
    fs::create_dir_all(&root)?;
    let mut figure = Figure::new(IntendedUse::ArticleWide);
    figure.height_mm = 99.;
    figure.latex.cache = PathBuf::from("target/ams-math-example-cache");
    for (text, x, y) in [
        (r"\textbf{Continuity and momentum}", 45., 9.),
        (r"\textbf{Vector and matrix notation}", 133., 9.),
        (
            r"$\begin{aligned}\partial_t n+\nabla\cdot(n\boldsymbol{u})&=S-L\\\rho D_t\boldsymbol{u}&=-\nabla p+\boldsymbol{f}\end{aligned}$",
            45.,
            28.,
        ),
        (
            r"$\boldsymbol{u}=\begin{pmatrix}u\\v\\w\end{pmatrix}\in\mathbb{R}^{3},\quad\mathbf{I}=\begin{bmatrix}1&0\\0&1\end{bmatrix}$",
            133.,
            28.,
        ),
        (r"\textbf{Flux balance}", 45., 45.),
        (r"\textbf{Piecewise source}", 133., 45.),
        (
            r"$\displaystyle\int_{\Omega}\nabla\cdot\boldsymbol{u}\,\mathrm{d}V=\oint_{\partial\Omega}\boldsymbol{u}\cdot\hat{\boldsymbol{n}}\,\mathrm{d}A$",
            45.,
            58.,
        ),
        (
            r"$S(t)=\begin{cases}S_0 e^{-t/\tau},&t\geq0,\\0,&t<0.\end{cases}$",
            133.,
            58.,
        ),
        (r"\textbf{Admissible states}", 45., 76.),
        (r"\textbf{Sets and operators}", 133., 76.),
        (
            r"$\lVert\boldsymbol{u}\rVert_2\leq U_{\max},\quad\operatorname{Re}\lambda\leq0$",
            45.,
            88.,
        ),
        (
            r"$\varnothing\neq\Omega\subsetneq\mathbb{R}^{3},\quad\mathfrak{g},\quad\nexists x\in\partial\Omega$",
            133.,
            88.,
        ),
    ] {
        figure.labels.push(Label::new(text, x, y));
    }
    for extension in ["svg", "pdf", "png"] {
        figure.save(root.join(format!("ams-math.{extension}")))?;
    }
    Ok(())
}
