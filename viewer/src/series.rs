//! Lightweight scientific line/scatter panels for live browser diagnostics.
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
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
        self.map = false;
        self.time = false;
        self.labels.clear();
        self.canvas.set_width((width * dpr).round() as u32);
        self.canvas.set_height((height * dpr).round() as u32);
        let c = &self.ctx;
        c.set_transform(dpr, 0., 0., dpr, 0., 0.)?;
        self.draw_axes(left, right)
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
        let ratio = (width - 75.) / (height - 55.);
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
        self.ctx.clear_rect(0., 0., width, height);
        self.draw_axes("Longitude (°E)", "")
    }
    /// Paired x,y samples; nonfinite samples break lines. Markers stay in CSS px.
    pub fn series(&self, xy: &[f64], color: &str, line: bool, radius: f64) -> Result<(), JsValue> {
        if xy.len() % 2 != 0 || !radius.is_finite() || radius < 0. {
            return Err(JsValue::from_str("Invalid series"));
        }
        let c = &self.ctx;
        c.save();
        c.begin_path();
        c.rect(55., 20., self.width - 75., self.height - 55.);
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
            let py = (anchor[1] + offset[1]).clamp(21., self.height - 57.);
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
        c.set_font("18px RustplotlibPagella, Palatino, serif");
        c.set_fill_style_str("#314858");
        c.set_stroke_style_str("#e2e7ea");
        c.set_line_width(1.);
        c.set_text_align("right");
        for j in 0..5 {
            let v = self.bounds[2] + (self.bounds[3] - self.bounds[2]) * j as f64 / 4.;
            let py = self.height - 35. - (self.height - 55.) * j as f64 / 4.;
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
                c.move_to(px, 20.);
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
            - (y - self.bounds[2]) / (self.bounds[3] - self.bounds[2]) * (self.height - 55.)
    }
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
    fn map_projection_preserves_latitudes_and_local_scale() {
        for lat in [-80., -45., 0., 54., 69.7, 80.] {
            assert!((inverse_mercator(mercator(lat)) - lat).abs() < 1e-10);
        }
        let dy = (mercator(69.7001) - mercator(69.7)) / 0.0001;
        assert!((dy * 69.7_f64.to_radians().cos() - 1.).abs() < 1e-5);
    }
}
