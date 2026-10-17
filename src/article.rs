//! Figure typography specified at its final size in the article, not on an
//! arbitrary canvas that will later be scaled by LaTeX.
use crate::{Figure, IntendedUse};
use anyhow::{ensure, Result};

/// Measure `\linewidth` in the intended figure environment (column or full
/// width), and use the manuscript's normal body font size in TeX points.
#[derive(Clone, Copy, Debug)]
pub struct ArticleLayout {
    linewidth_mm: f64,
    body_font_pt: f64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ArticleContract {
    layout: ArticleLayout,
    width_mm: f64,
}

impl ArticleLayout {
    /// Check typography after the actual LaTeX inclusion scaling. This also
    /// audits legacy figures that were not created with an article contract.
    pub fn check_inclusion(self, figure: &Figure, included_width_mm: f64) -> Result<()> {
        ensure!(!figure.tight_export,
            "Cannot certify article font size with tight cropping enabled");
        ensure!(included_width_mm.is_finite() && included_width_mm > 0.,
            "Invalid article inclusion width");
        let scale=included_width_mm/figure.width_mm;
        for font in [figure.font_pt,figure.latex.font_pt] {
            let printed=font*scale;
            ensure!((printed-self.body_font_pt).abs()<0.01,
                "Printed figure font is {printed:.3} pt, not article body size {:.3} pt; regenerate at the final inclusion width",
                self.body_font_pt);
        }
        let explicit = regex::Regex::new(r"\\fontsize\s*\{\s*([0-9]+(?:\.[0-9]+)?)\s*\}")?;
        for label in figure.all_labels() {
            for size in explicit.captures_iter(&label) {
                let printed=size[1].parse::<f64>()?*scale;
                ensure!(printed+0.01>=self.body_font_pt,
                    "Explicit annotation font prints at {printed:.3} pt, below article body size {:.3} pt",
                    self.body_font_pt);
            }
        }
        Ok(())
    }

    pub fn new(linewidth_mm: f64, body_font_pt: f64) -> Result<Self> {
        ensure!(
            linewidth_mm.is_finite() && linewidth_mm > 0.,
            "Invalid article linewidth"
        );
        ensure!(
            body_font_pt.is_finite() && body_font_pt > 0.,
            "Invalid article body font size"
        );
        Ok(Self {
            linewidth_mm,
            body_font_pt,
        })
    }

    /// Design at final physical size. A narrower fraction changes the canvas,
    /// never the font size. Height is also the final height in millimetres.
    pub fn figure(self, width_fraction: f64, height_mm: f64) -> Result<Figure> {
        ensure!(
            width_fraction.is_finite() && width_fraction > 0. && width_fraction <= 1.,
            "Article width fraction must be in (0, 1]"
        );
        ensure!(
            height_mm.is_finite() && height_mm > 0.,
            "Invalid article figure height"
        );
        let width_mm = self.linewidth_mm * width_fraction;
        let mut figure = Figure::new(IntendedUse::Custom {
            width_mm,
            font_pt: self.body_font_pt,
        });
        figure.height_mm = height_mm;
        figure.tight_export = false;
        figure.article_contract = Some(ArticleContract {
            layout: self,
            width_mm,
        });
        Ok(figure)
    }
}

impl ArticleContract {
    pub(crate) fn validate(self, f: &Figure) -> Result<()> {
        ensure!((f.width_mm - self.width_mm).abs() < 1e-8,
            "Article figure width changed: construct a new ArticleLayout figure at the desired final width");
        ensure!(
            (f.font_pt - self.layout.body_font_pt).abs() < 1e-8
                && (f.latex.font_pt - self.layout.body_font_pt).abs() < 1e-8,
            "Article typography changed: labels must match the configured body font size"
        );
        ensure!(!f.tight_export,
            "Tight cropping changes the article inclusion scale; keep the fixed article canvas and adjust panel layout instead");
        self.layout.check_inclusion(f,self.width_mm)?;
        Ok(())
    }
}

impl Figure {
    /// Default publication export: tight ink bounds, no typography rescaling.
    #[cfg(feature = "export")]
    pub fn save_for_article(
        &self,
        path: impl AsRef<std::path::Path>,
        pdf_path_in_article: &str,
    ) -> Result<()> {
        self.save_trimmed_for_article(path, pdf_path_in_article)
    }

