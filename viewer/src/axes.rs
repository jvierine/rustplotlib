// Shared scientific axes for browser fields and curves. Same default family as
// the native mathpazo figures; dimensions and text are in CSS pixels.
#[derive(serde::Deserialize, Default)]
struct Axis {
    #[serde(default)]
    label: String,
    #[serde(default)]
    format: String,
}
#[derive(serde::Deserialize)]
struct Axes {
    width: f64,
    height: f64,
    #[serde(default = "one")]
    dpr: f64,
    bounds: [f64; 4],
    #[serde(default)]
    x: Axis,
    #[serde(default)]
    y: Axis,
    #[serde(default = "margins")]
    margins: [f64; 4],
    #[serde(default)]
    right_y: bool,
    #[serde(default)]
    no_frame: bool,
}
fn one() -> f64 {
    1.
}
fn margins() -> [f64; 4] {
    [76., 22., 24., 58.]
}
fn ticks(lo: f64, hi: f64, format: &str, count: usize) -> Vec<(f64, String)> {
    let target = (hi - lo) / (count.max(2) - 1) as f64;
    let step = if format == "utc" {
        [
            1., 2., 5., 10., 15., 30., 60., 120., 300., 600., 900., 1800., 3600., 7200., 10800.,
            21600., 43200., 86400.,
        ]
        .into_iter()
        .find(|s| *s >= target)
        .unwrap_or(86400.)
    } else if format == "log10" && hi - lo >= 1. {
        target.ceil().max(1.)
    } else {
        let power = 10_f64.powf(target.log10().floor());
        [1., 2., 2.5, 5., 10.]
            .into_iter()
            .find(|s| *s * power >= target)
            .unwrap_or(10.)
            * power
    };
    let mut out = Vec::new();
    let mut value = ((lo - step * 1e-7) / step).ceil() * step;
    while value <= hi + step * 1e-7 && out.len() < 100 {
        if value.abs() < step * 1e-7 {
            value = 0.;
        }
        let label = if format == "utc" {
            let sec = (value.round() as i64).rem_euclid(86400);
            if step < 60. {
                format!("{:02}:{:02}:{:02}", sec / 3600, sec / 60 % 60, sec % 60)
            } else {
                format!("{:02}:{:02}", sec / 3600, sec / 60 % 60)
            }
        } else if format == "log10" && (value - value.round()).abs() < 1e-7 {
            let superscript: String = format!("{:.0}", value)
                .chars()
                .map(|c| match c {
                    '0' => '⁰',
                    '1' => '¹',
                    '2' => '²',
                    '3' => '³',
                    '4' => '⁴',
                    '5' => '⁵',
                    '6' => '⁶',
                    '7' => '⁷',
                    '8' => '⁸',
                    '9' => '⁹',
                    '-' => '⁻',
                    _ => c,
                })
                .collect();
            format!("10{superscript}")
        } else {
            let v = if format == "log10" {
                10_f64.powf(value)
            } else {
                value
            };
            if v != 0. && (v.abs() >= 1e5 || v.abs() < 0.001) {
                format!("{v:.1e}")
            } else {
                let precision = ((-step.log10().floor() + 1.).max(0.).min(6.)) as usize;
                let label = format!("{v:.*}", precision);
                if precision > 0 {
                    label
                        .trim_end_matches('0')
                        .trim_end_matches('.')
                        .to_string()
                } else {
                    label
                }
            }
        };
        out.push((value, label));
        value += step;
    }
    out
}

