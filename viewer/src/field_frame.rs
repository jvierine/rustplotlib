//! Browser field decorations: Rust owns layout, ticks, palette and HSV keys.
//! No process, filesystem, TeX installation or web-server round trip is needed.
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
struct Spec {
    bounds: [f64; 4],
    xlabel: String,
    ylabel: String,
    colorbar_label: String,
    panels: Vec<String>,
    utc_origin_seconds: Option<f64>,
    #[serde(default)]
    equal_aspect: bool,
    #[serde(default)]
    color_stops: Option<Vec<[f64; 3]>>,
    #[serde(default)]
    hsv_key: Option<HsvKey>,
}
#[derive(Deserialize)]
struct Request {
    width: f64,
    viewport: [f64; 4],
    color_limits: [f64; 2],
}
#[derive(Deserialize)]
struct HsvKey {
    velocity_limit: f64,
    width_limit: f64,
}
#[derive(Serialize)]
struct Frame {
    svg: String,
    width: f64,
    height: f64,
    panels: Vec<[f64; 4]>,
    panel_labels: Vec<String>,
    color_stops: Vec<f64>,
    color_regions: Vec<[f64; 4]>,
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn text(s: &mut String, x: f64, y: f64, label: &str, anchor: &str, size: f64) {
    let label = label.replace("m s$^{-1}$", "m/s");
    write!(
        s,
        "<text x='{x:.2}' y='{y:.2}' text-anchor='{anchor}' font-size='{size}'>{}</text>",
        escape(&label)
    )
    .unwrap();
}
fn line(s: &mut String, x: f64, y: f64, u: f64, v: f64, color: &str) {
    write!(
        s,
        "<path d='M{x:.2},{y:.2}L{u:.2},{v:.2}' fill='none' stroke='{color}' stroke-width='.7'/>"
    )
    .unwrap();
}
fn number(v: f64, span: f64) -> String {
    if v.abs() < span.abs() * 1e-9 {
        return "0".into();
    }
    if v.abs() >= 1e5 || (v != 0. && v.abs() < 0.001) {
        return format!("{v:.1e}");
    }
    let places = (2. - span.abs().max(1e-12).log10().floor()).clamp(0., 7.) as usize;
    let t = format!("{v:.places$}");
    if t.contains('.') {
        t.trim_end_matches('0').trim_end_matches('.').into()
    } else {
        t
    }
}
fn ticks(lo: f64, hi: f64, target: usize) -> Vec<f64> {
    let raw = (hi - lo) / (target.max(3) - 1) as f64;
    let scale = 10_f64.powf(raw.log10().floor());
    let mut step = [1., 2., 2.5, 5., 10.]
        .into_iter()
        .find(|n| *n * scale >= raw)
        .unwrap()
        * scale;
    while ((hi / step).floor() - (lo / step).ceil() + 1.) < 3. {
        step /= 2.;
    }
    let first = (lo / step).ceil() * step;
    (0..32)
        .map(|i| first + i as f64 * step)
        .take_while(|v| *v <= hi + step * 1e-8)
        .collect()
}
fn xticks(lo: f64, hi: f64, origin: Option<f64>, width: f64) -> Vec<(f64, String)> {
    if let Some(origin) = origin {
        let span = (hi - lo) * 3600.;
        let target = (width / 105.).floor().clamp(3., 8.);
        let needed = span / (target - 1.);
        let step = [
            1., 2., 5., 10., 15., 30., 60., 120., 300., 600., 900., 1800., 3600., 7200., 10800.,
            21600., 43200., 86400.,
        ]
        .into_iter()
        .find(|n| *n >= needed)
        .unwrap_or(86400.);
        let first = ((origin + lo * 3600.) / step).ceil() * step;
        (0..32)
            .map(|i| first + i as f64 * step)
            .take_while(|t| *t <= origin + hi * 3600. + 1e-6)
            .filter_map(|t| {
                chrono::DateTime::<chrono::Utc>::from_timestamp(t.round() as i64, 0).map(|date| {
                    (
                        (t - origin) / 3600.,
                        date.format(if span < 120. {
                            "%H:%M:%S"
                        } else if span <= 86400. {
                            "%H:%M"
                        } else {
                            "%Y-%m-%d"
                        })
                        .to_string(),
                    )
                })
            })
            .collect()
    } else {
        ticks(lo, hi, if width < 450. { 3 } else { 5 })
            .into_iter()
            .map(|v| (v, number(v, hi - lo)))
            .collect()
    }
}
fn rgb(v: [f64; 3]) -> String {
    format!(
        "rgb({},{},{})",
        (255. * v[0]).round() as u8,
        (255. * v[1]).round() as u8,
        (255. * v[2]).round() as u8
    )
}
// Same HSV mapping as FieldView and the native HsvColorKey.
fn hsv(velocity: f64, width: f64, value: f64) -> [f64; 3] {
    let h = (2. / 3.) * (1. - velocity.clamp(0., 1.));
    let sat = 1. - 0.85 * width.clamp(0., 1.);
    std::array::from_fn(|i| {
        let p = (((h + [0., 2. / 3., 1. / 3.][i]).fract() * 6. - 3.).abs() - 1.).clamp(0., 1.);
        value * (1. - sat + sat * p)
    })
}
fn axes(s: &mut String, r: [f64; 4], b: [f64; 4], origin: Option<f64>, bottom: bool) {
    let [x, y, w, h] = r;
    let [x0, y0, x1, y1] = b;
    for v in ticks(y0, y1, 4) {
        let q = y + h * (1. - (v - y0) / (y1 - y0));
        line(s, x, q, x + w, q, "#d4dce1");
        text(s, x - 7., q + 4., &number(v, y1 - y0), "end", 13.);
    }
    for (v, label) in xticks(x0, x1, origin, w) {
        let q = x + w * (v - x0) / (x1 - x0);
        line(s, q, y, q, y + h, "#d4dce1");
        if bottom {
            text(s, q, y + h + 19., &label, "middle", 13.);
        }
    }
    line(s, x, y, x, y + h, "#485d6c");
    line(s, x, y + h, x + w, y + h, "#485d6c");
}
fn rotated(s: &mut String, x: f64, y: f64, label: &str) {
    write!(s, "<g transform='translate({x:.2},{y:.2}) rotate(-90)'>").unwrap();
    text(s, 0., 0., label, "middle", 15.);
    s.push_str("</g>");
}
fn make_frame(spec: &Spec, req: &Request) -> Result<Frame, String> {
    let b = req.viewport;
    let lim = req.color_limits;
    if !spec
        .bounds
        .iter()
        .chain(b.iter())
        .chain(lim.iter())
        .all(|v| v.is_finite())
        || !req.width.is_finite()
        || req.width < 280.
        || req.width > 2400.
        || !(1..=12).contains(&spec.panels.len())
        || spec.bounds[2] <= spec.bounds[0]
        || spec.bounds[3] <= spec.bounds[1]
        || b[0] < 0.
        || b[1] < 0.
        || b[2] > 1.
        || b[3] > 1.
        || b[2] <= b[0]
        || b[3] <= b[1]
        || lim[1] <= lim[0]
    {
        return Err("Invalid interactive field bounds".into());
    }
    if spec.xlabel.trim().is_empty()
        || spec.ylabel.trim().is_empty()
        || spec.colorbar_label.trim().is_empty()
    {
        return Err("Interactive plots require axis and colorbar labels".into());
    }
    if let Some(k) = &spec.hsv_key {
        if !k.velocity_limit.is_finite()
            || !k.width_limit.is_finite()
            || k.velocity_limit <= 0.
            || k.width_limit <= 0.
        {
            return Err("Invalid HSV key limits".into());
        }
    }
    let palette = spec.color_stops.clone().unwrap_or_else(|| {
        vec![
            [0.05, 0.03, 0.53],
            [0.49, 0.01, 0.66],
            [0.8, 0.28, 0.47],
            [0.97, 0.58, 0.25],
            [0.94, 0.98, 0.13],
        ]
    });
    if palette.len() < 2
        || palette
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
    {
        return Err("Invalid color palette".into());
    }
    let n = spec.panels.len();
    let width = if spec.equal_aspect {
        req.width.min(650.)
    } else {
        req.width
    };
    let pw = (width - 166.).max(110.);
    let ph = if spec.equal_aspect {
        pw
    } else if n == 1 {
        205.
    } else {
        105.
    };
    let gap = if n == 1 { 0. } else { 7. };
    let top = 14.;
    let left = 71.;
    let end = top + n as f64 * ph + (n - 1) as f64 * gap;
    let key_h = if spec.hsv_key.is_some() { 150. } else { 0. };
    let height = end + 62. + key_h;
    let bounds = [
        spec.bounds[0] + b[0] * (spec.bounds[2] - spec.bounds[0]),
        spec.bounds[1] + b[1] * (spec.bounds[3] - spec.bounds[1]),
        spec.bounds[0] + b[2] * (spec.bounds[2] - spec.bounds[0]),
        spec.bounds[1] + b[3] * (spec.bounds[3] - spec.bounds[1]),
    ];
    let mut svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='{width}' height='{height}' viewBox='0 0 {width} {height}'><rect width='100%' height='100%' fill='white'/><g fill='#172b38' font-family='Palatino,Georgia,serif'>"
    );
    let mut panels = vec![];
    let mut panel_labels = vec![];
    for (i, name) in spec.panels.iter().enumerate() {
        let y = top + i as f64 * (ph + gap);
        let rect = [left, y, pw, ph];
        axes(&mut svg, rect, bounds, spec.utc_origin_seconds, i == n - 1);
        panels.push([left / width, y / height, pw / width, ph / height]);
        let mut label="<svg xmlns='http://www.w3.org/2000/svg' width='80' height='24'><rect width='80' height='24' fill='white' fill-opacity='.85'/><g font-family='Palatino,Georgia,serif' fill='#172b38'>".to_string();
        text(&mut label, 6., 17., name, "start", 14.);
        label.push_str("</g></svg>");
        panel_labels.push(label);
    }
    text(
        &mut svg,
        left + pw / 2.,
        end + 44.,
        &spec.xlabel,
        "middle",
        15.,
    );
    rotated(&mut svg, 18., (top + end) / 2., &spec.ylabel);
    let mut regions = vec![];
    if let Some(k) = &spec.hsv_key {
        let ky = end + 81.;
        let kw = ((width - 180.) / 2.).max(95.);
        let kh = 65.;
        for kind in 0..2 {
            let kx = left + kind as f64 * (kw + 70.);
            for j in 0..64 {
                let v = j as f64 / 63.;
                let a = if kind == 0 {
                    hsv(v, 0., 1.)
                } else {
                    hsv(v, 1., 1.)
                };
                let z = if kind == 0 { [0.; 3] } else { hsv(v, 0., 1.) };
                write!(svg,"<defs><linearGradient id='k{kind}_{j}' x1='0' y1='0' x2='0' y2='1'><stop stop-color='{}'/><stop offset='1' stop-color='{}'/></linearGradient></defs><rect x='{:.2}' y='{ky}' width='{:.2}' height='{kh}' fill='url(#k{kind}_{j})'/>",rgb(a),rgb(z),kx+j as f64*kw/64.,kw/64.+0.1).unwrap();
            }
            text(
                &mut svg,
                kx + kw / 2.,
                ky - 10.,
                if kind == 0 {
                    "Zero spectral width"
                } else {
                    "Maximum brightness"
                },
                "middle",
                12.,
            );
            for f in [0., 0.5, 1.] {
                text(
                    &mut svg,
                    kx + kw * f,
                    ky + kh + 17.,
                    &number((2. * f - 1.) * k.velocity_limit, 2. * k.velocity_limit),
                    "middle",
                    12.,
                );
                let v = if kind == 0 {
                    lim[0] + f * (lim[1] - lim[0])
                } else {
                    f * k.width_limit
                };
                text(
                    &mut svg,
                    kx - 5.,
                    ky + kh * (1. - f) + 4.,
                    &number(
                        v,
                        if kind == 0 {
                            lim[1] - lim[0]
                        } else {
                            k.width_limit
                        },
                    ),
                    "end",
                    12.,
                );
            }
            text(
                &mut svg,
                kx + kw / 2.,
                ky + kh + 35.,
                "Velocity (m/s)",
                "middle",
                13.,
            );
            rotated(
                &mut svg,
                kx - 36.,
                ky + kh / 2.,
                if kind == 0 {
                    &spec.colorbar_label
                } else {
                    "Width (m/s)"
                },
            );
            regions.push([kx / width, ky / height, kw / width, kh / height]);
        }
    } else {
        let x = left + pw + 13.;
        let h = end - top;
        svg.push_str("<defs><linearGradient id='scalar' x1='0' y1='1' x2='0' y2='0'>");
        for (i, c) in palette.iter().enumerate() {
            write!(
                svg,
                "<stop offset='{}' stop-color='{}'/>",
                i as f64 / (palette.len() - 1) as f64,
                rgb(*c)
            )
            .unwrap();
        }
        write!(svg,"</linearGradient></defs><rect x='{x}' y='{top}' width='12' height='{h}' fill='url(#scalar)'/>").unwrap();
        let mut ct = vec![lim[0]];
        ct.extend(ticks(lim[0], lim[1], 5).into_iter().filter(|v| {
            *v > lim[0] + 0.12 * (lim[1] - lim[0]) && *v < lim[1] - 0.12 * (lim[1] - lim[0])
        }));
        ct.push(lim[1]);
        for v in ct {
            text(
                &mut svg,
                x + 18.,
                top + h * (1. - (v - lim[0]) / (lim[1] - lim[0])) + 4.,
                &number(v, lim[1] - lim[0]),
                "start",
                13.,
            );
        }
        rotated(
            &mut svg,
            width - 10.,
            (top + end) / 2.,
            &spec.colorbar_label,
        );
        regions.push([x / width, top / height, 12. / width, h / height]);
    }
    svg.push_str("</g></svg>");
    Ok(Frame {
        svg,
        width,
        height,
        panels,
        panel_labels,
        color_stops: palette.into_iter().flatten().collect(),
        color_regions: regions,
    })
}
#[wasm_bindgen]
pub fn interactive_field_frame(spec_json: &str, request_json: &str) -> Result<String, JsValue> {
    let result = (|| {
        let spec: Spec = serde_json::from_str(spec_json).map_err(|e| e.to_string())?;
        let req: Request = serde_json::from_str(request_json).map_err(|e| e.to_string())?;
        serde_json::to_string(&make_frame(&spec, &req)?).map_err(|e| e.to_string())
    })();
    result.map_err(|e| JsValue::from_str(&e))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn axes_and_colorbar_are_local_and_share_palette() {
        let s:Spec=serde_json::from_str(r#"{"bounds":[0,50,24,120],"xlabel":"UTC","ylabel":"Altitude","colorbar_label":"SNR","panels":["Zenith","North","East","South","West"],"utc_origin_seconds":0}"#).unwrap();
        let r = Request {
            width: 1000.,
            viewport: [0., 0., 1., 1.],
            color_limits: [0., 13.],
        };
        let f = make_frame(&s, &r).unwrap();
        assert_eq!(f.panels.len(), 5);
        assert_eq!(f.panel_labels.len(), 5);
        assert_eq!(f.color_regions.len(), 1);
        assert!(f.svg.contains("SNR"));
        assert!(f.svg.contains("00:00"));
        assert!(!f.svg.contains("http://localhost"));
        assert!(ticks(-99.67, 99.67, 3).len() >= 3);
    }
    #[test]
    fn hsv_slices_match_field_mapping() {
        for (v, expected) in [(0., [0., 0., 1.]), (0.5, [0., 1., 0.]), (1., [1., 0., 0.])] {
            for (a, b) in hsv(v, 0., 1.).iter().zip(expected) {
                assert!((a - b).abs() < 1e-12);
            }
        }
        assert!((hsv(1., 1., 1.)[1] - 0.85).abs() < 1e-12);
    }
    #[test]
    fn rejects_invalid_viewports() {
        let s:Spec=serde_json::from_str(r#"{"bounds":[0,0,1,1],"xlabel":"x","ylabel":"y","colorbar_label":"z","panels":[""],"utc_origin_seconds":null}"#).unwrap();
        assert!(
            make_frame(
                &s,
                &Request {
                    width: 1000.,
                    viewport: [0., 0., 0., 1.],
                    color_limits: [0., 1.]
                }
            )
            .is_err()
        );
    }
}
