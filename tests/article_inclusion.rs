#![cfg(feature = "export")]
use rustplotlib::*;
use std::{fs, process::Command};

/// Requires the same LaTeX/export toolchain as publication rendering.
#[test]
#[ignore = "requires pdflatex and dvisvgm; run explicitly for publication API changes"]
fn latex_checks_final_article_dimensions_and_body_font() -> anyhow::Result<()> {
    let dir = std::env::temp_dir().join(format!(
        "rustplotlib-article-contract-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir)?;
    let mut figure = ArticleLayout::new(164.6, 10.)?.figure(0.5, 65.)?;
    figure.latex.cache = dir.join("cache");
    let panels = figure.grid(1, 1)?;
    figure.panels[panels[0]].push(Layer::Line(Line::new(vec![[0., 0.], [1., 1.]], "blue")));
    figure.save_for_article(dir.join("figure.pdf"), "figure.pdf")?;
    let include = fs::read_to_string(dir.join("figure.include.tex"))?;
    for (name, width, font, expected) in [
        ("matching", 164.6, 10, None),
        ("wrong_width", 170., 10, Some("Article linewidth changed")),
        ("wrong_font", 164.6, 11, Some("Article body font changed")),
    ] {
        let tex = format!(
            "\\documentclass[{font}pt]{{article}}\n\\usepackage[textwidth={width}mm]{{geometry}}\n\\usepackage{{graphicx}}\n\\begin{{document}}\n{include}\n\\end{{document}}\n"
        );
        fs::write(dir.join(format!("{name}.tex")), tex)?;
        let output = Command::new("pdflatex")
            .current_dir(&dir)
            .args([
                "-interaction=nonstopmode",
                "-halt-on-error",
                &format!("{name}.tex"),
            ])
            .output()?;
        let log = String::from_utf8_lossy(&output.stdout);
        match expected {
            None => assert!(output.status.success(), "{log}"),
            Some(message) => {
                assert!(!output.status.success());
                assert!(log.contains(message), "{log}");
            }
        }
    }
    fs::remove_dir_all(dir)?;
    Ok(())
}