    /// Trim outer whitespace while preserving physical typography. The companion
    /// includes the cropped width in millimetres, never scaled to fill linewidth.
    #[cfg(feature = "export")]
    pub fn save_trimmed_for_article(
        &self,
        path: impl AsRef<std::path::Path>,
        pdf_path_in_article: &str,
    ) -> Result<()> {
        let path = path.as_ref();
        ensure!(path.extension().and_then(|s| s.to_str()) == Some("pdf"),
            "Article export requires PDF");
        let mut inclusion = self.latex_include(pdf_path_in_article)?;
        let mut export = self.clone();
        export.article_contract = None;
        export.tight_export = true;
        let svg = export.tight_svg(&self.svg()?)?;
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default())?;
        let width_mm = tree.size().width() as f64 * 25.4 / 96.;
        inclusion = inclusion.replace(
            &format!("width={:.6}mm", self.width_mm),
            &format!("width={width_mm:.6}mm"));
        inclusion = inclusion.replace(
            &format!("final width {:.6} mm", self.width_mm),
            &format!("final width {width_mm:.6} mm"));
        for extension in ["pdf", "png", "svg"] {
            export.save(path.with_extension(extension))?;
        }
        std::fs::write(path.with_extension("include.tex"), inclusion)?;
        Ok(())
    }

    /// Checked LaTeX inclusion at the final physical size. Fails at TeX compile
    /// time if the surrounding article width or normal font size has changed.
    /// Do not wrap the result in scalebox/resizebox or another scaled container.
    pub fn latex_include(&self, pdf_path: &str) -> Result<String> {
        let c = self.article_contract.ok_or_else(|| {
            anyhow::anyhow!("Use ArticleLayout::figure before generating article inclusion")
        })?;
        c.validate(self)?;
        ensure!(
            !pdf_path.is_empty()
                && !pdf_path
                    .chars()
                    .any(|x| matches!(x, '{' | '}' | '%' | '\n' | '\r' | '\\' | '#')),
            "Unsupported LaTeX figure path"
        );
        Ok(format!(
            r"% rustplotlib: final width {width:.6} mm; body/label font {font:.6} TeX pt.
% Include directly, without resizebox/scalebox or outer width scaling.
\begingroup
\normalsize
\dimen0=\linewidth
\advance\dimen0 by -{line:.6}mm
\ifdim\dimen0<0pt \dimen0=-\dimen0\fi
\ifdim\dimen0>0.1mm
  \PackageError{{rustplotlib}}{{Article linewidth changed}}{{Regenerate this figure using the new ArticleLayout.}}
\fi
\dimen0=\csname f@size\endcsname pt
\advance\dimen0 by -{font:.6}pt
\ifdim\dimen0<0pt \dimen0=-\dimen0\fi
\ifdim\dimen0>0.01pt
  \PackageError{{rustplotlib}}{{Article body font changed}}{{Regenerate this figure using the new ArticleLayout.}}
\fi
\includegraphics[width={width:.6}mm]{{\detokenize{{{pdf_path}}}}}
\endgroup
",
            width = c.width_mm,
            line = c.layout.linewidth_mm,
            font = c.layout.body_font_pt
        ))
    }

