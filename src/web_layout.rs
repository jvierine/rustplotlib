//! Web exports sized relative to surrounding CSS text, without responsive
//! image rules silently shrinking all plot labels.
use crate::{Figure, IntendedUse};
use anyhow::{ensure, Result};

#[derive(Clone, Copy, Debug)]
pub struct WebLayout {
    width_css_px: f64,
    body_font_css_px: f64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct WebContract {
    layout: WebLayout,
}

impl WebLayout {
    /// Intended container width and surrounding text size in CSS pixels
    /// (not device pixels). Device pixel ratio does not affect layout.
    pub fn new(width_css_px: f64, body_font_css_px: f64) -> Result<Self> {
        ensure!(
            [width_css_px, body_font_css_px]
                .iter()
                .all(|v| v.is_finite() && *v > 0.),
            "Invalid web layout dimensions"
        );
        Ok(Self {
            width_css_px,
            body_font_css_px,
        })
    }

    /// Design a layout at its intended width; labels match the CSS body size.
    /// Construct another layout for a narrower responsive variant, not a scaled
    /// copy. Panel and label placement must be recalculated for that variant.
    pub fn figure(self, height_css_px: f64) -> Result<Figure> {
        ensure!(
            height_css_px.is_finite() && height_css_px > 0.,
            "Invalid web figure height"
        );
        let mut f = Figure::new(IntendedUse::Custom {
            width_mm: self.width_css_px * 25.4 / 96.,
            font_pt: self.body_font_css_px * 72.27 / 96.,
        });
        f.height_mm = height_css_px * 25.4 / 96.;
        f.tight_export = false;
        f.web_contract = Some(WebContract { layout: self });
        Ok(f)
    }
}

impl WebContract {
    pub(crate) fn validate(self, f: &Figure) -> Result<()> {
        ensure!(
            (f.width_mm - self.layout.width_css_px * 25.4 / 96.).abs() < 1e-8,
            "Web width changed: construct a new WebLayout for this container"
        );
        let font = self.layout.body_font_css_px * 72.27 / 96.;
        ensure!(
            (f.font_pt - font).abs() < 1e-8 && (f.latex.font_pt - font).abs() < 1e-8,
            "Web typography changed: use the surrounding CSS font size in WebLayout"
        );
        ensure!(
            !f.tight_export,
            "Tight cropping breaks the web text-size contract"
        );
        Ok(())
    }
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

impl Figure {
    /// An accessible SVG image in a keyboard-scrollable container. Its width
    /// is in em, so plot typography follows the surrounding CSS font (including
    /// browser text-size changes). Narrow containers scroll instead of shrinking
    /// labels. Do not override the image width with max-width:100% or transforms.
    pub fn html_embed(&self, svg_url: &str, alt: &str) -> Result<String> {
        let c = self.web_contract.ok_or_else(|| {
            anyhow::anyhow!("Use WebLayout::figure before generating a web embed")
        })?;
        c.validate(self)?;
        ensure!(
            !svg_url.trim().is_empty() && !alt.trim().is_empty(),
            "Web embeds require a URL and accessible description"
        );
        let width_em = c.layout.width_css_px / c.layout.body_font_css_px;
        Ok(format!("<div class=\"rustplotlib-viewport\" role=\"region\" aria-label=\"{alt}\" tabindex=\"0\" style=\"overflow-x:auto;max-width:100%\">\n  <img src=\"{url}\" alt=\"{alt}\" style=\"display:block;font-size:inherit;width:{width_em:.8}em;max-width:none;height:auto\">\n</div>\n", alt=escape(alt), url=escape(svg_url)))
    }

    /// Export SVG plus a `.embed.html` companion, without network dependencies.
    #[cfg(feature = "export")]
    pub fn save_for_web(
        &self,
        path: impl AsRef<std::path::Path>,
        svg_url: &str,
        alt: &str,
    ) -> Result<()> {
        let path = path.as_ref();
        ensure!(
            path.extension().and_then(|s| s.to_str()) == Some("svg"),
            "Web typography export requires SVG"
        );
        let embed = self.html_embed(svg_url, alt)?;
        self.save(path)?;
        std::fs::write(path.with_extension("embed.html"), embed)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn css_size_and_relative_embed() {
        let f = WebLayout::new(640., 16.).unwrap().figure(400.).unwrap();
        assert!((f.font_pt - 12.045).abs() < 1e-12);
        assert!(f
            .html_embed("plot.svg?a=1&b=2", "A < B")
            .unwrap()
            .contains("width:40.00000000em"));
        assert!(f
            .html_embed("plot.svg?a=1&b=2", "A < B")
            .unwrap()
            .contains("A &lt; B"));
        let mobile = WebLayout::new(320., 16.).unwrap().figure(400.).unwrap();
        assert_eq!(f.font_pt, mobile.font_pt);
        assert!(mobile.html_embed("plot.svg", "").is_err());
        assert!(WebLayout::new(0., 16.).is_err());
    }
    #[test]
    fn rejects_unbound_and_scaled_output() {
        assert!(Figure::new(IntendedUse::Mobile)
            .html_embed("plot.svg", "Plot")
            .is_err());
        let mut f = WebLayout::new(640., 16.).unwrap().figure(400.).unwrap();
        f.width_mm *= 0.5;
        assert!(f.html_embed("plot.svg", "Plot").is_err());
        assert!(f.svg().is_err());
    }
}