#[wasm_bindgen]
impl SeriesView {
    /// Draw readable axes, reserving fixed CSS-pixel margins. Bounds are
    /// [x_min, x_max, y_min, y_max]; log10 axes take log10 coordinates.
    pub fn axes(&mut self, options: JsValue) -> Result<(), JsValue> {
        let a: Axes = serde_wasm_bindgen::from_value(options)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let [x0, x1, y0, y1] = a.bounds;
        if ![a.width, a.height, a.dpr, x0, x1, y0, y1]
            .iter()
            .all(|v| v.is_finite())
            || a.dpr <= 0.
            || x1 <= x0
            || y1 <= y0
            || a.margins.iter().any(|m| !m.is_finite() || *m < 0.)
            || a.width <= a.margins[0] + a.margins[2] + 1.
            || a.height <= a.margins[1] + a.margins[3] + 1.
        {
            return Err(JsValue::from_str("Invalid scientific axes"));
        }
        self.width = a.width;
        self.height = a.height;
        self.bounds = a.bounds;
        self.margins = a.margins;
        self.canvas.set_width((a.width * a.dpr).round() as u32);
        self.canvas.set_height((a.height * a.dpr).round() as u32);
        let c = &self.ctx;
        c.set_transform(a.dpr, 0., 0., a.dpr, 0., 0.)?;
        c.set_fill_style_str("#ffffff");
        c.fill_rect(0., 0., a.width, a.height);
        c.set_font("16px RustplotlibPagella, Palatino, serif");
        c.set_fill_style_str("#314858");
        c.set_stroke_style_str("#314858");
        c.set_line_width(0.8);
        let [left, top, right, bottom] = a.margins;
        let w = a.width - left - right;
        let h = a.height - top - bottom;
        if !a.no_frame {
            c.stroke_rect(left, top, w, h);
        }
        if !a.x.format.eq("none") {
            c.set_text_align("center");
            for (v, label) in ticks(
                x0,
                x1,
                &a.x.format,
                ((w / 65.).floor() as usize).clamp(4, 8),
            ) {
                let x = self.px(v);
                c.begin_path();
                c.move_to(x, top + h);
                c.line_to(x, top + h + 5.);
                c.stroke();
                c.fill_text(&label, x, top + h + 23.)?;
            }
            c.set_font("18px RustplotlibPagella, Palatino, serif");
            c.fill_text(&a.x.label, left + w / 2., a.height - 8.)?;
        }
        c.set_font("16px RustplotlibPagella, Palatino, serif");
        if !a.y.format.eq("none") {
            c.set_text_align(if a.right_y { "left" } else { "right" });
            for (v, label) in ticks(
                y0,
                y1,
                &a.y.format,
                ((h / 55.).floor() as usize).clamp(4, 7),
            ) {
                let y = self.py(v);
                let x = if a.right_y { left + w } else { left };
                let d = if a.right_y { 1. } else { -1. };
                c.begin_path();
                c.move_to(x, y);
                c.line_to(x + 5. * d, y);
                c.stroke();
                c.fill_text(&label, x + 9. * d, y + 5.)?;
            }
            if !a.y.label.is_empty() {
                c.save();
                c.set_text_align("center");
                c.set_font("18px RustplotlibPagella, Palatino, serif");
                c.translate(if a.right_y { a.width - 15. } else { 18. }, top + h / 2.)?;
                c.rotate(-std::f64::consts::FRAC_PI_2)?;
                c.fill_text(&a.y.label, 0., 0.)?;
                c.restore();
            }
        }
        c.set_text_align("left");
        Ok(())
    }
    pub fn axes_rect(&self) -> Vec<f64> {
        vec![
            self.margins[0],
            self.margins[1],
            self.width - self.margins[0] - self.margins[2],
            self.height - self.margins[1] - self.margins[3],
        ]
    }
    /// Independent line segments, e.g. measurement error bars. Four values
    /// per segment: x0, y0, x1, y1. Nonfinite segments are omitted.
    pub fn segments(&self, xy: &[f64], color: &str) -> Result<(), JsValue> {
        if xy.len() % 4 != 0 {
            return Err(JsValue::from_str("Invalid segments"));
        }
        let c = &self.ctx;
        c.save();
        self.clip();
        c.set_stroke_style_str(color);
        c.set_line_width(1.);
        c.begin_path();
        for p in xy.chunks_exact(4) {
            if p.iter().all(|v| v.is_finite()) {
                c.move_to(self.px(p[0]), self.py(p[1]));
                c.line_to(self.px(p[2]), self.py(p[3]));
            }
        }
        c.stroke();
        c.restore();
        Ok(())
    }
    /// x, lower, upper triples. Missing uncertainty breaks the band.
    pub fn band(&self, xlu: &[f64], color: &str) -> Result<(), JsValue> {
        if xlu.len() % 3 != 0 {
            return Err(JsValue::from_str("Invalid uncertainty band"));
        }
        let c = &self.ctx;
        c.save();
        self.clip();
        c.set_fill_style_str(color);
        let points: Vec<_> = xlu.chunks_exact(3).collect();
        for segment in points.split(|p| p.iter().any(|v| !v.is_finite())) {
            if segment.len() < 2 {
                continue;
            }
            c.begin_path();
            for (i, p) in segment.iter().enumerate() {
                let x = self.px(p[0]);
                let y = self.py(p[2]);
                if i == 0 {
                    c.move_to(x, y);
                } else {
                    c.line_to(x, y);
                }
            }
            for p in segment.iter().rev() {
                c.line_to(self.px(p[0]), self.py(p[1]));
            }
            c.close_path();
            c.fill();
        }
        c.restore();
        Ok(())
    }
    /// style 0: filled circles, 1: open circles, 2/3: left/right limit triangles.
    pub fn markers(&self, xy: &[f64], color: &str, radius: f64, style: i32) -> Result<(), JsValue> {
        if xy.len() % 2 != 0 || !radius.is_finite() || radius < 0. || !(0..=3).contains(&style) {
            return Err(JsValue::from_str("Invalid markers"));
        }
        let c = &self.ctx;
        c.save();
        self.clip();
        c.set_stroke_style_str(color);
        c.set_fill_style_str(color);
        c.set_line_width(1.3);
        for p in xy.chunks_exact(2) {
            if p.iter().any(|v| !v.is_finite()) {
                continue;
            }
            let x = self.px(p[0]);
            let y = self.py(p[1]);
            c.begin_path();
            if style < 2 {
                c.arc(x, y, radius, 0., std::f64::consts::TAU)?;
            } else {
                let d = if style == 2 { 1. } else { -1. };
                c.move_to(x, y);
                c.line_to(x + d * radius * 2., y - radius);
                c.line_to(x + d * radius * 2., y + radius);
                c.close_path();
            }
            if style == 1 {
                c.stroke();
            } else {
                c.fill();
            }
        }
        c.restore();
        Ok(())
    }
}
impl SeriesView {
    fn clip(&self) {
        let [l, t, r, b] = self.margins;
        self.ctx.begin_path();
        self.ctx.rect(l, t, self.width - l - r, self.height - t - b);
        self.ctx.clip();
    }
}
#[cfg(test)]
mod axes_tests {
    use super::*;
    #[test]
    fn utc_labels_round_to_hour_boundaries() {
        assert_eq!(
            ticks(53999.999, 75600.001, "utc", 8)
                .iter()
                .map(|(_, s)| s.as_str())
                .collect::<Vec<_>>(),
            vec![
                "15:00", "16:00", "17:00", "18:00", "19:00", "20:00", "21:00"
            ]
        );
    }
    #[test]
    fn log_labels_are_integer_powers() {
        assert_eq!(
            ticks(8., 13., "log10", 4),
            vec![
                (8., "10⁸".into()),
                (10., "10¹⁰".into()),
                (12., "10¹²".into())
            ]
        );
    }
    #[test]
    fn zero_is_labelled() {
        assert!(
            ticks(-1000., 1000., "", 5)
                .iter()
                .any(|(v, s)| *v == 0. && s == "0")
        );
    }
}
