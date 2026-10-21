//! Scientific raster panels: the library owns axes, colorbar, and GPU rendering.
use crate::{FieldView, error};
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
        self.bounds = [x0, x1, y0, y1];
        self.field.set_data(nx, ny, values)?;
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
        self.field.set_colormap(&[1., 1., 1., 0.02, 0.08, 0.13])?;
        self.low = low;
        self.high = high;
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
            let red = (255. * (1. - 0.98 * z)) as u32;
            let green = (255. * (1. - 0.92 * z)) as u32;
            let blue = (255. * (1. - 0.87 * z)) as u32;
            c.set_fill_style_str(&format!("rgb({red},{green},{blue})"));
            c.fill_rect(bx, t + h * (1. - z), 13., h / 127. + 1.);
        }
        c.set_fill_style_str("#243746");
        c.set_text_align("left");
        for i in 0..=4 {
            let z = i as f64 / 4.;
            let v = self.low as f64 + (self.high - self.low) as f64 * z;
            c.fill_text(&format!("{v:.0}"), bx + 18., t + h * (1. - z) + 4.)?;
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
            let s = v.rem_euclid(86400.) as u64;
            if step < 60. {
                format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
            } else {
                format!("{:02}:{:02}", s / 3600, s / 60 % 60)
            }
        } else if step >= 1. {
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
