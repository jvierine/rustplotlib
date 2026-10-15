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
        Ok(())
    }
}

impl Figure {
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

    /// Export the PDF and a `.include.tex` companion containing the checked
    /// inclusion. `pdf_path_in_article` is relative to the manuscript root.
    #[cfg(feature = "export")]
    pub fn save_for_article(
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
}
