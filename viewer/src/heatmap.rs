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
    x_labels_visible: bool,
    column_support_applied: bool,
    colors: Vec<f32>,
    points: Vec<(f64, f64, f32)>,
    data: Vec<f32>,
    shape: [u32; 2],
    markers: bool,
    point_geometry: bool,
    point_offset: f64,
    font_size: f64,
    marker_size: f64,
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
            x_labels_visible: true,
            column_support_applied: false,
            colors: vec![1., 1., 1., 0.02, 0.08, 0.13],
            points: Vec::new(),
            data: Vec::new(),
            shape: [0, 0],
            markers: false,
            point_geometry: false,
            point_offset: 0.,
            font_size: 16.,
            marker_size: 1.1,
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
        self.point_geometry = false;
        self.data = values.to_vec();
        self.column_support_applied = false;
        self.shape = [nx, ny];
        self.refresh_points();
        self.bounds = [x0, x1, y0, y1];
        self.view = [0., 0., 1., 1.];
        self.draw()
    }
    /// Progressive row upload for a raster. Physical bounds and viewport stay
    /// unchanged; callers do not need to resend the full image or reset zoom.
    pub fn update_rows(&mut self, first_row: u32, values: &[f32]) -> Result<(), JsValue> {
        if self.point_geometry || self.markers {
            return Err(error("Row streaming requires an unmarked raster"));
        }
        self.field.update_rows(first_row, values)?;
        let first = first_row as usize * self.shape[0] as usize;
        self.data[first..first + values.len()].copy_from_slice(values);
        Ok(())
    }
    /// Recorded columns, before any display-only expansion of sample support.
    pub fn sampled_columns(&self) -> Vec<u32> {
        let nx = self.shape[0] as usize;
        (0..nx)
            .filter(|&x| self.data.iter().skip(x).step_by(nx).any(|v| v.is_finite()))
            .map(|x| x as u32)
            .collect()
    }
    /// Nearest sample support for regularly spaced observations. This never
    /// changes stored values, crosses a long outage, or fills missing values
    /// within an explicitly recorded observation. Width is in column units.
    pub fn set_column_support(
        &mut self,
        sources: &[u32],
        max_half_width: f64,
    ) -> Result<bool, JsValue> {
        let nx = self.shape[0] as usize;
        if !max_half_width.is_finite()
            || max_half_width < 0.
            || sources.iter().any(|&x| x as usize >= nx)
            || sources.windows(2).any(|w| w[0] >= w[1])
        {
            return Err(error("Invalid sample support"));
        }
        let expanded = expand_columns(&self.data, nx, sources, max_half_width);
        let applied = expanded.is_some();
        if applied || self.column_support_applied {
            self.field.set_data(
                self.shape[0],
                self.shape[1],
                expanded.as_deref().unwrap_or(&self.data),
            )?;
            self.column_support_applied = applied;
            self.draw()?;
        }
        Ok(applied)
    }
    /// Use one bottom time axis for a stack of aligned scientific panels.
    pub fn set_x_labels_visible(&mut self, visible: bool) -> Result<(), JsValue> {
        self.x_labels_visible = visible;
        self.draw()
    }
    /// Actual sample centers for irregular frequency/range scans. Edge lookup
    /// is normalized before sending to WebGL, preserving large UTC epochs.
    pub fn set_rectilinear_data(
        &mut self,
        x: &[f64],
        y: &[f64],
        values: &[f32],
    ) -> Result<(), JsValue> {
        let (x0, x1, xe) = normalized_edges(x).map_err(error)?;
        let (y0, y1, ye) = normalized_edges(y).map_err(error)?;
        self.field.set_rectilinear_data(&xe, &ye, values)?;
        self.data = values.to_vec();
        self.shape = [x.len() as u32, y.len() as u32];
        self.point_geometry = false;
        self.bounds = [x0, x1, y0, y1];
        self.view = [0., 0., 1., 1.];
        self.refresh_points();
        self.draw()
    }
    /// Packed [x offset, y, scalar] samples with full physical bounds. The
    /// offset keeps float32 UTC positions precise without large epoch values.
    pub fn set_points(
        &mut self,
        xyz: &[f32],
        x0: f64,
        x1: f64,
        y0: f64,
        y1: f64,
        offset: f64,
    ) -> Result<(), JsValue> {
        if xyz.len() % 3 != 0
            || ![x0, x1, y0, y1, offset].iter().all(|v| v.is_finite())
            || x1 <= x0
            || y1 <= y0
        {
            return Err(error("Invalid scalar points"));
        }
        self.field.set_data(1, 1, &[f32::NAN])?;
        self.bounds = [x0, x1, y0, y1];
        self.data = xyz.to_vec();
        self.shape = [0, 0];
        self.point_geometry = true;
        self.point_offset = offset;
        self.markers = true;
        self.refresh_points();
        self.view = [0., 0., 1., 1.];
        self.draw()
    }
    pub fn set_font_size(&mut self, pixels: f64) -> Result<(), JsValue> {
        if !pixels.is_finite() || !(10. ..=32.).contains(&pixels) {
            return Err(error("Font size must be 10–32 pixels"));
        }
        self.font_size = pixels;
        self.draw()
    }
    pub fn set_marker_size(&mut self, pixels: f64) -> Result<(), JsValue> {
        if !pixels.is_finite() || !(0.25..=20.).contains(&pixels) {
            return Err(error("Invalid marker radius"));
        }
        self.marker_size = pixels;
        self.draw()
    }
    pub fn scale_limits(&self) -> Vec<f64> {
        vec![self.low as f64, self.high as f64]
    }
    pub fn color_label(&self) -> String {
        self.label.clone()
    }
    pub fn colorbar_rect(&self) -> Vec<f64> {
        let r = self.plot_rect();
        vec![r[0] + r[2] + 12., r[1] - 5., 70., r[3] + 10.]
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
        } else if name == "rainbow" {
            (0..32)
                .flat_map(|i| {
                    let t = i as f64 / 31.;
                    [
                        (2. * t - 0.5).abs().clamp(0., 1.) as f32,
                        (std::f64::consts::PI * t).sin() as f32,
                        (std::f64::consts::FRAC_PI_2 * t).cos().max(0.) as f32,
                    ]
                })
                .collect()
        } else if name == "viridis" {
            (0..32)
                .flat_map(|i| palettes::VIRIDIS[i * 255 / 31].map(|v| v as f32))
                .collect()
        } else if name == "greys" || name == "gray" {
            vec![1., 1., 1., 0., 0., 0.]
        } else {
            return Err(error("Palette must be plasma, greys, rainbow, or viridis"));
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
        let scale = (self.font_size / 18.).max(1.);
        vec![
            82. * scale,
            12. * scale,
            (self.width - 190. * scale).max(1.),
            (self.height
                - if self.x_labels_visible {
                    76. * scale
                } else {
                    26. * scale
                })
            .max(1.),
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
        c.set_font(&format!(
            "{}px RustplotlibPagella, Palatino, serif",
            self.font_size
        ));
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
                c.set_font(&format!("{}px system-ui", (self.font_size * 0.8).max(12.)));
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
                    self.marker_size,
                    0.,
                    std::f64::consts::TAU,
                )?;
                c.fill();
            }
        }
        c.restore();
        c.set_font(&format!(
            "{}px RustplotlibPagella, Palatino, serif",
            self.font_size
        ));
        c.set_fill_style_str("#243746");
        c.set_stroke_style_str("#61717b");
        c.set_text_align("center");
        if self.x_labels_visible {
            for (x, label) in ticks(
                x0,
                x1,
                if self.time {
                    ((w / 90.).floor() as usize).clamp(2, 4)
                } else {
                    5
                },
                self.time,
            ) {
                let px = l + (x - x0) / (x1 - x0) * w;
                c.begin_path();
                c.move_to(px, t + h);
                c.line_to(px, t + h + 5.);
                c.stroke();
                c.fill_text(&label, px, t + h + self.font_size + 10.)?;
            }
            c.fill_text(&self.xlabel, l + w / 2., self.height - 17.)?;
        }
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
        c.translate(22., t + h / 2.)?;
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
        c.translate(self.width - 14., t + h / 2.)?;
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
        self.points = if self.markers && self.point_geometry {
            let [xa, xb, ya, yb] = self.bounds;
            self.data
                .chunks_exact(3)
                .filter(|p| p.iter().all(|v| v.is_finite()))
                .map(|p| {
                    (
                        (p[0] as f64 + self.point_offset - xa) / (xb - xa),
                        (p[1] as f64 - ya) / (yb - ya),
                        p[2],
                    )
                })
                .collect()
        } else if self.markers && nx > 0 && ny > 0 {
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
fn normalized_edges(a: &[f64]) -> Result<(f64, f64, Vec<f32>), &'static str> {
    if a.len() < 2 || a.iter().any(|v| !v.is_finite()) || a.windows(2).any(|v| v[1] <= v[0]) {
        return Err("Sample centers must increase and be finite");
    }
    let low = a[0] - (a[1] - a[0]) / 2.;
    let n = a.len();
    let high = a[n - 1] + (a[n - 1] - a[n - 2]) / 2.;
    let mut edges = vec![0.];
    edges.extend(
        a.windows(2)
            .map(|v| (((v[0] + v[1]) / 2. - low) / (high - low)) as f32),
    );
    edges.push(1.);
    Ok((low, high, edges))
}
fn expand_columns(
    data: &[f32],
    nx: usize,
    sources: &[u32],
    max_half_width: f64,
) -> Option<Vec<f32>> {
    if nx == 0 || sources.len() < 4 {
        return None;
    }
    let mut gaps: Vec<_> = sources.windows(2).map(|w| (w[1] - w[0]) as f64).collect();
    gaps.sort_by(f64::total_cmp);
    let cadence = gaps[gaps.len() / 2];
    if gaps
        .iter()
        .filter(|&&g| (g - cadence).abs() <= (cadence * 0.25).max(1.))
        .count()
        < 3
    {
        return None;
    }
    let half = ((cadence + 1.) / 2.).min(max_half_width);
    let mut nearest = (0..nx).collect::<Vec<_>>();
    let mut j = 0;
    let mut changed = false;
    for x in sources[0] as usize..=*sources.last().unwrap() as usize {
        while j + 1 < sources.len()
            && (sources[j + 1] as isize - x as isize).abs()
                < (sources[j] as isize - x as isize).abs()
        {
            j += 1;
        }
        let source = sources[j] as usize;
        if (source as f64 - x as f64).abs() <= half
            && source != x
            && !data.iter().skip(x).step_by(nx).any(|v| v.is_finite())
        {
            nearest[x] = source;
            changed = true;
        }
    }
    if !changed {
        return None;
    }
    let mut display = data.to_vec();
    for (row, original) in display.chunks_exact_mut(nx).zip(data.chunks_exact(nx)) {
        for x in 0..nx {
            row[x] = original[nearest[x]];
        }
    }
    Some(display)
}
fn ticks(a: f64, b: f64, n: usize, time: bool) -> Vec<(f64, String)> {
    let desired = (b - a) / n as f64;
    let step = if time {
        [
            1., 2., 5., 10., 15., 30., 60., 120., 300., 600., 900., 1800., 3600., 7200., 10800.,
            21600., 43200., 86400.,
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
    fn irregular_centers_keep_physical_frequency_geometry() {
        let (lo, hi, e) = normalized_edges(&[2., 3., 7.]).unwrap();
        assert_eq!((lo, hi), (1.5, 9.));
        assert_eq!(e.len(), 4);
        assert!((e[1] - 1. / 7.5).abs() < 1e-6);
        assert!(normalized_edges(&[2., 2.]).is_err());
    }
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
    #[test]
    fn narrow_full_day_keeps_a_midday_tick() {
        let start = 1792627200.;
        let labels: Vec<_> = ticks(start, start + 86400., 2, true)
            .into_iter()
            .map(|(_, s)| s)
            .collect();
        assert_eq!(labels, vec!["00:00", "12:00", "24:00"]);
    }
    #[test]
    fn support_fills_cadence_but_preserves_outages_and_raw_values() {
        let sources = [2, 7, 12, 17, 50, 55];
        let mut raw = vec![f32::NAN; 60];
        for &x in &sources {
            raw[x as usize] = x as f32;
        }
        let view = expand_columns(&raw, 60, &sources, 15.).unwrap();
        assert_eq!(view[4], 2.);
        assert_eq!(view[5], 7.);
        assert!(view[30].is_nan());
        assert!(view[0].is_nan());
        assert!(view[59].is_nan());
        assert!(raw[4].is_nan());
    }
    #[test]
    fn recorded_non_detection_is_not_replaced_with_a_neighbor_echo() {
        let sources = [0, 4, 8, 12];
        let mut raw = vec![f32::NAN; 13];
        raw[0] = 7.;
        raw[8] = 8.;
        raw[12] = 9.;
        let view = expand_columns(&raw, 13, &sources, 15.).unwrap();
        assert!(view[3].is_nan());
        assert!(view[4].is_nan());
        assert!(view[5].is_nan());
        assert_eq!(view[7], 8.);
        assert!(expand_columns(&raw, 13, &[0, 4], 15.).is_none());
    }
}
