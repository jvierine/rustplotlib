//! Lightweight scientific line/scatter panels for live browser diagnostics.
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
#[path = "../../src/palettes.rs"]
#[allow(dead_code)]
mod palettes;
#[wasm_bindgen]
pub struct SeriesView {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    width: f64,
    height: f64,
    bounds: [f64; 4],
    map: bool,
    time: bool,
    labels: Vec<[f64; 4]>,
    color_key: Vec<(String, String)>,
    key_positions: Vec<[f64; 2]>,
    top: f64,
    color_scale: Option<(f64, f64, String, String)>,
    text_scale: f64,
    full_bounds: [f64; 4],
}
#[wasm_bindgen]
impl SeriesView {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<SeriesView, JsValue> {
        let ctx = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("Canvas2D unavailable"))?
            .dyn_into::<CanvasRenderingContext2d>()?;
        Ok(Self {
            canvas,
            ctx,
            width: 600.,
            height: 230.,
            bounds: [0., 1., 0., 1.],
            map: false,
            time: false,
            labels: Vec::new(),
            color_key: Vec::new(),
            key_positions: Vec::new(),
            top: 20.,
            color_scale: None,
            text_scale: 1.,
            full_bounds: [0., 1., 0., 1.],
        })
    }
    /// Axis font size in CSS pixels. Call before begin/begin_map. Typography,
    /// label padding and hit regions scale together, independently of device DPR.
    pub fn set_font_size(&mut self, pixels: f64) -> Result<(), JsValue> {
        if !pixels.is_finite() || !(10. ..=32.).contains(&pixels) {
            return Err(JsValue::from_str("Font size must be 10–32 pixels"));
        }
        self.text_scale = pixels / 18.;
        Ok(())
    }
    /// CSS dimensions and device pixel ratio are separate, preserving font size.
    pub fn begin(
        &mut self,
        width: f64,
        height: f64,
        dpr: f64,
        x0: f64,
        x1: f64,
        y0: f64,
        y1: f64,
        left: &str,
        right: &str,
    ) -> Result<(), JsValue> {
        if ![width, height, dpr, x0, x1, y0, y1]
            .iter()
            .all(|x| x.is_finite())
            || width < 100.
            || height < 100.
            || dpr <= 0.
            || x1 <= x0
            || y1 <= y0
        {
            return Err(JsValue::from_str("Invalid plot dimensions or bounds"));
        }
        self.canvas.set_width((width * dpr).round() as u32);
        self.canvas.set_height((height * dpr).round() as u32);
        let width = width / self.text_scale;
        let height = height / self.text_scale;
        let dpr = dpr * self.text_scale;
        self.width = width;
        self.height = height;
        self.bounds = [x0, x1, y0, y1];
        self.full_bounds = self.bounds;
        self.map = false;
        self.time = false;
        self.labels.clear();
        let c = &self.ctx;
        c.set_transform(dpr, 0., 0., dpr, 0., 0.)?;
        c.set_font("15px RustplotlibPagella, Palatino, serif");
        let widths = self
            .color_key
            .iter()
            .map(|(_, label)| c.measure_text(label).map(|m| m.width() + 28.))
            .collect::<Result<Vec<_>, _>>()?;
        let (mut positions, mut top) = key_layout(&widths, width);
        if self.color_scale.is_some() {
            for p in &mut positions {
                p[1] += 58.;
            }
            top = if widths.is_empty() { 64. } else { top + 58. };
        }
        if widths.iter().any(|w| *w > width - 75.) || height - top - 35. < 30. {
            return Err(JsValue::from_str("Color key does not fit plot dimensions"));
        }
        self.key_positions = positions;
        self.top = top;
        self.draw_axes(left, right)
    }
    /// Categorical color key. Set before begin/begin_map; labels wrap above
    /// the data rectangle and share its font without covering observations.
    pub fn set_color_key(
        &mut self,
        colors: Vec<String>,
        labels: Vec<String>,
    ) -> Result<(), JsValue> {
        if colors.len() != labels.len() || colors.len() > 16 {
            return Err(JsValue::from_str(
                "Color key requires matching colors and labels",
            ));
        }
        self.color_key = colors.into_iter().zip(labels).collect();
        Ok(())
    }
    /// Continuous horizontal colorbar. Set before begin; series colors sampled
    /// with scale_color use exactly the same palette and clipping as the bar.
    pub fn set_color_scale(
        &mut self,
        low: f64,
        high: f64,
        palette: &str,
        label: &str,
    ) -> Result<(), JsValue> {
        if !valid_scale(low, high) || !matches!(palette, "viridis" | "plasma" | "turbo") {
            return Err(JsValue::from_str("Invalid color scale or palette"));
        }
        self.color_scale = Some((low, high, palette.into(), label.into()));
        Ok(())
    }
    pub fn set_scale(&mut self, low: f64, high: f64) -> Result<(), JsValue> {
        if !valid_scale(low, high) {
            return Err(JsValue::from_str("Invalid color limits"));
        }
        let Some(scale) = &mut self.color_scale else {
            return Err(JsValue::from_str("No color scale"));
        };
        scale.0 = low;
        scale.1 = high;
        Ok(())
    }
    pub fn scale_limits(&self) -> Vec<f64> {
        self.color_scale
            .as_ref()
            .map(|s| vec![s.0, s.1])
            .unwrap_or_default()
    }
    pub fn color_label(&self) -> String {
        self.color_scale
            .as_ref()
            .map(|s| s.3.clone())
            .unwrap_or_default()
    }
    pub fn colorbar_rect(&self) -> Vec<f64> {
        if self.color_scale.is_some() {
            [55., 0., self.width - 75., 56.]
                .map(|v| v * self.text_scale)
                .to_vec()
        } else {
            vec![0.; 4]
        }
    }
    pub fn scale_color(&self, value: f64) -> Result<String, JsValue> {
        let Some(s) = &self.color_scale else {
            return Err(JsValue::from_str("No color scale"));
        };
        if !value.is_finite() {
            return Err(JsValue::from_str("Nonfinite color value"));
        }
        Ok(scale_color(s.0, s.1, &s.2, value))
    }
    /// UTC x ticks for a time-series panel. Call before adding series.
    pub fn set_time_axis(&mut self) -> Result<(), JsValue> {
        self.time = true;
        self.ctx.clear_rect(0., 0., self.width, self.height);
        self.draw_axes("", "")
    }
    /// Mercator map with equal horizontal/vertical projected scale. Input
    /// series and labels remain longitude/latitude in degrees.
    pub fn begin_map(
        &mut self,
        width: f64,
        height: f64,
        dpr: f64,
        west: f64,
        east: f64,
        south: f64,
        north: f64,
    ) -> Result<(), JsValue> {
        if south < -85. || north > 85. {
            return Err(JsValue::from_str("Map latitude must be within 85 degrees"));
        }
        self.begin(width, height, dpr, west, east, south, north, "", "")?;
        self.map = true;
        let mut bounds = [west, east, mercator(south), mercator(north)];
        let ratio = (self.width - 75.) / (self.height - self.top - 35.);
        let dx = bounds[1] - bounds[0];
        let dy = bounds[3] - bounds[2];
        if dx / dy < ratio {
            let pad = (dy * ratio - dx) / 2.;
            bounds[0] -= pad;
            bounds[1] += pad;
        } else {
            let pad = (dx / ratio - dy) / 2.;
            bounds[2] -= pad;
            bounds[3] += pad;
        }
        self.bounds = bounds;
        self.full_bounds = bounds;
        self.ctx.clear_rect(0., 0., self.width, self.height);
        self.draw_axes("Longitude (°E)", "")
    }
    /// CSS data rectangle for navigation hit testing at any font size or DPR.
    pub fn plot_rect(&self) -> Vec<f64> {
        [
            55.,
            self.top,
            self.width - 75.,
            self.height - self.top - 35.,
        ]
        .map(|v| v * self.text_scale)
        .to_vec()
    }
    /// Normalized viewport of the original data bounds (projected for maps).
    /// Call after begin and before adding series. Zoom/pan never changes data.
    pub fn set_view(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) -> Result<(), JsValue> {
        if ![x0, y0, x1, y1].iter().all(|v| v.is_finite()) || x1 <= x0 || y1 <= y0 {
            return Err(JsValue::from_str("Invalid series viewport"));
        }
        let b = self.full_bounds;
        self.bounds = [
            b[0] + (b[1] - b[0]) * x0,
            b[0] + (b[1] - b[0]) * x1,
            b[2] + (b[3] - b[2]) * y0,
            b[2] + (b[3] - b[2]) * y1,
        ];
        self.labels.clear();
        self.ctx.clear_rect(0., 0., self.width, self.height);
        self.draw_axes("", "")
    }
    /// Paired x,y samples; nonfinite samples break lines. Markers stay in CSS px.
    pub fn series(&self, xy: &[f64], color: &str, line: bool, radius: f64) -> Result<(), JsValue> {
        if xy.len() % 2 != 0 || !radius.is_finite() || radius < 0. {
            return Err(JsValue::from_str("Invalid series"));
        }
        let c = &self.ctx;
        c.save();
        c.begin_path();
        c.rect(
            55.,
            self.top,
            self.width - 75.,
            self.height - self.top - 35.,
        );
        c.clip();
        c.set_stroke_style_str(color);
        c.set_fill_style_str(color);
        c.set_line_width(1.7);
        if line {
            c.begin_path();
            let mut first = true;
            for p in xy.chunks_exact(2) {
                if !p.iter().all(|v| v.is_finite()) {
                    first = true;
                    continue;
                }
                let (x, y) = (self.px(p[0]), self.py(p[1]));
                if first {
                    c.move_to(x, y);
                    first = false;
                } else {
                    c.line_to(x, y);
                }
            }
            c.stroke();
        }
        if radius > 0. {
            for p in xy.chunks_exact(2) {
                if p.iter().all(|v| v.is_finite()) {
                    c.begin_path();
                    c.arc(
                        self.px(p[0]),
                        self.py(p[1]),
                        radius,
                        0.,
                        std::f64::consts::TAU,
                    )?;
                    c.fill();
                }
            }
        }
        c.restore();
        Ok(())
    }
    /// Annotate data coordinates using the same font and transform as the axes.
    pub fn label(&mut self, x: f64, y: f64, text: &str, color: &str) -> Result<(), JsValue> {
        if !x.is_finite() || !y.is_finite() {
            return Err(JsValue::from_str("Invalid label coordinate"));
        }
        let c = &self.ctx;
        c.save();
        c.set_font("16px RustplotlibPagella, Palatino, serif");
        c.set_fill_style_str(color);
        c.set_text_align("left");
        let anchor = [self.px(x), self.py(y)];
        if anchor[0] < 55.
            || anchor[0] > self.width - 20.
            || anchor[1] < self.top
            || anchor[1] > self.height - 35.
        {
            c.restore();
            return Ok(());
        }
        let width = c.measure_text(text)?.width();
        let mut chosen = [0.; 4];
        let mut best = f64::INFINITY;
        for offset in [
            [6., -22.],
            [6., 8.],
            [-width - 8., -22.],
            [-width - 8., 8.],
            [6., -48.],
            [6., 34.],
            [-width - 8., -48.],
            [-width - 8., 34.],
        ] {
            let px = (anchor[0] + offset[0]).clamp(57., (self.width - width - 22.).max(57.));
            let py = (anchor[1] + offset[1]).clamp(self.top + 1., self.height - 57.);
            let rect = [px, py, width + 4., 22.];
            let score = self.labels.iter().map(|b| overlap(rect, *b)).sum::<f64>();
            if score < best {
                best = score;
                chosen = rect;
            }
            if score == 0. {
                break;
            }
        }
        self.labels.push(chosen);
        let px = chosen[0] + 2.;
        let py = chosen[1] + 16.;
        if (px - anchor[0]).hypot(py - anchor[1]) > 30. {
            c.set_stroke_style_str(color);
            c.set_line_width(0.7);
            c.begin_path();
            c.move_to(anchor[0], anchor[1]);
            c.line_to(px, py - 6.);
            c.stroke();
        }
        c.set_fill_style_str("rgba(255,255,255,0.9)");
        c.fill_rect(px - 2., py - 15., width + 4., 20.);
        c.set_fill_style_str(color);
        c.fill_text(text, px, py)?;
        c.restore();
        Ok(())
    }
}
impl SeriesView {
    fn draw_axes(&self, left: &str, right: &str) -> Result<(), JsValue> {
        let c = &self.ctx;
        c.set_font("15px RustplotlibPagella, Palatino, serif");
        c.set_text_align("left");
        if let Some((low, high, palette, label)) = &self.color_scale {
            c.set_fill_style_str("#314858");
            c.fill_text(label, 55., 15.)?;
            let width = self.width - 75.;
            for i in 0..256 {
                let value = low + (high - low) * i as f64 / 255.;
                c.set_fill_style_str(&scale_color(*low, *high, palette, value));
                c.fill_rect(55. + width * i as f64 / 256., 23., width / 256. + 0.5, 10.);
            }
            c.set_fill_style_str("#314858");
            let n = if width < 400. { 2 } else { 5 };
            for i in 0..=n {
                c.set_text_align(if i == 0 {
                    "left"
                } else if i == n {
                    "right"
                } else {
                    "center"
                });
                c.fill_text(
                    &format!("{:.1}", low + (high - low) * i as f64 / n as f64),
                    55. + width * i as f64 / n as f64,
                    51.,
                )?;
            }
            c.set_text_align("left");
        }
        for ((color, label), position) in self.color_key.iter().zip(&self.key_positions) {
            c.set_fill_style_str(color);
            c.fill_rect(position[0], position[1] + 3., 18., 8.);
            c.set_fill_style_str("#314858");
            c.fill_text(label, position[0] + 24., position[1] + 13.)?;
        }
        c.set_font("18px RustplotlibPagella, Palatino, serif");
        c.set_fill_style_str("#314858");
        c.set_stroke_style_str("#e2e7ea");
        c.set_line_width(1.);
        c.set_text_align("right");
        for j in 0..5 {
            let v = self.bounds[2] + (self.bounds[3] - self.bounds[2]) * j as f64 / 4.;
            let py = self.height - 35. - (self.height - self.top - 35.) * j as f64 / 4.;
            c.begin_path();
            c.move_to(55., py);
            c.line_to(self.width - 20., py);
            c.stroke();
            let value = if self.map { inverse_mercator(v) } else { v };
            c.fill_text(&format!("{value:.1}"), 48., py + 5.)?;
        }
        if self.map || self.time {
            let count = if self.width < 550. { 2 } else { 4 };
            for j in 0..=count {
                let x =
                    self.bounds[0] + (self.bounds[1] - self.bounds[0]) * j as f64 / count as f64;
                let px = self.px(x);
                c.begin_path();
                c.move_to(px, self.top);
                c.line_to(px, self.height - 35.);
                c.stroke();
                c.set_text_align(if j == 0 {
                    "left"
                } else if j == count {
                    "right"
                } else {
                    "center"
                });
                let label = if self.time {
                    if j == count && x.rem_euclid(86400.) < 1. {
                        "24:00 UTC".into()
                    } else {
                        format!(
                            "{:02}:{:02}",
                            (x / 3600.).floor().rem_euclid(24.) as u32,
                            (x / 60.).floor().rem_euclid(60.) as u32
                        )
                    }
                } else {
                    format!("{x:.0}°")
                };
                c.fill_text(&label, px, self.height - 12.)?;
            }
        } else {
            c.set_text_align("left");
            c.fill_text(left, 55., self.height - 8.)?;
            c.set_text_align("right");
            c.fill_text(right, self.width - 20., self.height - 8.)?;
        }
        c.set_text_align("left");
        Ok(())
    }
    fn px(&self, x: f64) -> f64 {
        55. + (x - self.bounds[0]) / (self.bounds[1] - self.bounds[0]) * (self.width - 75.)
    }
    fn py(&self, y: f64) -> f64 {
        let y = if self.map {
            mercator(y.clamp(-85., 85.))
        } else {
            y
        };
        self.height
            - 35.
            - (y - self.bounds[2]) / (self.bounds[3] - self.bounds[2])
                * (self.height - self.top - 35.)
    }
}
fn valid_scale(low: f64, high: f64) -> bool {
    low.is_finite() && high.is_finite() && high > low && (high - low).is_finite()
}
fn scale_color(low: f64, high: f64, palette: &str, value: f64) -> String {
    let index = (((value - low) / (high - low)).clamp(0., 1.) * 255.).round() as usize;
    let rgb = if palette == "turbo" {
        palettes::turbo((value - low) / (high - low))
    } else if palette == "plasma" {
        palettes::PLASMA[index]
    } else {
        palettes::VIRIDIS[index]
    };
    format!(
        "rgb({},{},{})",
        (rgb[0] * 255.).round() as u8,
        (rgb[1] * 255.).round() as u8,
        (rgb[2] * 255.).round() as u8
    )
}
fn key_layout(widths: &[f64], width: f64) -> (Vec<[f64; 2]>, f64) {
    if widths.is_empty() {
        return (Vec::new(), 20.);
    }
    let mut x = 55.;
    let mut y = 2.;
    let mut positions = Vec::new();
    for &item in widths {
        if x > 55. && x + item > width - 20. {
            x = 55.;
            y += 22.;
        }
        positions.push([x, y]);
        x += item + 16.;
    }
    (positions, y + 36.)
}
fn mercator(lat: f64) -> f64 {
    (std::f64::consts::FRAC_PI_4 + lat.to_radians() / 2.)
        .tan()
        .ln()
        .to_degrees()
}
fn inverse_mercator(y: f64) -> f64 {
    (2. * y.to_radians().exp().atan() - std::f64::consts::FRAC_PI_2).to_degrees()
}
fn overlap(a: [f64; 4], b: [f64; 4]) -> f64 {
    ((a[0] + a[2]).min(b[0] + b[2]) - a[0].max(b[0])).max(0.)
        * ((a[1] + a[3]).min(b[1] + b[3]) - a[1].max(b[1])).max(0.)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn continuous_scale_clips_and_rejects_invalid_limits() {
        assert_eq!(
            scale_color(0., 25., "viridis", -3.),
            scale_color(0., 25., "viridis", 0.)
        );
        assert_eq!(
            scale_color(0., 25., "viridis", 30.),
            scale_color(0., 25., "viridis", 25.)
        );
        assert_ne!(
            scale_color(0., 25., "viridis", 10.),
            scale_color(0., 25., "viridis", 15.)
        );
        assert!(!valid_scale(3., 3.));
        assert!(!valid_scale(f64::NAN, 25.));
        assert!(!valid_scale(-f64::MAX, f64::MAX));
    }
    #[test]
    fn categorical_key_wraps_without_covering_data() {
        let (wide, top) = key_layout(&[100., 100., 100., 100.], 600.);
        assert!(wide.iter().all(|p| p[1] == 2.));
        assert_eq!(top, 38.);
        let (narrow, top) = key_layout(&[100., 100., 100., 100.], 300.);
        assert_eq!(narrow[2], [55., 24.]);
        assert_eq!(top, 60.);
        assert!(narrow.iter().all(|p| p[1] + 16. < top));
        assert_eq!(key_layout(&[], 300.).1, 20.);
    }
    #[test]
    fn map_projection_preserves_latitudes_and_local_scale() {
        for lat in [-80., -45., 0., 54., 69.7, 80.] {
            assert!((inverse_mercator(mercator(lat)) - lat).abs() < 1e-10);
        }
        let dy = (mercator(69.7001) - mercator(69.7)) / 0.0001;
        assert!((dy * 69.7_f64.to_radians().cos() - 1.).abs() < 1e-5);
    }
}
