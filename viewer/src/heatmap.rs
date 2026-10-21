//! Scientific raster panels: the library owns axes, colorbar, and GPU rendering.
use crate::{FieldView, error};
#[path = "../../src/palettes.rs"]
#[allow(dead_code)]
mod palettes;
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
#[wasm_bindgen]
pub struct HeatmapView {
    field: FieldView,
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    bounds: [f64; 4],
    view: [f64; 4],
    width: f64,
    height: f64,
    low: f32,
    high: f32,
    xlabel: String,
    ylabel: String,
    label: String,
    time: bool,
    colors: Vec<f32>,
    points: Vec<(f64, f64, f32)>,
    data: Vec<f32>,
    shape: [u32; 2],
    markers: bool,
    bands: Vec<(f64, f64, String)>,
}
#[wasm_bindgen]
impl HeatmapView {
    #[wasm_bindgen(constructor)]
    pub fn new(
        field_canvas: HtmlCanvasElement,
        axes_canvas: HtmlCanvasElement,
    ) -> Result<Self, JsValue> {
        let ctx = axes_canvas
            .get_context("2d")?
            .ok_or_else(|| error("Canvas2D required"))?
            .dyn_into()?;
        let field = FieldView::new(field_canvas)?;
        Ok(Self {
            field,
            canvas: axes_canvas,
            ctx,
            bounds: [0., 1., 0., 1.],
            view: [0., 0., 1., 1.],
            width: 600.,
            height: 400.,
            low: 0.,
            high: 20.,
            xlabel: String::new(),
            ylabel: String::new(),
            label: String::new(),
            time: false,
            colors: vec![1., 1., 1., 0.02, 0.08, 0.13],
            points: Vec::new(),
            data: Vec::new(),
            shape: [0, 0],
            markers: false,
            bands: Vec::new(),
        })
    }
    pub fn set_data(
        &mut self,
        nx: u32,
        ny: u32,
        values: &[f32],
        x0: f64,
        x1: f64,
        y0: f64,
        y1: f64,
    ) -> Result<(), JsValue> {
        if ![x0, x1, y0, y1].iter().all(|v| v.is_finite()) || x1 <= x0 || y1 <= y0 {
            return Err(error("Invalid physical bounds"));
        }
        self.field.set_data(nx, ny, values)?;
        self.data = values.to_vec();
        self.shape = [nx, ny];
        self.refresh_points();
        self.bounds = [x0, x1, y0, y1];
        self.view = [0., 0., 1., 1.];
        self.draw()
    }
    pub fn set_labels(&mut self, x: &str, y: &str, color: &str, time: bool) -> Result<(), JsValue> {
        self.xlabel = x.into();
        self.ylabel = y.into();
        self.label = color.into();
        self.time = time;
        self.draw()
    }
    pub fn set_scale(&mut self, low: f32, high: f32) -> Result<(), JsValue> {
        self.field.set_scale(low, high, 0, 1., 1)?;
        self.field.set_colormap(&self.colors)?;
        self.low = low;
        self.high = high;
        self.draw()
    }
    /// Perceptually uniform scientific palette, shared by texture and colorbar.
    pub fn set_palette(&mut self, name: &str) -> Result<(), JsValue> {
        self.colors = if name == "plasma" {
            (0..32)
                .flat_map(|i| palettes::PLASMA[i * 255 / 31].map(|v| v as f32))
                .collect()
        } else if name == "greys" || name == "gray" {
            vec![1., 1., 1., 0.02, 0.08, 0.13]
        } else {
            return Err(error("Palette must be plasma or greys"));
        };
        self.field.set_colormap(&self.colors)?;
        self.draw()
    }
    /// Choose nearest-cell sampling or a finite maximum over the screen pixel.
    pub fn set_reduction(&mut self, reduction: &str) -> Result<(), JsValue> {
        self.field.set_reduction(reduction)?;
        self.draw()
    }
    /// Explicit point marks preserve sparse detections at any panel resolution.
    pub fn set_markers(&mut self, enabled: bool) -> Result<(), JsValue> {
        self.markers = enabled;
        self.refresh_points();
        self.draw()
    }
    pub fn clear_bands(&mut self) -> Result<(), JsValue> {
        self.bands.clear();
        self.draw()
    }
    pub fn add_band(&mut self, low: f64, high: f64, label: &str) -> Result<(), JsValue> {
        if !low.is_finite() || !high.is_finite() || high <= low {
            return Err(error("Invalid band"));
        }
        self.bands.push((low, high, label.into()));
        self.draw()
    }
    pub fn set_view(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) -> Result<(), JsValue> {
        self.field
            .set_view(x0 as f32, y0 as f32, x1 as f32, y1 as f32)?;
        self.view = [x0, y0, x1, y1];
        self.draw()
    }
    /// CSS plot rectangle. Position the field canvas here; all labels stay in Rust.
    pub fn plot_rect(&self) -> Vec<f64> {
        vec![
            70.,
            22.,
            (self.width - 165.).max(1.),
            (self.height - 80.).max(1.),
        ]
    }
    pub fn resize(&mut self, width: f64, height: f64, dpr: f64) -> Result<(), JsValue> {
        if ![width, height, dpr].iter().all(|v| v.is_finite())
            || width < 250.
            || height < 160.
            || dpr <= 0.
        {
            return Err(error("Invalid panel dimensions"));
        }
        self.width = width;
        self.height = height;
        self.canvas.set_width((width * dpr).round() as u32);
        self.canvas.set_height((height * dpr).round() as u32);
        let r = self.plot_rect();
        self.field.canvas.set_width((r[2] * dpr).round() as u32);
        self.field.canvas.set_height((r[3] * dpr).round() as u32);
        self.ctx.set_transform(dpr, 0., 0., dpr, 0., 0.)?;
        self.draw()
    }
    pub fn draw(&self) -> Result<(), JsValue> {
        self.field.draw();
        let c = &self.ctx;
        let r = self.plot_rect();
        let (l, t, w, h) = (r[0], r[1], r[2], r[3]);
        c.clear_rect(0., 0., self.width, self.height);
        c.set_font("13px Georgia, serif");
        c.set_fill_style_str("#243746");
        c.set_stroke_style_str("#61717b");
        c.set_line_width(0.7);
        c.stroke_rect(l, t, w, h);
        let [xa, xb, ya, yb] = self.bounds;
        let [u0, v0, u1, v1] = self.view;
        let x0 = xa + (xb - xa) * u0;
        let x1 = xa + (xb - xa) * u1;
        let y0 = ya + (yb - ya) * v0;
        let y1 = ya + (yb - ya) * v1;
        c.save();
        c.begin_path();
        c.rect(l, t, w, h);
        c.clip();
        for (i, (a, b, label)) in self.bands.iter().enumerate() {
            let top = t + h - (b - y0) / (y1 - y0) * h;
            let bottom = t + h - (a - y0) / (y1 - y0) * h;
            c.set_fill_style_str("rgba(65,93,125,0.09)");
            c.fill_rect(l, top, w, bottom - top);
            c.set_stroke_style_str("rgba(65,93,125,0.4)");
            c.begin_path();
            c.move_to(l, top);
            c.line_to(l + w, top);
            c.move_to(l, bottom);
            c.line_to(l + w, bottom);
            c.stroke();
            if bottom > t && top < t + h {
                let py = (top + 13.).max(t + 13.).min(t + h - 4.);
                let px = l + 8. + (i % 2) as f64 * w * 0.48;
                c.set_font("11px system-ui");
                c.set_text_align("left");
                let tw = c.measure_text(label)?.width();
                c.set_fill_style_str("rgba(255,255,255,0.85)");
                c.fill_rect(px - 2., py - 11., tw + 4., 14.);
                c.set_fill_style_str("#314c66");
                c.fill_text(label, px, py)?;
            }
        }
        if self.markers {
            for &(u, v, z) in &self.points {
                if u < u0 || u > u1 || v < v0 || v > v1 {
                    continue;
                }
                c.set_fill_style_str(&color(
                    &self.colors,
                    ((z - self.low) / (self.high - self.low)) as f64,
                ));
                c.begin_path();
                c.arc(
                    l + (u - u0) / (u1 - u0) * w,
                    t + h - (v - v0) / (v1 - v0) * h,
                    1.7,
                    0.,
                    std::f64::consts::TAU,
                )?;
                c.fill();
            }
        }
        c.restore();
        c.set_font("13px Georgia, serif");
        c.set_fill_style_str("#243746");
        c.set_stroke_style_str("#61717b");
        c.set_text_align("center");
        for (x, label) in ticks(x0, x1, if self.time { 4 } else { 5 }, self.time) {
            let px = l + (x - x0) / (x1 - x0) * w;
            c.begin_path();
            c.move_to(px, t + h);
            c.line_to(px, t + h + 5.);
            c.stroke();
            c.fill_text(&label, px, t + h + 21.)?;
        }
        c.fill_text(&self.xlabel, l + w / 2., self.height - 12.)?;
        c.set_text_align("right");
        for (y, label) in ticks(y0, y1, 5, false) {
            let py = t + h - (y - y0) / (y1 - y0) * h;
            c.begin_path();
            c.move_to(l - 5., py);
            c.line_to(l, py);
            c.stroke();
            c.fill_text(&label, l - 9., py + 4.)?;
        }
        c.save();
        c.translate(17., t + h / 2.)?;
        c.rotate(-std::f64::consts::FRAC_PI_2)?;
        c.set_text_align("center");
        c.fill_text(&self.ylabel, 0., 0.)?;
        c.restore();
        let bx = l + w + 18.;
        for i in 0..128 {
            let z = i as f64 / 127.;
            c.set_fill_style_str(&color(&self.colors, z));
            c.fill_rect(bx, t + h * (1. - z), 13., h / 127. + 1.);
        }
        c.set_fill_style_str("#243746");
        c.set_text_align("left");
        for i in 0..=4 {
            let z = i as f64 / 4.;
            let v = self.low as f64 + (self.high - self.low) as f64 * z;
            c.fill_text(
                &format!(
                    "{v:.digits$}",
                    digits = ((-((self.high - self.low) as f64 / 4.).log10()).ceil() + 1.)
                        .clamp(0., 8.) as usize
                ),
                bx + 18.,
                t + h * (1. - z) + 4.,
            )?;
        }
        c.save();
        c.translate(self.width - 10., t + h / 2.)?;
        c.rotate(-std::f64::consts::FRAC_PI_2)?;
        c.set_text_align("center");
        c.fill_text(&self.label, 0., 0.)?;
        c.restore();
        Ok(())
    }
}
impl HeatmapView {
    fn refresh_points(&mut self) {
        let [nx, ny] = self.shape;
        self.points = if self.markers && nx > 0 && ny > 0 {
            self.data
                .iter()
                .enumerate()
                .filter(|(_, v)| v.is_finite())
                .map(|(i, v)| {
                    (
                        ((i % nx as usize) as f64 + 0.5) / nx as f64,
                        ((i / nx as usize) as f64 + 0.5) / ny as f64,
                        *v,
                    )
                })
                .collect()
        } else {
            Vec::new()
        };
    }
}
fn color(stops: &[f32], z: f64) -> String {
    let p = z.clamp(0., 1.) * ((stops.len() / 3 - 1) as f64);
    let i = (p.floor() as usize).min(stops.len() / 3 - 2);
    let f = p - i as f64;
    let v: Vec<_> = (0..3)
        .map(|k| {
            (255. * (stops[3 * i + k] as f64 * (1. - f) + stops[3 * i + 3 + k] as f64 * f)).round()
                as u32
        })
        .collect();
    format!("rgb({},{},{})", v[0], v[1], v[2])
}
fn ticks(a: f64, b: f64, n: usize, time: bool) -> Vec<(f64, String)> {
    let desired = (b - a) / n as f64;
    let step = if time {
        [
            1., 2., 5., 10., 15., 30., 60., 120., 300., 600., 900., 1800., 3600., 7200., 21600.,
            86400.,
        ]
        .into_iter()
        .find(|&v| v >= desired)
        .unwrap_or(86400.)
    } else {
        let p = 10f64.powf(desired.log10().floor());
        [1., 2., 2.5, 5., 10.]
            .into_iter()
            .map(|v| v * p)
            .find(|&v| v >= desired)
            .unwrap_or(p * 10.)
    };
    let mut out = Vec::new();
    for i in (a / step).ceil() as i64..=(b / step).floor() as i64 {
        let v = i as f64 * step;
        let label = if time {
            let s = if (v - b).abs() < 0.1 && v.rem_euclid(86400.) < 0.1 && b - a >= 86399. {
                86400
            } else {
                v.rem_euclid(86400.) as u64
            };
            if step < 60. {
                format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
            } else {
                format!("{:02}:{:02}", s / 3600, s / 60 % 60)
            }
        } else if step >= 1. && step.fract().abs() < 1e-9 {
            format!("{v:.0}")
        } else {
            let digits = (-step.log10().floor() + 1.).max(1.) as usize;
            format!("{v:.digits$}")
        };
        out.push((v, label))
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ticks_are_physical() {
        assert_eq!(
            ticks(1., 18., 5, false)
                .iter()
                .map(|p| p.0)
                .collect::<Vec<_>>(),
            vec![5., 10., 15.]
        );
    }
    #[test]
    fn utc_ticks() {
        assert!(
            ticks(1792612800., 1792612860., 4, true)
                .iter()
                .all(|(_, s)| s.contains(':'))
        )
    }
}
