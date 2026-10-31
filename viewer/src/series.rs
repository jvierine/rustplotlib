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
    margins: [f64; 4],
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
            margins: [55., 20., 20., 35.],
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
        self.margins = [55., 20., 20., 35.];
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
    /// Conventional centered horizontal and rotated vertical labels, with room
    /// for ticks and an optional secondary x scale. Leaves existing begin() intact.
    pub fn begin_axes(
        &mut self,
        width: f64,
        height: f64,
        dpr: f64,
        x0: f64,
        x1: f64,
        y0: f64,
        y1: f64,
        xlabel: &str,
        ylabel: &str,
    ) -> Result<(), JsValue> {
        if ![width, height, dpr, x0, x1, y0, y1]
            .iter()
            .all(|v| v.is_finite())
            || width < 150.
            || height < 120.
            || dpr <= 0.
            || x1 <= x0
            || y1 <= y0
        {
            return Err(JsValue::from_str("Invalid axes bounds"));
        }
        self.width = width;
        self.height = height;
        self.bounds = [x0, x1, y0, y1];
        self.margins = [70., 18., 30., 48.];
        self.canvas.set_width((width * dpr).round() as u32);
        self.canvas.set_height((height * dpr).round() as u32);
        let c = &self.ctx;
        c.set_transform(dpr, 0., 0., dpr, 0., 0.)?;
        c.set_font("16px Palatino, 'Palatino Linotype', 'Book Antiqua', serif");
        c.set_fill_style_str("#222");
        c.set_stroke_style_str("#444");
        c.set_line_width(0.7);
        c.set_text_align("right");
        // Match the native gallery: open axes, outward ticks, no default grid.
        c.begin_path();
        c.move_to(70., 30.);
        c.line_to(70., height - 48.);
        c.line_to(width - 18., height - 48.);
        c.stroke();
        let desired = (y1 - y0) / 4.;
        let power = 10_f64.powf(desired.log10().floor());
        let step = [1., 2., 2.5, 5., 10.]
            .into_iter()
            .map(|v| v * power)
            .min_by(|a, b| {
                (a / desired)
                    .ln()
                    .abs()
                    .total_cmp(&(b / desired).ln().abs())
            })
            .unwrap();
        let precision = (0..=6)
            .find(|&d| {
                let v = step * 10_f64.powi(d);
                (v - v.round()).abs() < 1e-8
            })
            .unwrap_or(6) as usize;
        for j in (y0 / step).ceil() as i64..=(y1 / step).floor() as i64 {
            let y = j as f64 * step;
            let py = self.py(y);
            c.begin_path();
            c.move_to(65.5, py);
            c.line_to(70., py);
            c.stroke();
            let label = format!("{y:.precision$}");
            c.fill_text(&label, 61., py + 5.)?;
        }
        c.set_font("17px Palatino, 'Palatino Linotype', 'Book Antiqua', serif");
        c.set_fill_style_str("#222");
        c.set_text_align("center");
        c.fill_text(xlabel, (70. + width - 18.) / 2., height - 7.)?;
        c.save();
        c.translate(15., (30. + height - 48.) / 2.)?;
        c.rotate(-std::f64::consts::FRAC_PI_2)?;
        c.fill_text(ylabel, 0., 0.)?;
        c.restore();
        c.set_text_align("left");
        Ok(())
    }
    /// Standalone secondary y axis using the same typography and ticks as begin_axes.
    pub fn right_axis(
        &self,
        width: f64,
        height: f64,
        dpr: f64,
        y0: f64,
        y1: f64,
        label: &str,
    ) -> Result<(), JsValue> {
        if ![width, height, dpr, y0, y1].iter().all(|v| v.is_finite()) || y1 <= y0 || dpr <= 0. {
            return Err(JsValue::from_str("Invalid secondary axis"));
        }
        self.canvas.set_width((width * dpr).round() as u32);
        self.canvas.set_height((height * dpr).round() as u32);
        let c = &self.ctx;
        c.set_transform(dpr, 0., 0., dpr, 0., 0.)?;
        c.set_font("16px Palatino, 'Palatino Linotype', 'Book Antiqua', serif");
        c.set_fill_style_str("#222");
        c.set_stroke_style_str("#444");
        c.set_line_width(0.7);
        c.set_text_align("left");
        c.begin_path();
        c.move_to(1., 30.);
        c.line_to(1., height - 48.);
        c.stroke();
        let desired = (y1 - y0) / 4.;
        let power = 10_f64.powf(desired.log10().floor());
        let step = [1., 2., 2.5, 5., 10.]
            .into_iter()
            .map(|v| v * power)
            .min_by(|a, b| {
                (a / desired)
                    .ln()
                    .abs()
                    .total_cmp(&(b / desired).ln().abs())
            })
            .unwrap();
        let precision = (0..=6)
            .find(|&d| {
                let v = step * 10_f64.powi(d);
                (v - v.round()).abs() < 1e-8
            })
            .unwrap_or(6) as usize;
        for j in (y0 / step).ceil() as i64..=(y1 / step).floor() as i64 {
            let y = j as f64 * step;
            let py = height - 48. - (y - y0) / (y1 - y0) * (height - 78.);
            c.begin_path();
            c.move_to(1., py);
            c.line_to(5.5, py);
            c.stroke();
            c.fill_text(&format!("{y:.precision$}"), 10., py + 5.)?;
        }
        c.set_font("17px Palatino, 'Palatino Linotype', 'Book Antiqua', serif");
        c.set_text_align("center");
        c.save();
        c.translate(width - 15., (height - 18.) / 2.)?;
        c.rotate(std::f64::consts::FRAC_PI_2)?;
        c.fill_text(label, 0., 0.)?;
        c.restore();
        Ok(())
    }
    pub fn x_ticks(&self, wrapped: bool, top: bool) -> Result<(), JsValue> {
        let c = &self.ctx;
        c.set_font("16px Palatino, 'Palatino Linotype', 'Book Antiqua', serif");
        c.set_text_align("center");
        c.set_fill_style_str("#222");
        let span = self.bounds[1] - self.bounds[0];
        let desired = span / 5.;
        let power = 10_f64.powf(desired.log10().floor());
        let step = if span > 300. && span <= 500. {
            90.
        } else {
            [1., 2., 2.5, 5., 10.]
                .into_iter()
                .map(|v| v * power)
                .find(|&v| v >= desired)
                .unwrap_or(power * 10.)
        };
        for j in (self.bounds[0] / step).ceil() as i64..=(self.bounds[1] / step).floor() as i64 {
            let x = j as f64 * step;
            let v = if wrapped {
                (x + 180.).rem_euclid(360.) - 180.
            } else {
                x
            };
            let label = if step >= 1. {
                format!("{v:.0}")
            } else {
                format!("{v:.2}")
            };
            let y = if top {
                self.margins[2]
            } else {
                self.height - self.margins[3]
            };
            c.begin_path();
            c.move_to(self.px(x), y);
            c.line_to(self.px(x), y + if top { -4.5 } else { 4.5 });
            c.stroke();
            c.fill_text(&label, self.px(x), if top { y - 12. } else { y + 22. })?;
        }
        c.set_text_align("left");
        Ok(())
    }
    /// UTC ticks for elapsed-second coordinates, anchored at a Unix epoch.
    pub fn time_ticks(&self, epoch: f64) -> Result<(), JsValue> {
        let c = &self.ctx;
        c.set_font("16px Palatino, 'Palatino Linotype', 'Book Antiqua', serif");
        c.set_text_align("center");
        c.set_fill_style_str("#222");
        let desired = (self.bounds[1] - self.bounds[0]) / ((self.width - 88.) / 85.).ceil().max(3.);
        let step = [
            1., 2., 5., 10., 15., 30., 60., 120., 300., 600., 900., 1200., 1800., 3600., 7200.,
            21600., 86400.,
        ]
        .into_iter()
        .find(|&s| s >= desired)
        .unwrap_or(86400.);
        for j in ((epoch + self.bounds[0]) / step).ceil() as i64
            ..=((epoch + self.bounds[1]) / step).floor() as i64
        {
            let t = j as f64 * step;
            let seconds = t.rem_euclid(86400.).round() as i64;
            let h = seconds / 3600;
            let m = (seconds % 3600) / 60;
            let s = seconds % 60;
            let label = if step < 60. {
                format!("{h:02}:{m:02}:{s:02}")
            } else {
                format!("{h:02}:{m:02}")
            };
            c.fill_text(
                &label,
                self.px(t - epoch),
                self.height - self.margins[3] + 18.,
            )?;
        }
        c.set_text_align("left");
        Ok(())
    }
    pub fn series(&self, xy: &[f64], color: &str, line: bool, radius: f64) -> Result<(), JsValue> {
        if xy.len() % 2 != 0 || !radius.is_finite() || radius < 0. {
            return Err(JsValue::from_str("Invalid series"));
        }
        let c = &self.ctx;
        c.save();
        c.begin_path();
        c.rect(
            self.margins[0],
            self.margins[2],
            self.width - self.margins[0] - self.margins[1],
            self.height - self.margins[2] - self.margins[3],
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
    pub fn node_label(&self, x: f64, y: f64, label: &str, color: &str) -> Result<(), JsValue> {
        if !x.is_finite()
            || !y.is_finite()
            || x < self.bounds[0]
            || x > self.bounds[1]
            || y < self.bounds[2]
            || y > self.bounds[3]
        {
            return Ok(());
        }
        let c = &self.ctx;
        let px = self.px(x);
        let py = self.py(y);
        c.save();
        c.set_font("14px Palatino, 'Palatino Linotype', 'Book Antiqua', serif");
        c.set_fill_style_str(color);
        c.set_text_align(if px > self.width - self.margins[1] - 35. {
            "right"
        } else {
            "left"
        });
        c.fill_text(
            label,
            if px > self.width - self.margins[1] - 35. {
                px - 7.
            } else {
                px + 7.
            },
            if py < self.margins[2] + 16. {
                py + 16.
            } else {
                py - 8.
            },
        )?;
        c.restore();
        Ok(())
    }
}
impl SeriesView {
    fn px(&self, x: f64) -> f64 {
        self.margins[0]
            + (x - self.bounds[0]) / (self.bounds[1] - self.bounds[0])
                * (self.width - self.margins[0] - self.margins[1])
    }
    fn py(&self, y: f64) -> f64 {
        self.height
            - self.margins[3]
            - (y - self.bounds[2]) / (self.bounds[3] - self.bounds[2])
                * (self.height - self.margins[2] - self.margins[3])
    }
}
