//! Lightweight scientific line/scatter panels for live browser diagnostics.
use serde::Deserialize;
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

#[derive(Deserialize)]
struct PlotSeries {
    #[serde(default)]
    name: String,
    color: String,
    points: Vec<[Option<f64>; 2]>,
    #[serde(default = "line_default")]
    line: bool,
    #[serde(default)]
    radius: f64,
    #[serde(default)]
    errors: Vec<Option<f64>>,
    #[serde(default = "line_width_default")]
    line_width: f64,
    #[serde(default)]
    fill: Option<String>,
}
fn line_default() -> bool {
    true
}
fn line_width_default() -> f64 {
    1.7
}
#[derive(Deserialize)]
struct SeriesSpec {
    bounds: [f64; 4],
    xlabel: String,
    ylabel: String,
    #[serde(default)]
    utc_origin_seconds: Option<f64>,
    series: Vec<PlotSeries>,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    bars: bool,
    #[serde(default)]
    sky: Option<SkyDensity>,
    #[serde(default)]
    field: Option<FieldData>,
    #[serde(default)]
    equal_aspect: bool,
    /// Dense small multiples: native, reduced margins and typography.
    #[serde(default)]
    compact_axes: bool,
    #[serde(default)]
    hide_x_labels: bool,
    #[serde(default)]
    panel_label: String,
}
include!("diagnostic_fields.rs");
fn nice_ticks(lo: f64, hi: f64, target: usize) -> Vec<f64> {
    let raw = (hi - lo) / (target.max(3) - 1) as f64;
    let magnitude = 10_f64.powf(raw.log10().floor());
    let mut step = [1., 2., 2.5, 5., 10.]
        .into_iter()
        .find(|s| *s * magnitude >= raw)
        .unwrap_or(10.)
        * magnitude;
    while (hi / step).floor() - (lo / step).ceil() + 1. < 3. {
        step /= 2.;
    }
    let first = (lo / step).ceil() * step;
    (0..32)
        .map(|i| first + i as f64 * step)
        .take_while(|v| *v <= hi + step * 1e-8)
        .collect()
}
fn number_label(v: f64, span: f64) -> String {
    if v == 0. {
        return "0".into();
    }
    if v.abs() >= 1e6 || span < 0.001 {
        return format!("{v:.1e}");
    }
    let decimals = if span >= 20. {
        0
    } else if span >= 2. {
        1
    } else if span >= 0.2 {
        2
    } else {
        3
    };
    let text = format!("{v:.decimals$}");
    if decimals > 0 {
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        text
    }
}
#[wasm_bindgen]
pub struct SeriesView {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    width: f64,
    height: f64,
    bounds: [f64; 4],
    rect: [f64; 4],
    line_width: f64,
    color_limits: [f64; 2],
}
#[wasm_bindgen]
impl SeriesView {
    pub fn color_limits(&self) -> Vec<f64> {
        self.color_limits.to_vec()
    }
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
            rect: [55., 20., 525., 175.],
            line_width: 1.7,
            color_limits: [0., 1.],
        })
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
        self.width = width;
        self.height = height;
        self.bounds = [x0, x1, y0, y1];
        self.rect = [55., 20., width - 75., height - 55.];
        self.canvas.set_width((width * dpr).round() as u32);
        self.canvas.set_height((height * dpr).round() as u32);
        let c = &self.ctx;
        c.set_transform(dpr, 0., 0., dpr, 0., 0.)?;
        c.set_font("18px RustplotlibPagella, Palatino, serif");
        c.set_fill_style_str("#314858");
        c.set_stroke_style_str("#d6dfe4");
        c.set_line_width(1.);
        for j in 0..5 {
            let v = y0 + (y1 - y0) * j as f64 / 4.;
            let py = self.py(v);
            c.begin_path();
            c.move_to(55., py);
            c.line_to(width - 20., py);
            c.stroke();
            c.fill_text(&format!("{v:.1}"), 2., py + 4.)?;
        }
        c.fill_text(left, 55., height - 8.)?;
        c.set_text_align("right");
        c.fill_text(right, width - 20., height - 8.)?;
        c.set_text_align("left");
        Ok(())
    }
    /// Paired x,y samples; nonfinite samples break lines. Markers stay in CSS px.
    pub fn series(&self, xy: &[f64], color: &str, line: bool, radius: f64) -> Result<(), JsValue> {
        if xy.len() % 2 != 0 || !radius.is_finite() || radius < 0. {
            return Err(JsValue::from_str("Invalid series"));
        }
        let c = &self.ctx;
        c.save();
        c.begin_path();
        c.rect(self.rect[0], self.rect[1], self.rect[2], self.rect[3]);
        c.clip();
        c.set_stroke_style_str(color);
        c.set_fill_style_str(color);
        c.set_line_width(self.line_width);
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

    /// Render arrays, axes, automatic numeric/UTC ticks and a wrapping legend
    /// wholly in Rust/WASM. No image or axis request to a server is required.
    pub fn render_plot(
        &mut self,
        spec_json: &str,
        width: f64,
        height: f64,
        dpr: f64,
        viewport: &[f64],
    ) -> Result<Vec<f64>, JsValue> {
        let spec: SeriesSpec =
            serde_json::from_str(spec_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        if viewport.len() != 4
            || !viewport
                .iter()
                .chain(spec.bounds.iter())
                .chain([width, height, dpr].iter())
                .all(|v| v.is_finite())
            || width < if spec.compact_axes { 150. } else { 260. }
            || height < if spec.compact_axes { 90. } else { 150. }
            || dpr <= 0.
            || spec.bounds[2] <= spec.bounds[0]
            || spec.bounds[3] <= spec.bounds[1]
            || viewport[0] < 0.
            || viewport[1] < 0.
            || viewport[2] > 1.
            || viewport[3] > 1.
            || viewport[2] <= viewport[0]
            || viewport[3] <= viewport[1]
            || spec.xlabel.is_empty()
            || spec.ylabel.is_empty()
        {
            return Err(JsValue::from_str("Invalid series plot specification"));
        }
        if let Some(sky) = &spec.sky {
            return self.render_sky(&spec, sky, width, height, dpr, viewport);
        }
        let b = spec.bounds;
        let x0 = b[0] + viewport[0] * (b[2] - b[0]);
        let x1 = b[0] + viewport[2] * (b[2] - b[0]);
        let y0 = b[1] + viewport[1] * (b[3] - b[1]);
        let y1 = b[1] + viewport[3] * (b[3] - b[1]);
        self.width = width;
        self.height = height;
        self.bounds = [x0, x1, y0, y1];
        self.canvas.set_width((width * dpr).round() as u32);
        self.canvas.set_height((height * dpr).round() as u32);
        let c = &self.ctx;
        c.set_transform(dpr, 0., 0., dpr, 0., 0.)?;
        c.set_fill_style_str("#ffffff");
        c.fill_rect(0., 0., width, height);
        c.set_font(if spec.compact_axes {
            "11px RustplotlibPagella, Palatino, serif"
        } else {
            "16px RustplotlibPagella, Palatino, serif"
        });
        c.set_text_align("left");
        let yt = nice_ticks(y0, y1, if spec.compact_axes { 3 } else { 5 });
        let yl: Vec<_> = yt.iter().map(|v| number_label(*v, y1 - y0)).collect();
        let tick_width = yl
            .iter()
            .map(|s| c.measure_text(s).map(|m| m.width()).unwrap_or(64.))
            .fold(0_f64, f64::max);
        let left = if spec.compact_axes {
            tick_width + 9.
        } else {
            (tick_width + 43.).max(if width < 450. { 76. } else { 90. })
        };
        let right = if spec.compact_axes {
            13.
        } else if spec.field.is_some() {
            88.
        } else if spec.utc_origin_seconds.is_some() {
            48.
        } else {
            18.
        };
        let entries: Vec<_> = spec.series.iter().filter(|s| !s.name.is_empty()).collect();
        let entry_width = entries
            .iter()
            .map(|s| {
                c.measure_text(&s.name)
                    .map(|m| m.width() + 36.)
                    .unwrap_or(100.)
            })
            .fold(0_f64, f64::max)
            .max(1.);
        let legend_left = if width < 500. { 20. } else { left };
        let legend_right = if width < 500. { 20. } else { right };
        let columns =
            (((width - legend_left - legend_right) / entry_width).floor() as usize).max(1);
        let rows = entries.len().div_ceil(columns);
        let top = if spec.compact_axes { 16. } else { 18. } + rows as f64 * 23.;
        let bottom_margin = if spec.compact_axes {
            if spec.hide_x_labels { 6. } else { 33. }
        } else {
            63.
        };
        self.rect = [
            left,
            top,
            width - left - right,
            height - top - bottom_margin,
        ];
        if spec.equal_aspect {
            self.rect = equal_aspect_rect(self.rect, x1 - x0, y1 - y0);
        }
        let [left, top, plot_width, plot_height] = self.rect;
        let right = width - left - plot_width;
        let bottom = top + plot_height;
        if self.rect[2] < 70. || self.rect[3] < 35. {
            return Err(JsValue::from_str("Series panel is too small"));
        }
        if !spec.panel_label.is_empty() {
            c.set_text_align("right");
            c.set_fill_style_str("#172134");
            c.fill_text(&spec.panel_label, width - right, 11.)?;
        }
        if let Some(field) = &spec.field {
            self.color_limits = field.limits;
            self.draw_field(field)?;
        }
        for (i, s) in entries.iter().enumerate() {
            let x = legend_left + (i % columns) as f64 * entry_width;
            let y = 16. + (i / columns) as f64 * 23.;
            c.set_stroke_style_str(&s.color);
            c.set_line_width(2.);
            c.begin_path();
            c.move_to(x, y - 4.);
            c.line_to(x + 17., y - 4.);
            c.stroke();
            c.set_fill_style_str("#172134");
            c.fill_text(&s.name, x + 23., y)?;
        }
        c.set_line_width(1.);
        c.set_text_align("right");
        for (v, label) in yt.iter().zip(&yl) {
            let py = self.py(*v);
            c.set_stroke_style_str("#e1e7eb");
            c.begin_path();
            c.move_to(left, py);
            c.line_to(width - right, py);
            c.stroke();
            c.set_fill_style_str("#172134");
            c.fill_text(
                label,
                left - if spec.compact_axes { 4. } else { 9. },
                py + if spec.compact_axes { 3. } else { 5. },
            )?;
        }
        let target = ((self.rect[2] / 145.).floor() as usize).clamp(3, 7);
        let xt = if spec.categories.is_empty() {
            nice_ticks(x0, x1, target)
        } else {
            (0..spec.categories.len())
                .map(|i| i as f64)
                .filter(|v| *v >= x0 && *v <= x1)
                .collect()
        };
        c.set_text_align("center");
        for v in xt {
            let label = if !spec.categories.is_empty() {
                spec.categories
                    .get(v.round() as usize)
                    .cloned()
                    .unwrap_or_default()
            } else if let Some(origin) = spec.utc_origin_seconds {
                chrono::DateTime::<chrono::Utc>::from_timestamp(
                    (origin + v * 3600.).round() as i64,
                    0,
                )
                .map(|t| {
                    t.format(if x1 - x0 < 0.04 {
                        "%H:%M:%S"
                    } else if x1 - x0 > 24. && width < 500. {
                        "%m-%d"
                    } else if x1 - x0 > 24. {
                        "%m-%d %H:%M"
                    } else {
                        "%H:%M"
                    })
                    .to_string()
                })
                .unwrap_or_default()
            } else {
                number_label(v, x1 - x0)
            };
            let px = self.px(v);
            c.set_stroke_style_str("#e1e7eb");
            c.begin_path();
            c.move_to(px, top);
            c.line_to(px, bottom);
            c.stroke();
            c.set_fill_style_str("#172134");
            if !spec.hide_x_labels {
                c.fill_text(
                    &label,
                    px,
                    bottom + if spec.compact_axes { 12. } else { 22. },
                )?;
            }
        }
        c.set_stroke_style_str("#526b7d");
        c.begin_path();
        c.move_to(left, top);
        c.line_to(left, bottom);
        c.line_to(width - right, bottom);
        c.stroke();
        if !spec.hide_x_labels {
            c.fill_text(
                &spec.xlabel,
                left + self.rect[2] / 2.,
                bottom + if spec.compact_axes { 28. } else { 53. },
            )?;
        }
        if !spec.compact_axes {
            c.save();
            c.translate(17., top + self.rect[3] / 2.)?;
            c.rotate(-std::f64::consts::FRAC_PI_2)?;
            c.fill_text(&spec.ylabel, 0., 0.)?;
            c.restore();
        }
        for s in &spec.series {
            self.line_width = s.line_width;
            if let Some(color) = &s.fill {
                self.fill_polygon(s, color)?;
            }
            let xy: Vec<f64> = s
                .points
                .iter()
                .flat_map(|p| p.iter().map(|v| v.unwrap_or(f64::NAN)))
                .collect();
            if spec.bars {
                self.draw_bars(s)?;
            } else {
                self.series(&xy, &s.color, s.line, s.radius)?;
                self.draw_errors(s)?;
            }
        }
        Ok(self.rect.to_vec())
    }
}
impl SeriesView {
    fn px(&self, x: f64) -> f64 {
        self.rect[0] + (x - self.bounds[0]) / (self.bounds[1] - self.bounds[0]) * self.rect[2]
    }
    fn py(&self, y: f64) -> f64 {
        self.rect[1] + self.rect[3]
            - (y - self.bounds[2]) / (self.bounds[3] - self.bounds[2]) * self.rect[3]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_ticks_have_context() {
        for (a, b) in [(-99.67, 99.67), (0., 0.0001), (35., 43.), (10000., 20000.)] {
            let t = nice_ticks(a, b, 5);
            assert!(t.len() >= 3);
            assert!(t.iter().all(|v| *v >= a && *v <= b));
        }
    }
    #[test]
    fn numeric_labels_preserve_units_and_small_values() {
        assert_eq!(number_label(40., 10.), "40");
        assert_eq!(number_label(0., 0.001), "0");
        assert_eq!(number_label(0.00002, 0.0001), "2.0e-5");
    }
}
