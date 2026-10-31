//! Native decorations for interactive fields. Clients send viewport state, not
//! hand-built ticks. The same Figure/Axes/Colorbar and LaTeX engine is used for
//! initial views, zoomed views, shared-axis stacks and saved figures.
use crate::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InteractiveFieldSpec {
    /// Optional paper color for the axes frame and separate panel labels.
    #[serde(default)]
    pub background: Option<[f64; 3]>,
    /// Optional sequential RGB stops shared by the native colorbar and GPU field.
    #[serde(default)]
    pub color_stops: Option<Vec<[f64; 3]>>,
    #[serde(default)]
    pub equal_aspect: bool,
    #[serde(default)]
    pub hsv_key: Option<crate::HsvColorKey>,
    /// Physical x0,y0,x1,y1, independent of texture dimensions.
    pub bounds: [f64; 4],
    pub xlabel: String,
    pub ylabel: String,
    pub colorbar_label: String,
    /// One name per panel; a stack shares x decorations automatically.
    pub panels: Vec<String>,
    /// If supplied, x coordinates are hours relative to this Unix time.
    pub utc_origin_seconds: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameRequest {
    pub width: f64,
    /// Normalized x0,y0,x1,y1 from PlotViewport; default [0,0,1,1].
    pub viewport: [f64; 4],
    pub color_limits: [f64; 2],
}
#[derive(Serialize)]
pub struct InteractiveFrame {
    /// Native hit regions for the default color-limit editor.
    pub color_regions: Vec<[f64; 4]>,
    pub color_stops: Vec<f32>,
    pub svg: String,
    pub width: f64,
    pub height: f64,
    pub panels: Vec<[f64; 4]>,
    pub panel_labels: Vec<String>,
}

pub fn nice_ticks(lo: f64, hi: f64, target: usize) -> Vec<f64> {
    if !lo.is_finite() || !hi.is_finite() || hi <= lo || target < 2 {
        return vec![];
    }
    let raw = (hi - lo) / (target - 1) as f64;
    let magnitude = 10_f64.powf(raw.log10().floor());
    let fraction = raw / magnitude;
    let mut step = [1., 2., 2.5, 5., 10.]
        .into_iter()
        .find(|x| *x >= fraction)
        .unwrap()
        * magnitude;
    // A rounded step can leave only zero on a symmetric Doppler axis.
    // Prefer a denser nice step rather than silently losing scale context.
    let minimum = target.min(3);
    while ((hi / step + 1e-8).floor() - (lo / step).ceil() + 1.).max(0.) < minimum as f64 {
        let exponent = 10_f64.powf(step.log10().floor());
        let normalized = step / exponent;
        step = [5., 2.5, 2., 1., 0.5]
            .into_iter()
            .find(|x| *x < normalized - 1e-8)
            .unwrap_or(0.5)
            * exponent;
    }
    let first = (lo / step).ceil() * step;
    (0..target + 2)
        .map(|i| first + i as f64 * step)
        .take_while(|v| *v <= hi + step * 1e-8)
        .collect()
}

fn utc_ticks(lo: f64, hi: f64, origin: f64, width: f64) -> (Vec<f64>, Vec<String>) {
    let target = (width / 110.).floor().clamp(2., 7.) as usize;
    let span = (hi - lo) * 3600.;
    let needed = span / (target - 1) as f64;
    let step = [
        1., 2., 5., 10., 15., 30., 60., 120., 300., 600., 900., 1800., 3600., 7200., 10800.,
        21600., 43200., 86400.,
    ]
    .into_iter()
    .find(|v| *v >= needed)
    .unwrap_or(86400.);
    let first = ((origin + lo * 3600.) / step).ceil() * step;
    let mut values = vec![];
    let mut labels = vec![];
    for i in 0..target + 2 {
        let unix = first + i as f64 * step;
        if unix > origin + hi * 3600. + 1e-6 {
            break;
        }
        if let Some(t) = chrono::DateTime::<chrono::Utc>::from_timestamp(unix.round() as i64, 0) {
            values.push((unix - origin) / 3600.);
            labels.push(
                t.format(if span < 120. {
                    "%H:%M:%S"
                } else if span < 86400. {
                    "%H:%M"
                } else {
                    "%m-%d %H:%M"
                })
                .to_string(),
            );
        }
    }
    (values, labels)
}

fn colorbar_ticks(lo: f64, hi: f64) -> Vec<f64> {
    // Endpoints describe the actual clipping limits, but must replace nearby
    // rounded ticks instead of colliding with them (e.g. 12.5 beside 13).
    let margin = (hi - lo) * 0.12;
    let mut ticks = vec![lo];
    ticks.extend(
        nice_ticks(lo, hi, 5)
            .into_iter()
            .filter(|v| *v > lo + margin && *v < hi - margin),
    );
    ticks.push(hi);
    ticks
}

/// Default native axis factory. Decorations cannot silently be omitted.
pub fn field_frame(
    spec: &InteractiveFieldSpec,
    request: &FrameRequest,
    cache: &Path,
) -> Result<InteractiveFrame> {
    ensure!(
        spec.bounds
            .iter()
            .chain(request.viewport.iter())
            .chain(request.color_limits.iter())
            .all(|x| x.is_finite()),
        "Nonfinite bounds"
    );
    ensure!(
        !spec.xlabel.is_empty() && !spec.ylabel.is_empty() && !spec.colorbar_label.is_empty(),
        "Interactive plots require axis and colorbar labels"
    );
    ensure!(
        (1..=12).contains(&spec.panels.len()),
        "Expected 1..12 panels"
    );
    ensure!(
        request.width >= 320. && request.width <= 2000.,
        "Invalid CSS width"
    );
    let b = request.viewport;
    ensure!(
        b[0] >= 0. && b[1] >= 0. && b[2] <= 1. && b[3] <= 1. && b[2] > b[0] && b[3] > b[1],
        "Invalid normalized viewport"
    );
    ensure!(
        spec.bounds[2] > spec.bounds[0]
            && spec.bounds[3] > spec.bounds[1]
            && request.color_limits[1] > request.color_limits[0],
        "Invalid physical limits"
    );
    let n = spec.panels.len();
    let single_hsv = n == 1 && spec.hsv_key.is_some();
    let key_below = spec.hsv_key.is_some() && request.width < 900.;
    let width = if single_hsv {
        request.width.clamp(600., 840.)
    } else if spec.hsv_key.is_some() {
        request.width.max(600.)
    } else if n == 1 && spec.equal_aspect {
        request.width.min(520.)
    } else {
        request.width
    };
    let square = if single_hsv {
        if key_below {
            60.
        } else {
            ((width * 25.4 / 96. - 82.) * 0.58).min(80.)
        }
    } else {
        0.
    };
    let height = if single_hsv {
        (square + if key_below { 69. } else { 32. }) * 96. / 25.4
    } else if n == 1 && spec.equal_aspect {
        width - 26. * 96. / 25.4
    } else if n == 1 {
        300.
    } else {
        n as f64 * 121. + 95. + if key_below { 155. } else { 0. }
    };
    let mut f = WebLayout::new(width, 16.)?.figure(height)?;
    f.latex.cache = cache.into();
    if let Some(background) = spec.background {
        f.background = background;
    }
    let x0 = spec.bounds[0] + b[0] * (spec.bounds[2] - spec.bounds[0]);
    let x1 = spec.bounds[0] + b[2] * (spec.bounds[2] - spec.bounds[0]);
    let y0 = spec.bounds[1] + b[1] * (spec.bounds[3] - spec.bounds[1]);
    let y1 = spec.bounds[1] + b[3] * (spec.bounds[3] - spec.bounds[1]);
    let mut rectangles = vec![];
    let mut panel_labels = vec![];
    for (i, name) in spec.panels.iter().enumerate() {
        let rect = if single_hsv {
            Rect::new(
                if key_below {
                    (f.width_mm - square) / 2.
                } else {
                    20.
                },
                7.,
                square,
                square,
            )
        } else if n == 1 {
            Rect::new(20., 7., f.width_mm - 50., f.height_mm - 24.)
        } else {
            Rect::new(
                20.,
                4. + i as f64 * 32.,
                f.width_mm
                    - if spec.hsv_key.is_some() && !key_below {
                        110.
                    } else {
                        50.
                    },
                30.,
            )
        };
        let mut axes = Axes::new(
            [x0, x1],
            [y0, y1],
            &spec.xlabel,
            if i == n / 2 { &spec.ylabel } else { "" },
        );
        axes.show_x_axis = i == n - 1;
        axes.yticks = nice_ticks(y0, y1, 5);
        if let Some(origin) = spec.utc_origin_seconds {
            let (ticks, labels) = utc_ticks(x0, x1, origin, rect.width * 96. / 25.4);
            axes.xticks = ticks;
            axes.xticklabels = Some(labels);
        } else {
            axes.xticks = nice_ticks(x0, x1, if request.width < 500. { 3 } else { 5 });
        }
        f.panels.push(Panel::new(rect, axes));
        rectangles.push([
            rect.x / f.width_mm,
            rect.y / f.height_mm,
            rect.width / f.width_mm,
            rect.height / f.height_mm,
        ]);
        let mut label = WebLayout::new(80., 16.)?.figure(24.)?;
        label.latex.cache = cache.into();
        label.background = f.background;
        label
            .labels
            .push(Label::new(name, label.width_mm / 2., label.height_mm / 2.));
        panel_labels.push(label.svg()?);
    }
    // A single native palette supplies both the GPU field and the colorbar.
    let palette = if let Some(stops) = &spec.color_stops {
        ColorMap::new(stops.clone())?
    } else {
        let plasma = ColorMap::plasma();
        ColorMap::new((0..32).map(|i| plasma.stops[i * 255 / 31]).collect())?
    };
    let color_stops = palette.stops.iter().flatten().map(|x| *x as f32).collect();
    f.scales.push(Scale::new(
        request.color_limits,
        Normalization::Linear,
        palette,
    )?);
    let r = f.panels[0].rect;
    let end = f.panels.last().unwrap().rect;
    let ticks = colorbar_ticks(request.color_limits[0], request.color_limits[1]);
    if let Some(key) = &spec.hsv_key {
        if key_below {
            let y = if single_hsv {
                r.y + r.height + 22.
            } else {
                end.y + end.height + 24.
            };
            let w = (f.width_mm - 50.) / 2.;
            key.add_to(
                &mut f,
                Rect::new(20., y, w, 20.),
                Rect::new(45. + w, y, w, 20.),
            )?;
        } else {
            let x = r.x + r.width + 25.;
            let w = f.width_mm - x - 7.;
            let h = if single_hsv { (square - 18.) / 2. } else { 30. };
            let top = r.y.max(7.);
            key.add_to(
                &mut f,
                Rect::new(x, top, w, h),
                Rect::new(x, top + h + 18., w, h),
            )?;
        }
    } else {
        f.colorbars.push(Colorbar::new(
            Rect::new(r.x + r.width + 3., r.y, 3., end.y + end.height - r.y),
            0,
            &spec.colorbar_label,
            ticks,
        ));
    }
    f.layout_colorbars()?;
    let svg = f.svg()?;
    let regions: Vec<Rect> = if spec.hsv_key.is_some() {
        f.panels[n..].iter().map(|p| p.rect).collect()
    } else {
        f.colorbars.iter().map(|b| b.rect).collect()
    };
    let color_regions = regions
        .iter()
        .map(|r| {
            [
                r.x / f.width_mm,
                r.y / f.height_mm,
                r.width / f.width_mm,
                r.height / f.height_mm,
            ]
        })
        .collect();
    Ok(InteractiveFrame {
        svg,
        width,
        height,
        panels: rectangles,
        panel_labels,
        color_stops,
        color_regions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ticks_are_nice_and_bounded() {
        assert_eq!(nice_ticks(3., 6., 4), vec![3., 4., 5., 6.]);
        assert_eq!(nice_ticks(50., 120., 4), vec![50., 75., 100.]);
    }
    #[test]
    fn doppler_ticks_never_collapse_to_zero() {
        for target in [3, 4, 5] {
            assert_eq!(
                nice_ticks(-99.66504587765958, 99.66504587765958, target),
                vec![-50., 0., 50.]
            );
        }
    }
    #[test]
    fn zoomed_numeric_axes_keep_three_ticks() {
        for (lo, hi) in [(-0.09967, 0.09967), (31., 39.), (-99., -51.), (0.01, 0.09)] {
            let ticks = nice_ticks(lo, hi, 3);
            assert!(ticks.len() >= 3, "{lo}..{hi}: {ticks:?}");
            assert!(ticks.iter().all(|v| *v >= lo && *v <= hi));
        }
    }
    #[test]
    fn utc_ticks_follow_zoom() {
        let (t, l) = utc_ticks(0., 1., 0., 500.);
        assert_eq!(l, vec!["00:00", "00:30", "01:00"]);
        assert_eq!(t, vec![0., 0.5, 1.]);
    }
    #[test]
    fn colorbar_endpoints_replace_nearby_rounded_ticks() {
        assert_eq!(colorbar_ticks(3., 13.), vec![3., 5., 7.5, 10., 13.]);
        for (lo, hi) in [(3., 6.), (3., 13.), (3., 30.), (-30., 0.), (3.1, 13.1)] {
            let ticks = colorbar_ticks(lo, hi);
            assert_eq!(ticks.first(), Some(&lo));
            assert_eq!(ticks.last(), Some(&hi));
            assert!(ticks.windows(2).all(|v| v[1] - v[0] >= (hi - lo) * 0.12));
        }
    }
}