    /// Explicitly retain the complete design canvas, including blank margins.
    /// Use for fixed-size frames or when whitespace is intentionally requested.
    #[cfg(feature = "export")]
    pub fn save_fixed_for_article(
        &self,
        path: impl AsRef<std::path::Path>,
        pdf_path_in_article: &str,
    ) -> Result<()> {
        let path = path.as_ref();
        ensure!(
            path.extension().and_then(|s| s.to_str()) == Some("pdf"),
            "Article export requires PDF"
        );
        let inclusion = self.latex_include(pdf_path_in_article)?;
        self.save(path)?;
        std::fs::write(path.with_extension("include.tex"), inclusion)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "export")]
    #[test]
    fn trimmed_article_export_never_rescales_typography() {
        let mut f = ArticleLayout::new(184., 10.).unwrap().figure(1., 100.).unwrap();
        let mut panel = crate::Panel::new(
            crate::Rect::new(30., 20., 100., 50.),
            crate::Axes::new([0., 1.], [0., 1.], "", ""));
        panel.axes_visible = false;
        panel.push(crate::Layer::Line(crate::Line::new(vec![[0., 0.], [1., 1.]], "black")));
        f.panels.push(panel);
        let dir = std::env::temp_dir().join(format!("rustplotlib-article-crop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        f.save_for_article(dir.join("figure.pdf"), "figure.pdf").unwrap();
        let svg = std::fs::read_to_string(dir.join("figure.svg")).unwrap();
        let include = std::fs::read_to_string(dir.join("figure.include.tex")).unwrap();
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).unwrap();
        let width = tree.size().width() as f64 * 25.4 / 96.;
        assert!(width > 100. && width < 104.);
        assert!(include.contains(&format!("width={width:.6}mm")));
        assert!(include.contains("body/label font 10.000000 TeX pt"));
        assert_eq!(f.font_pt, 10.);
        assert_eq!(f.width_mm, 184.);
        assert!(!f.tight_export);
        f.save_fixed_for_article(dir.join("fixed.pdf"), "fixed.pdf").unwrap();
        assert!(std::fs::read_to_string(dir.join("fixed.include.tex")).unwrap()
            .contains("width=184.000000mm"));
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn narrower_canvas_keeps_body_sized_text() {
        let article = ArticleLayout::new(164.6, 10.).unwrap();
        let a = article.figure(1., 100.).unwrap();
        let b = article.figure(0.5, 100.).unwrap();
        assert_eq!(a.font_pt, b.font_pt);
        assert_eq!(b.width_mm, 82.3);
        assert!(!b.tight_export);
        assert!(b
            .latex_include("figures/map.pdf")
            .unwrap()
            .contains("width=82.300000mm"));
        assert!(article.figure(1.1, 10.).is_err());
        assert!(ArticleLayout::new(f64::NAN, 10.).is_err());
    }
    #[test]
    fn rejects_silent_scale_and_font_changes() {
        let article = ArticleLayout::new(164.6, 10.).unwrap();
        for mutation in 0..4 {
            let mut f = article.figure(1., 100.).unwrap();
            match mutation {
                0 => f.width_mm *= 0.8,
                1 => f.font_pt = 8.,
                2 => f.tight_export = true,
                _ => f.latex.font_pt = 8.,
            }
            assert!(f.latex_include("map.pdf").is_err());
            assert!(f.svg().is_err());
        }
        let f = article.figure(1., 100.).unwrap();
        assert!(f.latex_include("bad}path.pdf").is_err());
    }

    #[test]
    fn radiant_figure_regression_checks_printed_not_requested_font() {
        let article=ArticleLayout::new(184.,10.).unwrap();
        let mut old=Figure::new(IntendedUse::Custom{width_mm:196.,font_pt:9.});
        old.tight_export=false;
        // The old recipe shrank a 196 mm, 9 pt figure to 86% of the article width.
        let error=article.check_inclusion(&old,0.86*184.).unwrap_err().to_string();
        assert!(error.contains("7.266"),"{error}");
        let corrected=article.figure(1.,106.).unwrap();
        article.check_inclusion(&corrected,184.).unwrap();
        assert!(article.check_inclusion(&corrected,0.86*184.).is_err());
        let mut undersized=article.figure(1.,106.).unwrap();
        undersized.labels.push(crate::Label::new(r"{\fontsize{7}{8}\selectfont 3 h}",30.,30.));
        assert!(article.check_inclusion(&undersized,184.).is_err());
        assert!(undersized.svg().is_err());
    }
}
