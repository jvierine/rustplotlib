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
        self.canvas.set_width((width * dpr).round() as u32);
        self.canvas.set_height((height * dpr).round() as u32);
        let c = &self.ctx;
        c.set_transform(dpr, 0., 0., dpr, 0., 0.)?;
        c.set_font("14px Georgia, serif");
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
}
impl SeriesView {
    fn px(&self, x: f64) -> f64 {
        55. + (x - self.bounds[0]) / (self.bounds[1] - self.bounds[0]) * (self.width - 75.)
    }
    fn py(&self, y: f64) -> f64 {
        self.height
            - 35.
            - (y - self.bounds[2]) / (self.bounds[3] - self.bounds[2]) * (self.height - 55.)
    }
}
