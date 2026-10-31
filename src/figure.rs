use crate::{Latex, Scale, latex::Glyph};
use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Write as _, io::Cursor, path::Path};
const MM_PER_PT: f64 = 25.4 / 72.27;
const PHI: f64 = 1.618033988749895;

#[derive(Clone, Copy, Debug)]
pub enum IntendedUse {
    ArticleColumn,
    ArticleWide,
    Mobile,
    DesktopHalf,
    Custom { width_mm: f64, font_pt: f64 },
}
impl IntendedUse {
    fn dimensions(self) -> (f64, f64) {
        match self {
            Self::ArticleColumn => (85.0, 10.0),
            Self::ArticleWide => (178.0, 10.0),
            Self::Mobile => (103.2, 13.0),
            Self::DesktopHalf => (190.5, 12.0),
            Self::Custom { width_mm, font_pt } => (width_mm, font_pt),
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    fn valid(self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|x| x.is_finite())
            && self.width > 0.0
            && self.height > 0.0
    }
}
#[derive(Clone, Debug)]
pub struct Label {
    pub latex: String,
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
    pub color: String,
    pub background: Option<String>,
    pub padding_mm: f64,
    pub leader: Option<[f64; 2]>,
}
impl Label {
    pub fn new(latex: impl Into<String>, x: f64, y: f64) -> Self {
        Self {
            latex: latex.into(),
            x,
            y,
            rotation: 0.0,
            color: "#171717".into(),
            background: None,
            padding_mm: 0.7,
            leader: None,
        }
    }
}
/// A top x axis with labels in another coordinate system.
/// `to_primary` maps secondary tick values onto the panel's primary x scale.
/// Tick positions are shared by every output backend; labels use LaTeX.
#[derive(Clone, Debug)]
pub struct SecondaryAxis {
    pub label: String,
    /// (primary coordinate, secondary label value).
    pub ticks: Vec<(f64, f64)>,
    pub tick_labels: Option<Vec<String>>,
}
impl SecondaryAxis {
    pub fn new(
        label: impl Into<String>,
        values: Vec<f64>,
        to_primary: impl Fn(f64) -> f64,
    ) -> Result<Self> {
        let ticks: Vec<_> = values.into_iter().map(|v| (to_primary(v), v)).collect();
        ensure!(
            ticks.iter().all(|(x, v)| x.is_finite() && v.is_finite()),
            "Secondary axis transform must yield finite ticks"
        );
        Ok(Self {
            label: label.into(),
            ticks,
            tick_labels: None,
        })
    }
}
#[derive(Clone, Debug)]
pub struct Axes {
    pub xlim: [f64; 2],
    pub ylim: [f64; 2],
    pub xlabel: String,
    pub ylabel: String,
    pub xticks: Vec<f64>,
    pub yticks: Vec<f64>,
    /// Optional LaTeX text for each tick, e.g. calendar times. Positions stay numeric.
    pub xticklabels: Option<Vec<String>>,
    pub yticklabels: Option<Vec<String>>,
    pub secondary_x: Option<SecondaryAxis>,
    pub secondary_y: Option<SecondaryAxis>,
    pub show_x_axis: bool,
    pub show_y_axis: bool,
    pub xscale: crate::AxisScale,
    pub yscale: crate::AxisScale,
    pub xminor_ticks: Vec<f64>,
    pub yminor_ticks: Vec<f64>,
}
impl Axes {
    pub fn new(
        xlim: [f64; 2],
        ylim: [f64; 2],
        xlabel: impl Into<String>,
        ylabel: impl Into<String>,
    ) -> Self {
        Self {
            xlim,
            ylim,
            xlabel: xlabel.into(),
            ylabel: ylabel.into(),
            xticks: ticks(xlim, 5),
            yticks: ticks(ylim, 4),
            xticklabels: None,
            yticklabels: None,
            secondary_x: None,
            secondary_y: None,
            show_x_axis: true,
            show_y_axis: true,
            xscale: crate::AxisScale::Linear,
            yscale: crate::AxisScale::Linear,
            xminor_ticks: vec![],
            yminor_ticks: vec![],
        }
    }
    pub fn set_xscale(&mut self, scale: crate::AxisScale) -> Result<()> {
        scale.validate(self.xlim)?;
        self.xscale = scale;
        (self.xticks, self.xminor_ticks) = scale.ticks(self.xlim);
        self.xticklabels = if scale == crate::AxisScale::Log10 {
            Some(
                self.xticks
                    .iter()
                    .map(|v| format!("$10^{{{:.0}}}$", v.log10()))
                    .collect(),
            )
        } else {
            None
        };
        Ok(())
    }
    pub fn set_yscale(&mut self, scale: crate::AxisScale) -> Result<()> {
        scale.validate(self.ylim)?;
        self.yscale = scale;
        (self.yticks, self.yminor_ticks) = scale.ticks(self.ylim);
        self.yticklabels = if scale == crate::AxisScale::Log10 {
            Some(
                self.yticks
                    .iter()
                    .map(|v| format!("$10^{{{:.0}}}$", v.log10()))
                    .collect(),
            )
        } else {
            None
        };
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct Line {
    pub points: Vec<[f64; 2]>,
    pub color: String,
    pub width_pt: f64,
    pub dash: Option<String>,
    /// Per-curve opacity. Overlapping ensemble members accumulate naturally.
    pub opacity: f64,
    /// Optional closed-path fill, useful for uncertainty bands and vector markers.
    pub fill: Option<String>,
    pub fill_opacity: f64,
    /// Protect data from automatic legend placement; contextual grids opt out.
    pub protect_from_legend: bool,
}
impl Line {
    pub fn new(points: Vec<[f64; 2]>, color: impl Into<String>) -> Self {
        Self {
            points,
            color: color.into(),
            width_pt: 1.1,
            dash: None,
            opacity: 1.,
            fill: None,
            fill_opacity: 1.,
            protect_from_legend: true,
        }
    }
}

fn write_line(svg: &mut String, panel: &Panel, line: &Line) -> Result<()> {
    let mut path = String::new();
    let mut prior = false;
    for point in &line.points {
        if !point.iter().all(|v| v.is_finite()) {
            prior = false;
            continue;
        }
        let [x, y] = panel.xy(*point);
        if !x.is_finite() || !y.is_finite() {
            prior = false;
            continue;
        }
        write!(path, "{} {:.6} {:.6} ", if prior { "L" } else { "M" }, x, y)?;
        prior = true;
    }
    write!(
        svg,
        "<path d=\"{path}\" fill=\"{}\" fill-opacity=\"{}\" stroke=\"{}\" stroke-width=\"{}\" stroke-opacity=\"{}\" {} stroke-linejoin=\"round\"/>",
        line.fill.as_deref().unwrap_or("none"),
        line.fill_opacity,
        line.color,
        line.width_pt * MM_PER_PT,
        line.opacity,
        line.dash
            .as_ref()
            .map(|d| format!("stroke-dasharray=\"{d}\""))
            .unwrap_or_default()
    )?;
    Ok(())
}
/// Rectilinear cell data. `values[j * nx + i]`; y increases upwards.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Field {
    pub x_edges: Vec<f64>,
    pub y_edges: Vec<f64>,
    pub values: Vec<f64>,
    pub scale: usize,
    #[serde(default)]
    pub interpolate: bool,
}
impl Field {
    pub fn new(
        x_edges: Vec<f64>,
        y_edges: Vec<f64>,
        values: Vec<f64>,
        scale: usize,
    ) -> Result<Self> {
        ensure!(
            increasing(&x_edges) && increasing(&y_edges),
            "Cell edges must be finite and strictly increasing"
        );
        ensure!(
            values.len() == (x_edges.len() - 1) * (y_edges.len() - 1),
            "pcolormesh shape mismatch"
        );
        Ok(Self {
            x_edges,
            y_edges,
            values,
            scale,
            interpolate: false,
        })
    }
    pub fn sample(&self, x: f64, y: f64) -> f64 {
        let Some(i) = bin(&self.x_edges, x) else {
            return f64::NAN;
        };
        let Some(j) = bin(&self.y_edges, y) else {
            return f64::NAN;
        };
        let nx = self.x_edges.len() - 1;
        let base = self.values[j * nx + i];
        if !self.interpolate || !base.is_finite() {
            return base;
        }
        let bracket = |e: &[f64], k: usize, q: f64| {
            let center = |i: usize| (e[i] + e[i + 1]) * 0.5;
            let a = if q < center(k) {
                k.saturating_sub(1)
            } else {
                k
            };
            let b = (a + 1).min(e.len() - 2);
            let t = if a == b {
                0.0
            } else {
                ((q - center(a)) / (center(b) - center(a))).clamp(0.0, 1.0)
            };
            (a, b, t)
        };
        let (a, b, u) = bracket(&self.x_edges, i, x);
        let (c, d, v) = bracket(&self.y_edges, j, y);
        let q = [
            self.values[c * nx + a],
            self.values[c * nx + b],
            self.values[d * nx + a],
            self.values[d * nx + b],
        ];
        // Do not interpolate across a missing neighbour or fill masked cells.
        if q.iter().any(|q| !q.is_finite()) {
            return base;
        }
        (1.0 - v) * ((1.0 - u) * q[0] + u * q[1]) + v * ((1.0 - u) * q[2] + u * q[3])
    }
}
/// Curvilinear quadrilaterals, with (nx+1)*(ny+1) physical corner positions.
#[derive(Clone, Debug)]
pub struct QuadMesh {
    pub nx: usize,
    pub ny: usize,
    pub corners: Vec<[f64; 2]>,
    pub values: Vec<f64>,
    pub scale: usize,
}
impl QuadMesh {
    pub fn new(
        nx: usize,
        ny: usize,
        corners: Vec<[f64; 2]>,
        values: Vec<f64>,
        scale: usize,
    ) -> Result<Self> {
        ensure!(
            nx > 0 && ny > 0 && corners.len() == (nx + 1) * (ny + 1) && values.len() == nx * ny,
            "Curvilinear mesh shape mismatch"
        );
        ensure!(
            corners.iter().flatten().all(|x| x.is_finite()),
            "Nonfinite cell geometry"
        );
        Ok(Self {
            nx,
            ny,
            corners,
            values,
            scale,
        })
    }
}
/// Nodal values on a rectilinear grid, with piecewise-linear triangulation.
#[derive(Clone, Debug)]
pub struct Contour {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub values: Vec<f64>,
    pub levels: Vec<f64>,
    pub scale: usize,
    pub filled: bool,
    pub width_pt: f64,
}
impl Contour {
    pub fn new(
        x: Vec<f64>,
        y: Vec<f64>,
        values: Vec<f64>,
        levels: Vec<f64>,
        scale: usize,
        filled: bool,
    ) -> Result<Self> {
        ensure!(
            increasing(&x) && increasing(&y) && values.len() == x.len() * y.len(),
            "Contour grid shape mismatch"
        );
        ensure!(
            increasing(&levels),
            "Contour levels must be finite and strictly increasing"
        );
        Ok(Self {
            x,
            y,
            values,
            levels,
            scale,
            filled,
            width_pt: 0.7,
        })
    }
    /// NaN at any cell corner masks the entire interpolating cell.
    pub fn sample(&self, x: f64, y: f64) -> f64 {
        let Some(i) = bin(&self.x, x) else {
            return f64::NAN;
        };
        let Some(j) = bin(&self.y, y) else {
            return f64::NAN;
        };
        let nx = self.x.len();
        let v = [
            self.values[j * nx + i],
            self.values[j * nx + i + 1],
            self.values[(j + 1) * nx + i + 1],
            self.values[(j + 1) * nx + i],
        ];
        if !v.iter().all(|v| v.is_finite()) {
            return f64::NAN;
        };
        let a = (x - self.x[i]) / (self.x[i + 1] - self.x[i]);
        let b = (y - self.y[j]) / (self.y[j + 1] - self.y[j]);
        if b <= a {
            v[0] * (1.0 - a) + v[1] * (a - b) + v[2] * b
        } else {
            v[0] * (1.0 - b) + v[2] * a + v[3] * (b - a)
        }
    }
    pub fn segments(&self, level: f64) -> Vec<[[f64; 2]; 2]> {
        let mut out = vec![];
        let nx = self.x.len();
        for j in 0..self.y.len() - 1 {
            for i in 0..nx - 1 {
                let v = [
                    [self.x[i], self.y[j], self.values[j * nx + i]],
                    [self.x[i + 1], self.y[j], self.values[j * nx + i + 1]],
                    [
                        self.x[i + 1],
                        self.y[j + 1],
                        self.values[(j + 1) * nx + i + 1],
                    ],
                    [self.x[i], self.y[j + 1], self.values[(j + 1) * nx + i]],
                ];
                if !v.iter().all(|p| p[2].is_finite()) {
                    continue;
                }
                for t in [[v[0], v[1], v[2]], [v[0], v[2], v[3]]] {
                    let mut points = vec![];
                    for e in 0..3 {
                        let (a, b) = (t[e], t[(e + 1) % 3]);
                        if (a[2] < level) != (b[2] < level) {
                            let w = (level - a[2]) / (b[2] - a[2]);
                            points.push([a[0] + w * (b[0] - a[0]), a[1] + w * (b[1] - a[1])]);
                        }
                    }
                    if points.len() == 2 {
                        out.push([points[0], points[1]])
                    }
                }
            }
        }
        out
    }

    /// Join the native contour segments into connected paths, suitable for
    /// projection and tangent-following labels. Masked cells remain gaps.
    pub fn paths(&self, level: f64) -> Vec<Vec<[f64; 2]>> {
        let mut segments = self.segments(level);
        let mut paths = Vec::new();
        let near =
            |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9;
        while let Some(segment) = segments.pop() {
            let mut path = std::collections::VecDeque::from(segment);
            loop {
                let head = *path.front().unwrap();
                let tail = *path.back().unwrap();
                let found = segments.iter().enumerate().find_map(|(i, s)| {
                    if near(s[0], tail) {
                        Some((i, false, s[1]))
                    } else if near(s[1], tail) {
                        Some((i, false, s[0]))
                    } else if near(s[0], head) {
                        Some((i, true, s[1]))
                    } else if near(s[1], head) {
                        Some((i, true, s[0]))
                    } else {
                        None
                    }
                });
                let Some((i, front, point)) = found else {
                    break;
                };
                segments.swap_remove(i);
                if front {
                    path.push_front(point);
                } else {
                    path.push_back(point);
                }
            }
            paths.push(path.into_iter().collect());
        }
        paths
    }
}
#[derive(Clone, Debug)]
pub enum Layer {
    /// An RGBA image fitted to the panel, with native vector axes and LaTeX labels.
    Image(image::RgbaImage),
    Line(Line),
    Pcolormesh(Field),
    QuadMesh(QuadMesh),
    /// Independent polygons, for irregular cells such as projected HEALPix data.
    PolygonMesh(PolygonMesh),
    Surface(crate::Surface),
    Contour(Contour),
}

/// Independently sampled polygon cells. Missing values leave transparent holes.
#[derive(Clone, Debug)]
pub struct PolygonMesh {
    pub polygons: Vec<Vec<[f64; 2]>>,
    pub values: Vec<f64>,
    pub scale: usize,
}
impl PolygonMesh {
    pub fn new(polygons: Vec<Vec<[f64; 2]>>, values: Vec<f64>, scale: usize) -> Result<Self> {
        let mesh = Self {
            polygons,
            values,
            scale,
        };
        mesh.validate()?;
        Ok(mesh)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            self.polygons.len() == self.values.len(),
            "Polygon/value count mismatch"
        );
        ensure!(
            self.polygons
                .iter()
                .all(|p| p.len() >= 3 && p.iter().flatten().all(|v| v.is_finite())),
            "Polygons need at least three finite vertices"
        );
        Ok(())
    }
}

/// A data-anchored connection between panels, routed through figure coordinates
/// (millimetres from the top-left). Routes can avoid axes and colour bars.
#[derive(Clone, Debug)]
pub struct Connector {
    pub from_panel: usize,
    pub from: [f64; 2],
    pub to_panel: usize,
    pub to: [f64; 2],
    pub via: Vec<[f64; 2]>,
    pub color: String,
    pub width_pt: f64,
}
#[derive(Clone, Debug)]
pub struct Panel {
    pub rect: Rect,
    pub axes: Option<Axes>,
    pub axes_visible: bool,
    pub layers: Vec<Layer>,
    pub labels: Vec<Label>,
    pub legend: Option<crate::Legend>,
}
impl Panel {
    /// Place a body-sized LaTeX label using data coordinates. Set panel limits
    /// and rectangle first. The returned label exposes color and rotation.
    pub fn label_at(&mut self, latex: impl Into<String>, point: [f64; 2]) -> &mut Label {
        let [x, y] = self.xy(point);
        self.labels.push(Label::new(latex, x, y));
        self.labels.last_mut().unwrap()
    }
    /// Replace a contiguous group of line layers by one transparent image.
    /// Useful for dense ensembles in small PDFs; other layers remain vector.
    /// Layer indices after the replaced group shift. Call after panel sizing.
    #[cfg(feature = "export")]
    pub fn rasterize_lines(&mut self, range: std::ops::Range<usize>, dpi: f64) -> Result<()> {
        ensure!(
            range.start < range.end && range.end <= self.layers.len(),
            "Invalid line layer range"
        );
        ensure!(
            dpi.is_finite() && dpi > 0. && self.rect.valid(),
            "Invalid rasterization dimensions"
        );
        let (xx, yy) = self.limits();
        ensure!(
            xx.iter().chain(&yy).all(|v| v.is_finite()) && xx[0] < xx[1] && yy[0] < yy[1],
            "Invalid rasterization axes"
        );
        let w = (self.rect.width * dpi / 25.4).ceil() as u32;
        let h = (self.rect.height * dpi / 25.4).ceil() as u32;
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"{} {} {} {}\">",
            self.rect.x, self.rect.y, self.rect.width, self.rect.height
        );
        for layer in &self.layers[range.clone()] {
            let Layer::Line(line) = layer else {
                anyhow::bail!("rasterize_lines requires only line layers");
            };
            ensure!(
                line.opacity.is_finite()
                    && (0. ..=1.).contains(&line.opacity)
                    && line.width_pt.is_finite()
                    && line.width_pt >= 0.
                    && line.fill_opacity.is_finite()
                    && (0. ..=1.).contains(&line.fill_opacity),
                "Invalid rasterized line style"
            );
            write_line(&mut svg, self, line)?;
        }
        svg.push_str("</svg>");
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default())?;
        let mut pix = resvg::tiny_skia::Pixmap::new(w, h).context("Invalid raster dimensions")?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pix.as_mut(),
        );
        // tiny-skia stores premultiplied channels; PNG expects straight RGBA.
        let rgba: Vec<u8> = pix
            .pixels()
            .iter()
            .flat_map(|p| {
                let p = p.demultiply();
                [p.red(), p.green(), p.blue(), p.alpha()]
            })
            .collect();
        let image = image::RgbaImage::from_raw(w, h, rgba).context("Invalid raster image")?;
        self.layers.splice(range, [Layer::Image(image)]);
        Ok(())
    }
    pub fn pcolormesh(
        &mut self,
        x_edges: Vec<f64>,
        y_edges: Vec<f64>,
        values: Vec<f64>,
        scale: usize,
    ) -> Result<&mut Self> {
        self.layers.push(Layer::Pcolormesh(Field::new(
            x_edges, y_edges, values, scale,
        )?));
        Ok(self)
    }
    pub fn contour(
        &mut self,
        x: Vec<f64>,
        y: Vec<f64>,
        values: Vec<f64>,
        levels: Vec<f64>,
        scale: usize,
    ) -> Result<&mut Self> {
        self.layers.push(Layer::Contour(Contour::new(
            x, y, values, levels, scale, false,
        )?));
        Ok(self)
    }
    pub fn contourf(
        &mut self,
        x: Vec<f64>,
        y: Vec<f64>,
        values: Vec<f64>,
        levels: Vec<f64>,
        scale: usize,
    ) -> Result<&mut Self> {
        self.layers.push(Layer::Contour(Contour::new(
            x, y, values, levels, scale, true,
        )?));
        Ok(self)
    }
    pub fn new(rect: Rect, axes: Axes) -> Self {
        Self {
            rect,
            axes: Some(axes),
            axes_visible: true,
            layers: vec![],
            labels: vec![],
            legend: None,
        }
    }
    pub fn push(&mut self, layer: Layer) -> &mut Self {
        self.layers.push(layer);
        self
    }
    fn limits(&self) -> ([f64; 2], [f64; 2]) {
        self.axes
            .as_ref()
            .map(|a| (a.xlim, a.ylim))
            .unwrap_or(([0.0, 1.0], [0.0, 1.0]))
    }
    fn xy(&self, p: [f64; 2]) -> [f64; 2] {
        let (x, y) = self.limits();
        let xs = self.axes.as_ref().map(|a| a.xscale).unwrap_or_default();
        let ys = self.axes.as_ref().map(|a| a.yscale).unwrap_or_default();
        [
            self.rect.x
                + (xs.transform(p[0]) - xs.transform(x[0]))
                    / (xs.transform(x[1]) - xs.transform(x[0]))
                    * self.rect.width,
            self.rect.y + self.rect.height
                - (ys.transform(p[1]) - ys.transform(y[0]))
                    / (ys.transform(y[1]) - ys.transform(y[0]))
                    * self.rect.height,
        ]
    }
    /// Convert a data coordinate to figure millimetres, using the same axis
    /// transformations as lines, symbols and meshes.
    pub fn data_to_figure(&self, point: [f64; 2]) -> [f64; 2] {
        self.xy(point)
    }

    /// Physical stroke footprints for annotation collision checks. Supply
    /// densely sampled curves when the coordinate transformation is nonlinear.
    pub fn stroke_obstacles(&self, line: &Line, clearance_mm: f64) -> Vec<Rect> {
        let margin = line.width_pt * MM_PER_PT / 2. + clearance_mm;
        line.points
            .windows(2)
            .filter_map(|p| {
                let a = self.xy(p[0]);
                let b = self.xy(p[1]);
                a.iter().chain(b.iter()).all(|v| v.is_finite()).then(|| {
                    Rect::new(
                        a[0].min(b[0]) - margin,
                        a[1].min(b[1]) - margin,
                        (a[0] - b[0]).abs() + 2. * margin,
                        (a[1] - b[1]).abs() + 2. * margin,
                    )
                })
            })
            .collect()
    }

    /// Add faint major grid lines without introducing minor-grid clutter.
    pub fn major_grid(&mut self, color: &str, width_pt: f64) {
        let Some(a) = &self.axes else {
            return;
        };
        let mut paths = Vec::new();
        for &x in &a.xticks {
            if x > a.xlim[0] && x < a.xlim[1] {
                paths.extend([[x, a.ylim[0]], [x, a.ylim[1]], [f64::NAN; 2]]);
            }
        }
        for &y in &a.yticks {
            if y > a.ylim[0] && y < a.ylim[1] {
                paths.extend([[a.xlim[0], y], [a.xlim[1], y], [f64::NAN; 2]]);
            }
        }
        let mut grid = Line::new(paths, color);
        grid.width_pt = width_pt;
        grid.protect_from_legend = false;
        self.layers.insert(0, Layer::Line(grid));
    }
    fn data_fraction(&self, u: f64, v: f64) -> [f64; 2] {
        let (x, y) = self.limits();
        let xs = self.axes.as_ref().map(|a| a.xscale).unwrap_or_default();
        let ys = self.axes.as_ref().map(|a| a.yscale).unwrap_or_default();
        [
            xs.inverse(xs.transform(x[0]) + u * (xs.transform(x[1]) - xs.transform(x[0]))),
            ys.inverse(ys.transform(y[0]) + v * (ys.transform(y[1]) - ys.transform(y[0]))),
        ]
    }
}
#[derive(Clone, Debug)]
pub struct Colorbar {
    pub rect: Rect,
    pub scale: usize,
    pub label: String,
    pub ticks: Vec<f64>,
    pub horizontal: bool,
}
impl Colorbar {
    pub fn new(rect: Rect, scale: usize, label: impl Into<String>, ticks: Vec<f64>) -> Self {
        Self {
            rect,
            scale,
            label: label.into(),
            ticks,
            horizontal: false,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Figure {
    pub(crate) article_contract: Option<crate::article::ArticleContract>,
    pub(crate) web_contract: Option<crate::web_layout::WebContract>,
    pub width_mm: f64,
    pub height_mm: f64,
    pub font_pt: f64,
    pub panels: Vec<Panel>,
    pub colorbars: Vec<Colorbar>,
    pub labels: Vec<Label>,
    pub scales: Vec<Scale>,
    pub connectors: Vec<Connector>,
    pub latex: Latex,
    pub raster_dpi: f64,
    /// Fail on cropped or colliding label boxes; explicitly disable for intentional overlays.
    pub check_label_bounds: bool,
    /// Crop exported page to visible content plus padding. Disable for fixed page layouts.
    pub tight_export: bool,
    pub export_padding_mm: f64,
    pub check_label_overlap: bool,
    /// Reject labels obscured by colorbars, and axis labels colliding with data panels.
    pub check_label_geometry: bool,
    /// Reserve measured axis-label space before drawing an external colorbar.
    /// Disable only when exact manual placement is required.
    pub auto_colorbar_layout: bool,
    /// Minimum whitespace between axis decorations and an external colorbar, in em.
    pub colorbar_gap_em: f64,
}
// Rasterize shared geometry into one coverage buffer. Independent antialiased
// SVG paths leave background seams even when their corners coincide exactly.
fn raster_mesh(f: &QuadMesh, p: &Panel, scale: &Scale, dpi: f64) -> image::RgbaImage {
    let r = p.rect;
    let w = (r.width * dpi / 25.4).ceil().max(1.0) as u32;
    let h = (r.height * dpi / 25.4).ceil().max(1.0) as u32;
    let mut img = image::RgbaImage::new(w, h);
    let points: Vec<[f64; 2]> = f
        .corners
        .iter()
        .map(|&v| {
            let q = p.xy(v);
            [
                (q[0] - r.x) * w as f64 / r.width,
                (q[1] - r.y) * h as f64 / r.height,
            ]
        })
        .collect();
    let cross = |a: [f64; 2], b: [f64; 2], q: [f64; 2]| {
        (b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0])
    };
    for j in 0..f.ny {
        for i in 0..f.nx {
            let Some(color) = scale.map(f.values[j * f.nx + i]) else {
                continue;
            };
            let a = j * (f.nx + 1) + i;
            for ids in [[a, a + 1, a + f.nx + 2], [a, a + f.nx + 2, a + f.nx + 1]] {
                let v = ids.map(|id| points[id]);
                let area = cross(v[0], v[1], v[2]);
                if area.abs() < 1e-14 {
                    continue;
                }
                let x0 = v
                    .iter()
                    .map(|v| v[0])
                    .fold(f64::INFINITY, f64::min)
                    .floor()
                    .max(0.0) as u32;
                let x1 = v
                    .iter()
                    .map(|v| v[0])
                    .fold(f64::NEG_INFINITY, f64::max)
                    .ceil()
                    .max(0.0)
                    .min(w as f64) as u32;
                let y0 = v
                    .iter()
                    .map(|v| v[1])
                    .fold(f64::INFINITY, f64::min)
                    .floor()
                    .max(0.0) as u32;
                let y1 = v
                    .iter()
                    .map(|v| v[1])
                    .fold(f64::NEG_INFINITY, f64::max)
                    .ceil()
                    .max(0.0)
                    .min(h as f64) as u32;
                for y in y0..y1 {
                    for x in x0..x1 {
                        let q = [x as f64 + 0.5, y as f64 + 0.5];
                        if (0..3).all(|k| cross(v[k], v[(k + 1) % 3], q) * area.signum() >= -1e-9) {
                            img.put_pixel(x, y, image::Rgba(color));
                        }
                    }
                }
            }
        }
    }
    img
}

fn tick_label(value: f64, labels: &Option<Vec<String>>, index: usize) -> String {
    labels
        .as_ref()
        .map(|v| v[index].clone())
        .unwrap_or_else(|| number(value))
}

// Scanline coverage of independent cells avoids antialiased seams between
// adjacent SVG polygons. The even-odd fill also supports concave footprints.
fn raster_polygons(mesh: &PolygonMesh, panel: &Panel, scale: &Scale, dpi: f64) -> image::RgbaImage {
    let r = panel.rect;
    let w = (r.width * dpi / 25.4).ceil().max(1.) as u32;
    let h = (r.height * dpi / 25.4).ceil().max(1.) as u32;
    let mut out = image::RgbaImage::new(w, h);
    for (vertices, value) in mesh.polygons.iter().zip(&mesh.values) {
        let Some(color) = scale.map(*value) else {
            continue;
        };
        let p: Vec<_> = vertices
            .iter()
            .map(|&v| {
                let q = panel.xy(v);
                [
                    (q[0] - r.x) * w as f64 / r.width,
                    (q[1] - r.y) * h as f64 / r.height,
                ]
            })
            .collect();
        let lo = p
            .iter()
            .map(|q| q[1])
            .fold(f64::INFINITY, f64::min)
            .floor()
            .max(0.) as u32;
        let hi = p
            .iter()
            .map(|q| q[1])
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil()
            .clamp(0., h as f64) as u32;
        for y in lo..hi {
            let scan = y as f64 + 0.5;
            let mut cuts = Vec::new();
            for i in 0..p.len() {
                let (a, b) = (p[i], p[(i + 1) % p.len()]);
                if (a[1] <= scan && b[1] > scan) || (b[1] <= scan && a[1] > scan) {
                    cuts.push(a[0] + (scan - a[1]) * (b[0] - a[0]) / (b[1] - a[1]));
                }
            }
            cuts.sort_by(f64::total_cmp);
            for pair in cuts.chunks_exact(2) {
                let x0 = (pair[0] - 0.5).ceil().clamp(0., w as f64) as u32;
                let x1 = (pair[1] - 0.5).ceil().clamp(0., w as f64) as u32;
                for x in x0..x1 {
                    out.put_pixel(x, y, image::Rgba(color));
                }
            }
        }
    }
    out
}

// Coordinates match Panel::markers; SVG y is inverted relative to data y.
fn render_legend_marker(
    svg: &mut String,
    shape: crate::MarkerShape,
    color: &str,
    x: f64,
    y: f64,
    size_pt: f64,
    outline_width_pt: Option<f64>,
) -> Result<()> {
    use crate::MarkerShape;
    let r = size_pt * MM_PER_PT / 2.;
    let paint = if let Some(width) = outline_width_pt {
        format!(
            "fill=\"none\" stroke=\"{color}\" stroke-width=\"{}\"",
            width * MM_PER_PT
        )
    } else {
        format!("fill=\"{color}\"")
    };
    if shape == MarkerShape::Circle {
        write!(svg, "<circle cx=\"{x}\" cy=\"{y}\" r=\"{r}\" {paint}/>")?;
    } else {
        let vertices: &[[f64; 2]] = match shape {
            MarkerShape::Square => &[[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]],
            MarkerShape::Diamond => &[[-1., 0.], [0., -1.], [1., 0.], [0., 1.]],
            MarkerShape::Triangle => &[[0., 1.], [-0.866, -0.5], [0.866, -0.5]],
            MarkerShape::HorizontalCap => &[[-1., 0.], [1., 0.]],
            MarkerShape::VerticalCap => &[[0., -1.], [0., 1.]],
            MarkerShape::Circle => unreachable!(),
        };
        let points = vertices
            .iter()
            .map(|v| format!("{},{}", x + r * v[0], y - r * v[1]))
            .collect::<Vec<_>>()
            .join(" ");
        if vertices.len() == 2 {
            write!(
                svg,
                "<polyline points=\"{points}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"{}\"/>",
                outline_width_pt.unwrap_or(0.6) * MM_PER_PT
            )?;
        } else {
            write!(svg, "<polygon points=\"{points}\" {paint}/>")?;
        }
    }
    Ok(())
}

impl Figure {
    pub fn new(use_case: IntendedUse) -> Self {
        let (w, font) = use_case.dimensions();
        Self {
            article_contract: None,
            web_contract: None,
            width_mm: w,
            height_mm: w / PHI,
            font_pt: font,
            panels: vec![],
            colorbars: vec![],
            labels: vec![],
            scales: vec![],
            connectors: vec![],
            latex: Latex {
                font_pt: font,
                ..Latex::default()
            },
            raster_dpi: 300.0,
            check_label_bounds: true,
            tight_export: true,
            export_padding_mm: 1.0,
            check_label_overlap: true,
            check_label_geometry: true,
            auto_colorbar_layout: true,
            colorbar_gap_em: 0.6,
        }
    }
    pub fn scale(&mut self, s: Scale) -> usize {
        let id = self.scales.len();
        self.scales.push(s);
        id
    }
    /// Consistent optional layout helper. Free `Panel::rect` placement also works.
    pub fn grid(&mut self, rows: usize, cols: usize) -> Result<Vec<usize>> {
        ensure!(rows > 0 && cols > 0, "Empty grid");
        let em = self.font_pt * MM_PER_PT;
        let (left, right, top, bottom) = (4.7 * em, 1.6 * em, 2.2 * em, 3.7 * em);
        let gap_x = 6.4 * em;
        let gap_y = 5.0 * em;
        let w = (self.width_mm - left - right - (cols - 1) as f64 * gap_x) / cols as f64;
        ensure!(
            w > 5.0 * em,
            "Panels too narrow for intended use; choose wider figure"
        );
        let h = w / PHI;
        self.height_mm = top + bottom + rows as f64 * h + (rows - 1) as f64 * gap_y;
        let mut ids = vec![];
        for r in 0..rows {
            for c in 0..cols {
                ids.push(self.panels.len());
                self.panels.push(Panel::new(
                    Rect::new(
                        left + c as f64 * (w + gap_x),
                        top + r as f64 * (h + gap_y),
                        w,
                        h,
                    ),
                    Axes::new([0.0, 1.0], [0.0, 1.0], "", ""),
                ));
            }
        }
        Ok(ids)
    }
    fn validate(&self) -> Result<()> {
        if let Some(contract) = self.article_contract {
            contract.validate(self)?;
        }
        if let Some(contract) = self.web_contract {
            contract.validate(self)?;
        }
        ensure!(
            [self.width_mm, self.height_mm, self.font_pt, self.raster_dpi]
                .iter()
                .all(|x| x.is_finite() && *x > 0.0),
            "Invalid figure dimensions"
        );
        for s in &self.scales {
            s.validate()?
        }
        for p in &self.panels {
            ensure!(p.rect.valid(), "Invalid panel rectangle");
            if let Some(legend) = &p.legend {
                legend.validate(p)?;
            }
            if let Some(a) = &p.axes {
                a.xscale.validate(a.xlim)?;
                a.yscale.validate(a.ylim)?;
                for (ticks, labels) in [(&a.xticks, &a.xticklabels), (&a.yticks, &a.yticklabels)] {
                    ensure!(ticks.iter().all(|v| v.is_finite()), "Non-finite axis tick");
                    ensure!(
                        labels.as_ref().is_none_or(
                            |l| l.len() == ticks.len() && l.iter().all(|s| !s.is_empty())
                        ),
                        "Tick label count must match tick positions and labels must be nonempty"
                    );
                }
                for secondary in a.secondary_x.iter().chain(a.secondary_y.iter()) {
                    ensure!(
                        secondary
                            .ticks
                            .iter()
                            .all(|(x, v)| x.is_finite() && v.is_finite()),
                        "Non-finite secondary-axis tick"
                    );
                    ensure!(
                        secondary
                            .tick_labels
                            .as_ref()
                            .is_none_or(|labels| labels.len() == secondary.ticks.len()
                                && labels.iter().all(|s| !s.is_empty())),
                        "Secondary tick label count must match tick positions"
                    );
                }
            }
            let (x, y) = p.limits();
            ensure!(
                x[0].is_finite()
                    && x[1].is_finite()
                    && x[1] > x[0]
                    && y[0].is_finite()
                    && y[1].is_finite()
                    && y[1] > y[0],
                "Invalid panel limits"
            );
            for l in &p.layers {
                let id = match l {
                    Layer::Image(img) => {
                        ensure!(img.width() > 0 && img.height() > 0, "Empty raster image");
                        continue;
                    }
                    Layer::Line(f) => {
                        ensure!(
                            f.fill_opacity.is_finite() && (0.0..=1.0).contains(&f.fill_opacity),
                            "Invalid path fill opacity"
                        );
                        ensure!(
                            f.opacity.is_finite() && (0.0..=1.0).contains(&f.opacity),
                            "Invalid line opacity"
                        );
                        ensure!(
                            f.width_pt.is_finite() && f.width_pt >= 0.0,
                            "Invalid line width"
                        );
                        continue;
                    }
                    Layer::Pcolormesh(f) => {
                        ensure!(
                            increasing(&f.x_edges)
                                && increasing(&f.y_edges)
                                && f.values.len() == (f.x_edges.len() - 1) * (f.y_edges.len() - 1),
                            "pcolormesh shape mismatch"
                        );
                        f.scale
                    }
                    Layer::Surface(f) => {
                        ensure!(f.scale < self.scales.len(), "Missing surface scale");
                        f.scale
                    }
                    Layer::QuadMesh(f) => {
                        ensure!(
                            f.nx > 0
                                && f.ny > 0
                                && f.corners.len() == (f.nx + 1) * (f.ny + 1)
                                && f.values.len() == f.nx * f.ny
                                && f.corners.iter().flatten().all(|x| x.is_finite()),
                            "Invalid curvilinear mesh"
                        );
                        f.scale
                    }
                    Layer::PolygonMesh(f) => {
                        f.validate()?;
                        f.scale
                    }
                    Layer::Contour(f) => {
                        ensure!(
                            increasing(&f.x)
                                && increasing(&f.y)
                                && increasing(&f.levels)
                                && f.values.len() == f.x.len() * f.y.len()
                                && f.width_pt.is_finite()
                                && f.width_pt >= 0.0,
                            "Invalid contour grid or levels"
                        );
                        f.scale
                    }
                };
                ensure!(id < self.scales.len(), "Unknown color scale");
            }
        }
        for b in &self.colorbars {
            ensure!(
                b.rect.valid() && b.scale < self.scales.len(),
                "Invalid colorbar"
            );
            ensure!(
                b.ticks.iter().all(|x| x.is_finite()
                    && *x >= self.scales[b.scale].limits[0]
                    && *x <= self.scales[b.scale].limits[1]),
                "Colorbar ticks outside scale"
            );
        }
        for c in &self.connectors {
            ensure!(
                c.from_panel < self.panels.len() && c.to_panel < self.panels.len(),
                "Unknown connector panel"
            );
            ensure!(
                c.from
                    .iter()
                    .chain(c.to.iter())
                    .chain(c.via.iter().flatten())
                    .all(|v| v.is_finite())
                    && c.width_pt.is_finite()
                    && c.width_pt >= 0.,
                "Invalid connector coordinates or width"
            );
        }
        Ok(())
    }
    pub(crate) fn all_labels(&self) -> Vec<String> {
        let mut v = vec![];
        for p in &self.panels {
            if let Some(a) = &p.axes
                && p.axes_visible
            {
                v.extend([a.xlabel.clone(), a.ylabel.clone()]);
                v.extend(
                    a.xticks
                        .iter()
                        .enumerate()
                        .map(|(i, x)| tick_label(*x, &a.xticklabels, i)),
                );
                v.extend(
                    a.yticks
                        .iter()
                        .enumerate()
                        .map(|(i, x)| tick_label(*x, &a.yticklabels, i)),
                );
                if let Some(top) = &a.secondary_x {
                    v.push(top.label.clone());
                    v.extend(
                        top.ticks
                            .iter()
                            .enumerate()
                            .map(|(i, (_, v))| tick_label(*v, &top.tick_labels, i)),
                    );
                }
                if let Some(right) = &a.secondary_y {
                    v.push(right.label.clone());
                    v.extend(
                        right
                            .ticks
                            .iter()
                            .enumerate()
                            .map(|(i, (_, v))| tick_label(*v, &right.tick_labels, i)),
                    );
                }
            }
            v.extend(p.labels.iter().map(|l| l.latex.clone()));
            if let Some(legend) = &p.legend {
                v.extend(legend.entries.iter().map(|entry| entry.label.clone()));
            }
        }
        for b in &self.colorbars {
            v.push(b.label.clone());
            v.extend(b.ticks.iter().map(|x| number(*x)));
        }
        v.extend(self.labels.iter().map(|l| l.latex.clone()));
        v.retain(|s| !s.is_empty());
        v
    }
    fn render_legend(
        &self,
        svg: &mut String,
        panel: &Panel,
        legend: &crate::Legend,
        glyphs: &BTreeMap<String, Glyph>,
        serial: &mut usize,
        bounds: &mut Vec<(String, [f64; 4])>,
        reserved: &[Rect],
    ) -> Result<Rect> {
        let row_height = legend
            .entries
            .iter()
            .map(|e| {
                let symbol_height = match e.symbol {
                    crate::LegendSymbol::OutlinedMarker {
                        size_pt, width_pt, ..
                    } => size_pt + width_pt,
                    crate::LegendSymbol::Line { width_pt, .. } => width_pt,
                    crate::LegendSymbol::Circle { radius_pt, .. } => 2. * radius_pt,
                    crate::LegendSymbol::Marker { size_pt, .. } => size_pt,
                };
                glyphs[&e.label].height_pt.max(symbol_height) * MM_PER_PT + 1.5
            })
            .fold(self.font_pt * MM_PER_PT * 1.35, f64::max);
        let symbol_width = legend
            .entries
            .iter()
            .map(|entry| match entry.symbol {
                crate::LegendSymbol::OutlinedMarker {
                    size_pt, width_pt, ..
                } => (size_pt + width_pt) * MM_PER_PT + 0.25,
                crate::LegendSymbol::Circle { radius_pt, .. } => 2. * radius_pt * MM_PER_PT + 0.25,
                crate::LegendSymbol::Marker { size_pt, .. } => size_pt * MM_PER_PT + 0.25,
                crate::LegendSymbol::Line { .. } => 7.,
            })
            .fold(7., f64::max);
        let text_offset = symbol_width + 3.;
        let column_gap = 4.;
        let layout = |columns: usize| {
            let mut text_widths = vec![0.0_f64; columns];
            for (i, entry) in legend.entries.iter().enumerate() {
                text_widths[i % columns] =
                    text_widths[i % columns].max(glyphs[&entry.label].width_pt * MM_PER_PT);
            }
            let mut offsets = vec![0.];
            for width in text_widths.iter().take(columns - 1) {
                offsets.push(offsets.last().unwrap() + text_offset + width + column_gap);
            }
            let width = 2. * legend.padding_mm
                + text_offset * columns as f64
                + text_widths.iter().sum::<f64>()
                + column_gap * (columns - 1) as f64;
            let height =
                2. * legend.padding_mm + row_height * legend.entries.len().div_ceil(columns) as f64;
            (offsets, width, height)
        };
        let mask = if legend.avoid_data {
            self.legend_occupancy(panel, legend)?
        } else {
            vec![]
        };
        let w = (panel.rect.width * 4.).ceil() as usize;
        let h = (panel.rect.height * 4.).ceil() as usize;
        // An integral image makes the placement search independent of data size.
        let mut summed = vec![0usize; (w + 1) * (h + 1)];
        if !mask.is_empty() {
            for y in 0..h {
                let mut row = 0;
                for x in 0..w {
                    row += usize::from(mask[y * w + x]);
                    summed[(y + 1) * (w + 1) + x + 1] = summed[y * (w + 1) + x + 1] + row;
                }
            }
        }
        let occupied = |r: Rect| {
            if r.x < 0.5
                || r.y < 0.5
                || r.x + r.width > self.width_mm - 0.5
                || r.y + r.height > self.height_mm - 0.5
            {
                return true;
            }
            let intersects = |other: Rect| {
                r.x < other.x + other.width + 0.5
                    && r.x + r.width > other.x - 0.5
                    && r.y < other.y + other.height + 0.5
                    && r.y + r.height > other.y - 0.5
            };
            if bounds
                .iter()
                .any(|(_, b)| intersects(Rect::new(b[0], b[1], b[2] - b[0], b[3] - b[1])))
                || reserved.iter().any(|&other| intersects(other))
                || self.colorbars.iter().any(|bar| intersects(bar.rect))
                || self
                    .panels
                    .iter()
                    .filter(|p| !std::ptr::eq(*p, panel))
                    .any(|p| intersects(p.rect))
            {
                return true;
            }
            // A half-millimetre clearance also protects antialiased edges.
            let ix = |v: f64| ((v - panel.rect.x) * 4.).floor().clamp(0., w as f64) as usize;
            let iy = |v: f64| ((v - panel.rect.y) * 4.).floor().clamp(0., h as f64) as usize;
            let (x0, x1, y0, y1) = (
                ix(r.x - 0.5),
                ix(r.x + r.width + 0.75),
                iy(r.y - 0.5),
                iy(r.y + r.height + 0.75),
            );
            summed[y1 * (w + 1) + x1] + summed[y0 * (w + 1) + x0]
                > summed[y0 * (w + 1) + x1] + summed[y1 * (w + 1) + x0]
        };
        let mut counts = vec![legend.columns];
        counts.extend((1..=legend.entries.len()).filter(|&n| n != legend.columns));
        let mut chosen = None;
        // Try every usable column count inside before considering any exterior.
        for outside in [false, true] {
            for &columns in &counts {
                let (offsets, width, height) = layout(columns);
                let r = if outside {
                    legend.place_outside(panel.rect, width, height, &occupied)
                } else {
                    legend.place_inside(panel.rect, width, height, &occupied)
                };
                if let Some(r) = r {
                    chosen = Some((r, columns, offsets));
                    break;
                }
            }
            if chosen.is_some() {
                break;
            }
        }
        let (r,columns,offsets)=chosen.ok_or_else(|| anyhow::anyhow!("No legend placement avoids protected content within the figure; shorten the legend or enlarge the figure"))?;
        write!(
            svg,
            "<g data-role=\"legend\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"0.8\" fill=\"white\" fill-opacity=\"{}\"/>",
            r.x, r.y, r.width, r.height, legend.background_opacity
        )?;
        for (i, entry) in legend.entries.iter().enumerate() {
            let x = r.x + legend.padding_mm + offsets[i % columns];
            let y = r.y + legend.padding_mm + ((i / columns) as f64 + 0.5) * row_height;
            match &entry.symbol {
                crate::LegendSymbol::Line {
                    color,
                    width_pt,
                    dash,
                } => {
                    write!(
                        svg,
                        "<path d=\"M {x} {y} h {symbol_width}\" stroke=\"{color}\" stroke-width=\"{}\" {} fill=\"none\"/>",
                        width_pt * MM_PER_PT,
                        dash.as_ref()
                            .map(|d| format!("stroke-dasharray=\"{d}\""))
                            .unwrap_or_default()
                    )?;
                }
                crate::LegendSymbol::Marker {
                    shape,
                    color,
                    size_pt,
                } => {
                    render_legend_marker(
                        svg,
                        *shape,
                        color,
                        x + symbol_width / 2.,
                        y,
                        *size_pt,
                        None,
                    )?;
                }
                crate::LegendSymbol::OutlinedMarker {
                    shape,
                    color,
                    size_pt,
                    width_pt,
                } => {
                    render_legend_marker(
                        svg,
                        *shape,
                        color,
                        x + symbol_width / 2.,
                        y,
                        *size_pt,
                        Some(*width_pt),
                    )?;
                }
                crate::LegendSymbol::Circle { color, radius_pt } => {
                    write!(
                        svg,
                        "<circle cx=\"{}\" cy=\"{y}\" r=\"{}\" fill=\"{color}\" stroke=\"white\" stroke-width=\"0.25\"/>",
                        x + symbol_width / 2.,
                        radius_pt * MM_PER_PT
                    )?;
                }
            }
            place(
                svg,
                glyphs,
                &entry.label,
                x + text_offset + glyphs[&entry.label].width_pt * MM_PER_PT / 2.,
                y,
                0.,
                "#171717",
                serial,
                bounds,
            )?;
        }
        svg.push_str("</g>");
        Ok(r)
    }

    pub(crate) fn legend_occupancy(
        &self,
        panel: &Panel,
        legend: &crate::Legend,
    ) -> Result<Vec<bool>> {
        let w = (panel.rect.width * 4.).ceil() as u32;
        let h = (panel.rect.height * 4.).ceil() as u32;
        let mut mask = vec![false; (w * h) as usize];
        let protect_line = |mask: &mut Vec<bool>, points: &[[f64; 2]], width_pt: f64| {
            for pair in points.windows(2) {
                if !pair.iter().flatten().all(|v| v.is_finite()) {
                    continue;
                }
                let [a, b] = [panel.xy(pair[0]), panel.xy(pair[1])];
                if !a.iter().chain(&b).all(|v| v.is_finite()) {
                    continue;
                }
                let steps = ((b[0] - a[0]).hypot(b[1] - a[1]) * 8.).ceil().max(1.) as usize;
                let radius = (width_pt * MM_PER_PT * 2.).ceil().max(1.) as i32;
                for i in 0..=steps {
                    let t = i as f64 / steps as f64;
                    let x = ((a[0] + t * (b[0] - a[0]) - panel.rect.x) * 4.).round() as i32;
                    let y = ((a[1] + t * (b[1] - a[1]) - panel.rect.y) * 4.).round() as i32;
                    for yy in (y - radius).max(0)..=(y + radius).min(h as i32 - 1) {
                        for xx in (x - radius).max(0)..=(x + radius).min(w as i32 - 1) {
                            mask[yy as usize * w as usize + xx as usize] = true;
                        }
                    }
                }
            }
        };
        for (i, layer) in panel.layers.iter().enumerate() {
            let protected = if legend.avoid_layers.is_empty() {
                match layer {
                    Layer::Image(_) => false,
                    Layer::Line(line) => line.protect_from_legend,
                    _ => true,
                }
            } else {
                legend.avoid_layers.contains(&i)
            };
            if !protected {
                continue;
            }
            let raster = match layer {
                Layer::Line(line) => {
                    if line.opacity > 0. {
                        protect_line(&mut mask, &line.points, line.width_pt);
                    }
                    if line.fill.is_some() && line.fill_opacity > 0. {
                        let polygons: Vec<Vec<[f64; 2]>> = line
                            .points
                            .split(|p| !p.iter().all(|v| v.is_finite()))
                            .filter(|p| p.len() >= 3)
                            .map(|p| p.to_vec())
                            .collect();
                        let values = vec![0.5; polygons.len()];
                        let mesh = PolygonMesh::new(polygons, values, 0)?;
                        let scale = Scale::new(
                            [0., 1.],
                            crate::Normalization::Linear,
                            crate::ColorMap::new(vec![[0., 0., 0.], [0., 0., 0.]])?,
                        )?;
                        raster_polygons(&mesh, panel, &scale, 101.6)
                    } else {
                        continue;
                    }
                }
                Layer::PolygonMesh(mesh) => {
                    raster_polygons(mesh, panel, &self.scales[mesh.scale], 101.6)
                }
                Layer::QuadMesh(mesh) => raster_mesh(mesh, panel, &self.scales[mesh.scale], 101.6),
                Layer::Surface(surface) => surface.raster(w, h, &self.scales[surface.scale]),
                Layer::Image(img) => {
                    image::imageops::resize(img, w, h, image::imageops::FilterType::Nearest)
                }
                Layer::Pcolormesh(_) | Layer::Contour(_) => {
                    if let Layer::Contour(c) = layer
                        && !c.filled
                    {
                        for level in &c.levels {
                            for segment in c.segments(*level) {
                                protect_line(&mut mask, &segment, c.width_pt);
                            }
                        }
                        continue;
                    }
                    let mut img = image::RgbaImage::new(w, h);
                    for y in 0..h {
                        for x in 0..w {
                            let [px, py] = panel.data_fraction(
                                (x as f64 + 0.5) / w as f64,
                                1. - (y as f64 + 0.5) / h as f64,
                            );
                            let visible = match layer {
                                Layer::Pcolormesh(f) => {
                                    self.scales[f.scale].map(f.sample(px, py)).is_some()
                                }
                                Layer::Contour(c) => {
                                    let v = c.sample(px, py);
                                    v.is_finite()
                                        && v >= c.levels[0]
                                        && v <= *c.levels.last().unwrap()
                                        && self.scales[c.scale].map(v).is_some()
                                }
                                _ => unreachable!(),
                            };
                            if visible {
                                img.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
                            }
                        }
                    }
                    img
                }
            };
            for (i, pixel) in raster.pixels().enumerate() {
                if pixel[3] > 0 {
                    mask[i] = true;
                }
            }
        }
        Ok(mask)
    }
    /// LaTeX glyph paths and geometry remain vector; dense fields are images.
    pub fn svg(&self) -> Result<String> {
        self.validate()?;
        let mut latex = self.latex.clone();
        latex.font_pt = self.font_pt;
        let glyphs = latex.typeset(self.all_labels())?;
        if self.auto_colorbar_layout {
            let mut resolved = self.clone();
            resolved.resolve_colorbar_layout(&glyphs)?;
            return resolved.svg_with_glyphs(&glyphs);
        }
        self.svg_with_glyphs(&glyphs)
    }

    /// Resolve external colorbar spacing using actual LaTeX glyph dimensions.
    /// SVG/save apply this automatically. Call explicitly before exporting layout
    /// coordinates for a live WebGL overlay. Panel/data rectangles do not move.
    pub fn layout_colorbars(&mut self) -> Result<()> {
        self.validate()?;
        let mut latex = self.latex.clone();
        latex.font_pt = self.font_pt;
        let glyphs = latex.typeset(self.all_labels())?;
        self.resolve_colorbar_layout(&glyphs)
    }

    fn resolve_colorbar_layout(&mut self, glyphs: &BTreeMap<String, Glyph>) -> Result<()> {
        ensure!(
            self.colorbar_gap_em.is_finite() && self.colorbar_gap_em >= 0.,
            "Invalid colorbar spacing"
        );
        let em = self.font_pt * MM_PER_PT;
        let gap = self.colorbar_gap_em * em;
        let height = |label: &str| glyphs.get(label).map_or(0., |g| g.height_pt * MM_PER_PT);
        for bar in &mut self.colorbars {
            let original = bar.rect;
            for panel in &self.panels {
                let r = panel.rect;
                if bar.horizontal
                    && original.y >= r.y + r.height - 0.05
                    && original.x < r.x + r.width
                    && original.x + original.width > r.x
                {
                    let mut bottom = r.y + r.height;
                    if panel.axes_visible
                        && let Some(a) = &panel.axes
                    {
                        if !a.xlabel.is_empty() {
                            bottom = bottom.max(r.y + r.height + 2.5 * em + height(&a.xlabel) / 2.);
                        }
                        for (i, v) in a.xticks.iter().enumerate() {
                            if *v >= a.xlim[0] && *v <= a.xlim[1] {
                                bottom = bottom.max(
                                    r.y + r.height
                                        + 0.95 * em
                                        + height(&tick_label(*v, &a.xticklabels, i)) / 2.,
                                );
                            }
                        }
                    }
                    bar.rect.y = bar.rect.y.max(bottom + gap);
                } else if !bar.horizontal
                    && original.x >= r.x + r.width - 0.05
                    && original.y < r.y + r.height
                    && original.y + original.height > r.y
                {
                    bar.rect.x = bar.rect.x.max(r.x + r.width + gap);
                }
            }
            if bar.rect.x > original.x {
                ensure!(
                    bar.rect.x + bar.rect.width <= self.width_mm,
                    "Insufficient figure width for colorbar clearance; reserve more space beside the panel"
                );
            }
            if bar.rect.y > original.y {
                let label_bottom = if bar.label.is_empty() {
                    bar.rect.y + bar.rect.height
                } else {
                    bar.rect.y + bar.rect.height + 2.9 * em + height(&bar.label) / 2.
                };
                let ticks_bottom = bar
                    .ticks
                    .iter()
                    .map(|v| bar.rect.y + bar.rect.height + 1.3 * em + height(&number(*v)) / 2.)
                    .fold(bar.rect.y + bar.rect.height, f64::max);
                self.height_mm = self.height_mm.max(label_bottom.max(ticks_bottom) + gap);
            }
        }
        Ok(())
    }

    fn svg_with_glyphs(&self, glyphs: &BTreeMap<String, Glyph>) -> Result<String> {
        let mut s = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{}mm\" height=\"{}mm\" viewBox=\"0 0 {} {}\"><rect width=\"100%\" height=\"100%\" fill=\"white\"/>",
            self.width_mm, self.height_mm, self.width_mm, self.height_mm
        );
        let mut serial = 0;
        let mut label_bounds: Vec<(String, [f64; 4])> = vec![];
        let mut axis_bounds: Vec<(String, [f64; 4])> = vec![];
        for (pi, p) in self.panels.iter().enumerate() {
            let r = p.rect;
            write!(
                s,
                "<defs><clipPath id=\"panel-{pi}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/></clipPath></defs><g clip-path=\"url(#panel-{pi})\">",
                r.x, r.y, r.width, r.height
            )?;
            for layer in &p.layers {
                match layer {
                    Layer::Line(line) => {
                        write_line(&mut s, p, line)?;
                    }
                    Layer::Image(img) => self.embed_image(&mut s, p.rect, img.clone())?,
                    Layer::Pcolormesh(field) => self.image(&mut s, p, |x, y| {
                        self.scales[field.scale].map(field.sample(x, y))
                    })?,
                    Layer::Surface(f) => {
                        let w = (p.rect.width * self.raster_dpi / 25.4).ceil() as u32;
                        let h = (p.rect.height * self.raster_dpi / 25.4).ceil() as u32;
                        self.embed_image(&mut s, p.rect, f.raster(w, h, &self.scales[f.scale]))?;
                    }
                    Layer::QuadMesh(f) => {
                        let img = raster_mesh(f, p, &self.scales[f.scale], self.raster_dpi);
                        self.embed_image(&mut s, p.rect, img)?;
                    }
                    Layer::PolygonMesh(f) => {
                        self.embed_image(
                            &mut s,
                            p.rect,
                            raster_polygons(f, p, &self.scales[f.scale], self.raster_dpi),
                        )?;
                    }
                    Layer::Contour(c) => {
                        let scale = &self.scales[c.scale];
                        if c.filled {
                            self.image(&mut s, p, |x, y| {
                                let v = c.sample(x, y);
                                if !v.is_finite()
                                    || v < c.levels[0]
                                    || v > *c.levels.last().unwrap()
                                {
                                    return None;
                                }
                                let k = c
                                    .levels
                                    .partition_point(|l| *l <= v)
                                    .saturating_sub(1)
                                    .min(c.levels.len() - 2);
                                scale.map((c.levels[k] + c.levels[k + 1]) * 0.5)
                            })?;
                        } else {
                            for level in &c.levels {
                                let Some(color) = scale.map(*level) else {
                                    continue;
                                };
                                let mut path = String::new();
                                for segment in c.segments(*level) {
                                    let [a, b] = segment.map(|x| p.xy(x));
                                    write!(path, "M {} {} L {} {} ", a[0], a[1], b[0], b[1])?;
                                }
                                write!(
                                    s,
                                    "<path d=\"{path}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/>",
                                    rgb(color),
                                    c.width_pt * MM_PER_PT
                                )?;
                            }
                        }
                    }
                }
            }
            s.push_str("</g>");
            if let Some(a) = &p.axes
                && p.axes_visible
            {
                let axis_start = label_bounds.len();
                let em = self.font_pt * MM_PER_PT;
                if a.show_x_axis {
                    write!(
                        s,
                        "<path d=\"M {} {} H {}\" stroke=\"#444\" stroke-width=\"0.18\" fill=\"none\"/>",
                        r.x,
                        r.y + r.height,
                        r.x + r.width
                    )?;
                }
                if a.show_y_axis {
                    write!(
                        s,
                        "<path d=\"M {} {} V {}\" stroke=\"#444\" stroke-width=\"0.18\" fill=\"none\"/>",
                        r.x,
                        r.y,
                        r.y + r.height
                    )?;
                }
                for (i, v) in a.xticks.iter().enumerate() {
                    if !a.show_x_axis {
                        continue;
                    }
                    if *v < a.xlim[0] || *v > a.xlim[1] {
                        continue;
                    }
                    let [x, y] = p.xy([*v, a.ylim[0]]);
                    write!(
                        s,
                        "<path d=\"M {x} {y} v 1.2\" stroke=\"#444\" stroke-width=\"0.18\"/>"
                    )?;
                    place(
                        &mut s,
                        &glyphs,
                        &tick_label(*v, &a.xticklabels, i),
                        x,
                        y + 0.95 * em,
                        0.0,
                        "#171717",
                        &mut serial,
                        &mut label_bounds,
                    )?;
                }
                for &v in &a.xminor_ticks {
                    if !a.show_x_axis {
                        continue;
                    }
                    if v < a.xlim[0] || v > a.xlim[1] {
                        continue;
                    }
                    let [x, y] = p.xy([v, a.ylim[0]]);
                    write!(
                        s,
                        "<path d=\"M {x} {y} v 0.7\" stroke=\"#444\" stroke-width=\"0.15\"/>"
                    )?;
                }
                for &v in &a.yminor_ticks {
                    if !a.show_y_axis {
                        continue;
                    }
                    if v < a.ylim[0] || v > a.ylim[1] {
                        continue;
                    }
                    let [x, y] = p.xy([a.xlim[0], v]);
                    write!(
                        s,
                        "<path d=\"M {x} {y} h -0.7\" stroke=\"#444\" stroke-width=\"0.15\"/>"
                    )?;
                }
                if let Some(top) = &a.secondary_x {
                    write!(
                        s,
                        "<path d=\"M {} {} H {}\" stroke=\"#444\" stroke-width=\"0.18\" fill=\"none\"/>",
                        r.x,
                        r.y,
                        r.x + r.width
                    )?;
                    for (i, &(primary, value)) in top.ticks.iter().enumerate() {
                        if primary < a.xlim[0] || primary > a.xlim[1] {
                            continue;
                        }
                        let [x, _] = p.xy([primary, a.ylim[1]]);
                        write!(
                            s,
                            "<path d=\"M {x} {} v -1.2\" stroke=\"#444\" stroke-width=\"0.18\"/>",
                            r.y
                        )?;
                        place(
                            &mut s,
                            &glyphs,
                            &tick_label(value, &top.tick_labels, i),
                            x,
                            r.y - 0.95 * em,
                            0.,
                            "#171717",
                            &mut serial,
                            &mut label_bounds,
                        )?;
                    }
                    place(
                        &mut s,
                        &glyphs,
                        &top.label,
                        r.x + r.width / 2.,
                        r.y - 2.5 * em,
                        0.,
                        "#171717",
                        &mut serial,
                        &mut label_bounds,
                    )?;
                }
                for (i, v) in a.yticks.iter().enumerate() {
                    if !a.show_y_axis {
                        continue;
                    }
                    if *v < a.ylim[0] || *v > a.ylim[1] {
                        continue;
                    }
                    let [x, y] = p.xy([a.xlim[0], *v]);
                    write!(
                        s,
                        "<path d=\"M {x} {y} h -1.2\" stroke=\"#444\" stroke-width=\"0.18\"/>"
                    )?;
                    let text = tick_label(*v, &a.yticklabels, i);
                    let width = glyphs[&text].width_pt * MM_PER_PT;
                    place(
                        &mut s,
                        &glyphs,
                        &text,
                        x - 0.6 * em - width / 2.0,
                        y,
                        0.0,
                        "#171717",
                        &mut serial,
                        &mut label_bounds,
                    )?;
                }
                if a.show_x_axis {
                    place(
                        &mut s,
                        &glyphs,
                        &a.xlabel,
                        r.x + r.width / 2.0,
                        r.y + r.height + 2.5 * em,
                        0.0,
                        "#171717",
                        &mut serial,
                        &mut label_bounds,
                    )?;
                }
                if a.show_y_axis {
                    place(
                        &mut s,
                        &glyphs,
                        &a.ylabel,
                        r.x - 3.8 * em,
                        r.y + r.height / 2.0,
                        -90.0,
                        "#171717",
                        &mut serial,
                        &mut label_bounds,
                    )?;
                }
                if let Some(right) = &a.secondary_y {
                    let edge = r.x + r.width;
                    let mut tick_right = edge;
                    write!(
                        s,
                        "<path d=\"M {edge} {} V {}\" stroke=\"#444\" stroke-width=\"0.18\"/>",
                        r.y,
                        r.y + r.height
                    )?;
                    for (i, &(primary, value)) in right.ticks.iter().enumerate() {
                        if primary < a.ylim[0] || primary > a.ylim[1] {
                            continue;
                        }
                        let [_, y] = p.xy([a.xlim[1], primary]);
                        write!(
                            s,
                            "<path d=\"M {edge} {y} h 1.2\" stroke=\"#444\" stroke-width=\"0.18\"/>"
                        )?;
                        let text = tick_label(value, &right.tick_labels, i);
                        let width = glyphs[&text].width_pt * MM_PER_PT;
                        tick_right = tick_right.max(edge + 0.6 * em + width);
                        place(
                            &mut s,
                            &glyphs,
                            &text,
                            edge + 0.6 * em + width / 2.,
                            y,
                            0.,
                            "#555555",
                            &mut serial,
                            &mut label_bounds,
                        )?;
                    }
                    let half_label = glyphs
                        .get(&right.label)
                        .map_or(0., |glyph| glyph.height_pt * MM_PER_PT / 2.);
                    let label_x = (edge + 3.8 * em).max(tick_right + 0.6 * em + half_label);
                    place(
                        &mut s,
                        &glyphs,
                        &right.label,
                        label_x,
                        r.y + r.height / 2.,
                        -90.,
                        "#555555",
                        &mut serial,
                        &mut label_bounds,
                    )?;
                }
                axis_bounds.extend(label_bounds[axis_start..].iter().cloned());
            }
            for l in &p.labels {
                place_background(&mut s, &glyphs, l)?;
                place(
                    &mut s,
                    &glyphs,
                    &l.latex,
                    l.x,
                    l.y,
                    l.rotation,
                    &l.color,
                    &mut serial,
                    &mut label_bounds,
                )?;
            }
        }
        for c in &self.connectors {
            let a = self.panels[c.from_panel].xy(c.from);
            let b = self.panels[c.to_panel].xy(c.to);
            let mut path = format!("M {} {}", a[0], a[1]);
            for v in c.via.iter().chain(std::iter::once(&b)) {
                write!(path, " L {} {}", v[0], v[1])?;
            }
            write!(
                s,
                "<path data-role=\"panel-connector\" d=\"{path}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" stroke-linejoin=\"round\"/>",
                c.color,
                c.width_pt * MM_PER_PT
            )?;
        }
        for (bi, b) in self.colorbars.iter().enumerate() {
            let sc = &self.scales[b.scale];
            let r = b.rect;
            if sc.colors.listed || sc.colors.stops.len() > 256 {
                // A lookup-table strip preserves discrete colours exactly and
                // avoids huge PDF stitching functions (two stops per bin),
                // which some TeX/PDF consumers cannot import safely. Dense
                // continuous palettes also use one strip, not oversized PDF
                // stitching functions with hundreds of sub-functions.
                let n = sc.colors.stops.len() as u32;
                let mut strip = image::RgbaImage::new(
                    if b.horizontal { n } else { 1 },
                    if b.horizontal { 1 } else { n },
                );
                for i in 0..n {
                    let c = sc.colors.sample((i as f64 + 0.5) / n as f64);
                    let (x, y) = if b.horizontal { (i, 0) } else { (0, n - 1 - i) };
                    strip.put_pixel(x, y, image::Rgba([c[0], c[1], c[2], 255]));
                }
                self.embed_image(&mut s, r, strip)?;
            } else {
                write!(
                    s,
                    "<defs><linearGradient id=\"bar-{bi}\" x1=\"0\" y1=\"{}\" x2=\"{}\" y2=\"0\">",
                    if b.horizontal { "0" } else { "1" },
                    if b.horizontal { "1" } else { "0" }
                )?;
                let n = (sc.colors.stops.len() - 1).max(64);
                for i in 0..=n {
                    let t = i as f64 / n as f64;
                    let c = sc.colors.sample(t);
                    write!(
                        s,
                        "<stop offset=\"{t}\" stop-color=\"rgb({},{},{})\"/>",
                        c[0], c[1], c[2]
                    )?;
                }
                write!(
                    s,
                    "</linearGradient></defs><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"url(#bar-{bi})\"/>",
                    r.x, r.y, r.width, r.height
                )?;
            }
            let em = self.font_pt * MM_PER_PT;
            for v in &b.ticks {
                if *v < sc.limits[0] || *v > sc.limits[1] {
                    continue;
                }
                let [a, z] = sc.limits.map(|x| sc.transform(x));
                let t = (sc.transform(*v) - a) / (z - a);
                let (x, y) = if b.horizontal {
                    (r.x + t * r.width, r.y + r.height + 1.3 * em)
                } else {
                    (r.x + r.width + 1.4 * em, r.y + (1.0 - t) * r.height)
                };
                place(
                    &mut s,
                    &glyphs,
                    &number(*v),
                    x,
                    y,
                    0.0,
                    "#171717",
                    &mut serial,
                    &mut label_bounds,
                )?;
            }
            let (x, y, rotation) = if b.horizontal {
                (r.x + r.width / 2.0, r.y + r.height + 2.9 * em, 0.0)
            } else {
                (r.x + r.width + 3.4 * em, r.y + r.height / 2.0, -90.0)
            };
            place(
                &mut s,
                &glyphs,
                &b.label,
                x,
                y,
                rotation,
                "#171717",
                &mut serial,
                &mut label_bounds,
            )?;
        }
        for l in &self.labels {
            place_background(&mut s, &glyphs, l)?;
            place(
                &mut s,
                &glyphs,
                &l.latex,
                l.x,
                l.y,
                l.rotation,
                &l.color,
                &mut serial,
                &mut label_bounds,
            )?;
        }
        // Place legends after all labels so both their keys and their white
        // backgrounds avoid annotations, secondary axes and adjacent content.
        let mut reserved_legends = Vec::new();
        for p in &self.panels {
            if let Some(legend) = &p.legend {
                let r = self.render_legend(
                    &mut s,
                    p,
                    legend,
                    glyphs,
                    &mut serial,
                    &mut label_bounds,
                    &reserved_legends,
                )?;
                reserved_legends.push(r);
            }
        }
        let intersects = |b: [f64; 4], r: Rect| {
            b[2].min(r.x + r.width) - b[0].max(r.x) > 0.05
                && b[3].min(r.y + r.height) - b[1].max(r.y) > 0.05
        };
        let mut warnings = Vec::new();
        for (label, bounds) in &label_bounds {
            for (i, bar) in self.colorbars.iter().enumerate() {
                if intersects(*bounds, bar.rect) {
                    warnings.push(format!("Label {label} overlaps colorbar {i}; move the colorbar or increase its margin."));
                }
            }
        }
        for (label, bounds) in &axis_bounds {
            for (i, panel) in self.panels.iter().enumerate() {
                if intersects(*bounds, panel.rect) {
                    warnings.push(format!(
                        "Axis label {label} overlaps data panel {i}; increase the panel spacing."
                    ));
                }
            }
        }
        for warning in &warnings {
            eprintln!("rustplotlib layout warning: {warning}");
        }
        ensure!(
            !self.check_label_geometry || warnings.is_empty(),
            "Unsafe layout: {} Explicitly set check_label_geometry=false only to force intentional overlaps.",
            warnings.join(" ")
        );
        if self.check_label_bounds {
            for (label, b) in &label_bounds {
                ensure!(
                    b[0] >= 0.0 && b[1] >= 0.0 && b[2] <= self.width_mm && b[3] <= self.height_mm,
                    "Label would be cropped: {label}; bounds {b:?}. Enlarge margins or reposition it."
                );
            }
        }
        if self.check_label_overlap {
            for i in 0..label_bounds.len() {
                for j in i + 1..label_bounds.len() {
                    let (a, b) = (&label_bounds[i], &label_bounds[j]);
                    let overlap_x = a.1[2].min(b.1[2]) - a.1[0].max(b.1[0]);
                    let overlap_y = a.1[3].min(b.1[3]) - a.1[1].max(b.1[1]);
                    ensure!(
                        overlap_x <= 0.05 || overlap_y <= 0.05,
                        "Labels overlap: {} and {}. Increase spacing or explicitly allow overlap.",
                        a.0,
                        b.0
                    );
                }
            }
        }
        s.push_str("</svg>");
        Ok(s)
    }
    fn image(
        &self,
        s: &mut String,
        p: &Panel,
        sample: impl Fn(f64, f64) -> Option<[u8; 4]>,
    ) -> Result<()> {
        let r = p.rect;
        let w = (r.width * self.raster_dpi / 25.4).ceil().max(1.0) as u32;
        let h = (r.height * self.raster_dpi / 25.4).ceil().max(1.0) as u32;
        let mut img = image::RgbaImage::new(w, h);
        for j in 0..h {
            for i in 0..w {
                let [xx, yy] = p.data_fraction(
                    (i as f64 + 0.5) / w as f64,
                    1. - (j as f64 + 0.5) / h as f64,
                );
                img.put_pixel(i, j, image::Rgba(sample(xx, yy).unwrap_or([0, 0, 0, 0])));
            }
        }
        self.embed_image(s, r, img)
    }
    fn embed_image(&self, s: &mut String, r: Rect, img: image::RgbaImage) -> Result<()> {
        let mut bytes = Cursor::new(vec![]);
        img.write_to(&mut bytes, image::ImageFormat::Png)?;
        write!(
            s,
            "<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"none\" href=\"data:image/png;base64,{}\"/>",
            r.x,
            r.y,
            r.width,
            r.height,
            STANDARD.encode(bytes.into_inner())
        )?;
        Ok(())
    }
    #[cfg(feature = "export")]
    pub(crate) fn tight_svg(&self, svg: &str) -> Result<String> {
        ensure!(
            self.export_padding_mm.is_finite() && self.export_padding_mm >= 0.,
            "Invalid export padding"
        );
        let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default())?;
        // Measure ink, including raster alpha and vector glyphs, at 150 dpi.
        let scale = 150. / 96.;
        let w = (tree.size().width() as f64 * scale).ceil() as u32;
        let h = (tree.size().height() as f64 * scale).ceil() as u32;
        let mut pix = resvg::tiny_skia::Pixmap::new(w, h).context("Invalid crop dimensions")?;
        pix.fill(resvg::tiny_skia::Color::WHITE);
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale as f32, scale as f32),
            &mut pix.as_mut(),
        );
        let mut bounds = [w, h, 0, 0];
        let mut found = false;
        for (i, p) in pix.data().chunks_exact(4).enumerate() {
            if p[..3].iter().any(|&v| v < 250) {
                let x = i as u32 % w;
                let y = i as u32 / w;
                bounds[0] = bounds[0].min(x);
                bounds[1] = bounds[1].min(y);
                bounds[2] = bounds[2].max(x + 1);
                bounds[3] = bounds[3].max(y + 1);
                found = true;
            }
        }
        if !found {
            return Ok(svg.to_string());
        }
        let mm = 25.4 / 150.;
        let pad = self.export_padding_mm;
        let x = (bounds[0] as f64 * mm - pad).max(0.);
        let y = (bounds[1] as f64 * mm - pad).max(0.);
        let width = (bounds[2] as f64 * mm + pad).min(self.width_mm) - x;
        let height = (bounds[3] as f64 * mm + pad).min(self.height_mm) - y;
        let end = svg.find('>').context("Missing SVG root")?;
        Ok(format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{width}mm\" height=\"{height}mm\" viewBox=\"{x} {y} {width} {height}\">{}",
            &svg[end + 1..]
        ))
    }
    #[cfg(feature = "export")]
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let raw_svg = self.svg()?;
        let svg = if self.tight_export {
            self.tight_svg(&raw_svg)?
        } else {
            raw_svg
        };
        match path
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_lowercase()
            .as_str()
        {
            "svg" => std::fs::write(path, svg)?,
            "pdf" => {
                // Parse physical SVG lengths at PDF's 72 dpi. Scaling a 96 dpi
                // tree through PageOptions mis-scales gradient coordinates.
                let options = svg2pdf::usvg::Options {
                    dpi: 72.0,
                    ..svg2pdf::usvg::Options::default()
                };
                let tree = svg2pdf::usvg::Tree::from_str(&svg, &options)?;
                let pdf = svg2pdf::to_pdf(
                    &tree,
                    svg2pdf::ConversionOptions::default(),
                    svg2pdf::PageOptions { dpi: 72.0 },
                )
                .map_err(|e| anyhow::anyhow!("PDF conversion failed: {e:?}"))?;
                std::fs::write(path, pdf)?;
            }
            "png" | "jpg" | "jpeg" => {
                let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default())?;
                let w = (tree.size().width() as f64 * self.raster_dpi / 96.0).round() as u32;
                let h = (tree.size().height() as f64 * self.raster_dpi / 96.0).round() as u32;
                let mut pix =
                    resvg::tiny_skia::Pixmap::new(w, h).context("Invalid export dimensions")?;
                pix.fill(resvg::tiny_skia::Color::WHITE);
                resvg::render(
                    &tree,
                    resvg::tiny_skia::Transform::from_scale(
                        w as f32 / tree.size().width(),
                        h as f32 / tree.size().height(),
                    ),
                    &mut pix.as_mut(),
                );
                let raw = image::RgbaImage::from_raw(w, h, pix.data().to_vec()).unwrap();
                if path.extension().unwrap().to_string_lossy().to_lowercase() == "png" {
                    let mut encoder = png::Encoder::new(std::fs::File::create(path)?, w, h);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    let ppm = (self.raster_dpi / 0.0254).round() as u32;
                    encoder.set_pixel_dims(Some(png::PixelDimensions {
                        xppu: ppm,
                        yppu: ppm,
                        unit: png::Unit::Meter,
                    }));
                    encoder.write_header()?.write_image_data(raw.as_raw())?;
                } else {
                    let rgb = image::DynamicImage::ImageRgba8(raw).to_rgb8();
                    let mut file = std::fs::File::create(path)?;
                    let mut encoder =
                        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, 95);
                    encoder.set_pixel_density(image::codecs::jpeg::PixelDensity::dpi(
                        self.raster_dpi.round().min(65535.0) as u16,
                    ));
                    encoder.encode_image(&rgb)?;
                }
            }
            _ => anyhow::bail!("Supported formats: PDF, PNG, JPEG, SVG"),
        };
        Ok(())
    }
}
fn rgb(c: [u8; 4]) -> String {
    format!("rgb({},{},{})", c[0], c[1], c[2])
}
fn place_background(s: &mut String, glyphs: &BTreeMap<String, Glyph>, label: &Label) -> Result<()> {
    if let Some(anchor) = label.leader {
        let glyph = &glyphs[&label.latex];
        let angle = label.rotation.to_radians();
        let dx = anchor[0] - label.x;
        let dy = anchor[1] - label.y;
        let local = [
            dx * angle.cos() + dy * angle.sin(),
            -dx * angle.sin() + dy * angle.cos(),
        ];
        let half = [
            glyph.width_pt * MM_PER_PT / 2. + 0.6,
            glyph.height_pt * MM_PER_PT / 2. + 0.6,
        ];
        let ratio = (local[0].abs() / half[0]).max(local[1].abs() / half[1]);
        let t = 1. / ratio.max(1.);
        let end = [label.x + t * dx, label.y + t * dy];
        write!(
            s,
            "<path data-role=\"label-leader\" d=\"M {} {} L {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"0.15\"/>",
            anchor[0], anchor[1], end[0], end[1], label.color
        )?;
    }
    if let Some(color) = &label.background {
        if label.latex.is_empty() {
            return Ok(());
        }
        let glyph = &glyphs[&label.latex];
        let w = glyph.width_pt * MM_PER_PT + 2. * label.padding_mm;
        let h = glyph.height_pt * MM_PER_PT + 2. * label.padding_mm;
        write!(
            s,
            "<rect data-role=\"label-backing\" x=\"{}\" y=\"{}\" width=\"{w}\" height=\"{h}\" transform=\"rotate({} {} {})\" fill=\"{color}\"/>",
            label.x - w / 2.,
            label.y - h / 2.,
            label.rotation,
            label.x,
            label.y
        )?;
    }
    Ok(())
}

fn place(
    s: &mut String,
    glyphs: &BTreeMap<String, Glyph>,
    label: &str,
    x: f64,
    y: f64,
    rotation: f64,
    color: &str,
    serial: &mut usize,
    boxes: &mut Vec<(String, [f64; 4])>,
) -> Result<()> {
    if label.is_empty() {
        return Ok(());
    }
    let g = &glyphs[label];
    let prefix = format!("glyph-{serial}-");
    *serial += 1;
    let body = g
        .body
        .replace("id='", &format!("id='{prefix}"))
        .replace("href='#", &format!("href='#{prefix}"));
    let w = g.width_pt * MM_PER_PT;
    let h = g.height_pt * MM_PER_PT;
    let angle = rotation.to_radians();
    let half_w = (w * angle.cos().abs() + h * angle.sin().abs()) / 2.0;
    let half_h = (w * angle.sin().abs() + h * angle.cos().abs()) / 2.0;
    boxes.push((
        label.to_string(),
        [x - half_w, y - half_h, x + half_w, y + half_h],
    ));
    write!(
        s,
        "<g transform=\"translate({x} {y}) rotate({rotation})\" fill=\"{color}\"><svg x=\"{}\" y=\"{}\" width=\"{w}\" height=\"{h}\" viewBox=\"{} {} {} {}\" overflow=\"visible\">{body}</svg></g>",
        -w / 2.0,
        -h / 2.0,
        g.view_box[0],
        g.view_box[1],
        g.view_box[2],
        g.view_box[3]
    )?;
    Ok(())
}
fn increasing(v: &[f64]) -> bool {
    v.len() >= 2 && v.iter().all(|x| x.is_finite()) && v.windows(2).all(|w| w[1] > w[0])
}
fn bin(e: &[f64], x: f64) -> Option<usize> {
    if !x.is_finite() || x < e[0] || x > *e.last()? {
        return None;
    }
    Some(
        e.partition_point(|v| *v <= x)
            .saturating_sub(1)
            .min(e.len() - 2),
    )
}
pub fn ticks(limits: [f64; 2], count: usize) -> Vec<f64> {
    let span = limits[1] - limits[0];
    if !span.is_finite() || span <= 0.0 {
        return vec![];
    }
    let target = span / count.max(1) as f64;
    let base = 10f64.powf(target.log10().floor());
    let f = target / base;
    let nice = [1.0, 2.0, 2.5, 5.0, 10.0]
        .into_iter()
        .find(|v| *v >= f)
        .unwrap();
    let step = base * nice;
    let lo = (limits[0] / step).ceil() as i64;
    let hi = (limits[1] / step).floor() as i64;
    (lo..=hi)
        .map(|i| {
            let v = i as f64 * step;
            if v.abs() < step * 1e-10 { 0.0 } else { v }
        })
        .collect()
}
pub fn number(x: f64) -> String {
    if x == 0.0 {
        return "$0$".into();
    }
    let a = x.abs();
    if !(0.001..10000.0).contains(&a) {
        let exp = a.log10().floor() as i32;
        format!("${:.2}\\times10^{{{exp}}}$", x / 10f64.powi(exp))
    } else {
        let mut n = format!("{x:.5}");
        while n.ends_with('0') {
            n.pop();
        }
        if n.ends_with('.') {
            n.pop();
        }
        format!("${n}$")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorMap, Legend, LegendEntry, Normalization};
    #[test]
    fn shape_keys_render_with_matching_geometry_and_export_guard() {
        use crate::MarkerShape;
        for (shape, tag) in [
            (MarkerShape::Circle, "circle"),
            (MarkerShape::Square, "polygon"),
            (MarkerShape::Diamond, "polygon"),
            (MarkerShape::Triangle, "polygon"),
            (MarkerShape::HorizontalCap, "polyline"),
            (MarkerShape::VerticalCap, "polyline"),
        ] {
            let mut svg = String::new();
            render_legend_marker(&mut svg, shape, "#123456", 10., 20., 6., None).unwrap();
            assert!(svg.starts_with(&format!("<{tag} ")) && svg.contains("#123456"));
            assert!(!svg.contains("NaN"));
        }
        let mut f = Figure::new(IntendedUse::ArticleWide);
        let mut p = Panel::new(
            Rect::new(20., 20., 140., 70.),
            Axes::new([0., 1.], [0., 1.], "", ""),
        );
        let mut legend = Legend::new(vec![
            LegendEntry::marker("Optical", MarkerShape::Circle, "#444444"),
            LegendEntry::marker("Epoch", MarkerShape::Circle, "#111111"),
        ]);
        legend.avoid_data = false;
        p.legend = Some(legend);
        f.panels.push(p);
        assert!(
            f.svg()
                .unwrap_err()
                .to_string()
                .contains("Ambiguous legend")
        );
        f.panels[0].legend.as_mut().unwrap().entries[1] =
            LegendEntry::marker("Epoch", MarkerShape::Diamond, "#111111");
        let svg = f.svg().unwrap();
        let legend = svg.split("data-role=\"legend\"").nth(1).unwrap();
        assert!(legend.contains("<circle ") && legend.contains("<polygon "));
    }
    #[test]
    fn secondary_axis_reserves_measured_negative_tick_width() {
        let mut f = crate::ArticleLayout::new(184., 10.)
            .unwrap()
            .figure(1., 100.)
            .unwrap();
        let mut axes = Axes::new([0., 1.], [-1., 1.], "Time", "Residual");
        axes.secondary_y = Some(
            SecondaryAxis::new(
                r"Path-rate residual ($\mathrm{m\,s^{-1}}$)",
                vec![-1000., -500., 0., 500., 1000.],
                |v| v / 1000.,
            )
            .unwrap(),
        );
        f.panels
            .push(Panel::new(Rect::new(20., 15., 130., 60.), axes));
        assert!(f.svg().is_ok());
    }
    #[test]
    fn native_secondary_y_and_clock_labels_are_validated() {
        let mut f = Figure::new(IntendedUse::ArticleWide);
        let mut a = Axes::new([12., 24.], [0., 10.], "UTC", "Counts");
        a.secondary_x = Some(SecondaryAxis::new("Solar", vec![19., 21.], |v| v - 7.).unwrap());
        a.secondary_x.as_mut().unwrap().tick_labels = Some(vec!["19:00".into(), "21:00".into()]);
        a.secondary_y = Some(SecondaryAxis::new("Speed", vec![20., 40.], |v| v / 5.).unwrap());
        a.show_y_axis = false;
        f.panels.push(Panel::new(Rect::new(20., 20., 100., 50.), a));
        assert!(f.validate().is_ok());
        assert!(f.all_labels().contains(&"19:00".into()));
        assert!(f.all_labels().contains(&"Speed".into()));
        f.panels[0]
            .axes
            .as_mut()
            .unwrap()
            .secondary_x
            .as_mut()
            .unwrap()
            .tick_labels = Some(vec![]);
        assert!(f.validate().is_err());
    }
    #[test]
    fn native_contour_paths_preserve_all_segments() {
        let c = Contour::new(
            vec![-1., 0., 1.],
            vec![-1., 0., 1.],
            vec![-2., -1., 0., -1., 0., 1., 0., 1., 2.],
            vec![-0.5, 0.5],
            0,
            false,
        )
        .unwrap();
        let paths = c.paths(0.5);
        assert_eq!(
            paths.iter().map(|p| p.len() - 1).sum::<usize>(),
            c.segments(0.5).len()
        );
        assert_eq!(paths.len(), 1);
        for p in paths.into_iter().flatten() {
            assert!((p[0] + p[1] - 0.5).abs() < 1e-12);
        }
    }
    #[test]
    fn dense_native_colourbar_uses_one_texture_not_pdf_stitching() {
        let mut f = Figure::new(IntendedUse::ArticleWide);
        let scale =
            f.scale(Scale::new([15., 71.], Normalization::Linear, ColorMap::turbo()).unwrap());
        f.colorbars.push(Colorbar::new(
            Rect::new(20., 20., 3., 50.),
            scale,
            "Speed",
            vec![20., 40., 60.],
        ));
        let svg = f.svg().unwrap();
        assert!(svg.contains("data:image/png;base64,"));
        assert!(!svg.contains("linearGradient"));
    }
    #[test]
    fn logarithmic_axes_map_equal_decades_and_raster_inverse() {
        let mut axes = Axes::new([1., 1000.], [1., 1000.], "", "");
        axes.set_xscale(crate::AxisScale::Log10).unwrap();
        axes.set_yscale(crate::AxisScale::Log10).unwrap();
        let p = Panel::new(Rect::new(0., 0., 90., 90.), axes);
        assert_eq!(p.xy([10., 100.]), [30., 30.]);
        let sample = p.data_fraction(1. / 3., 2. / 3.);
        assert!((sample[0] - 10.).abs() < 1e-10 && (sample[1] - 100.).abs() < 1e-10);
    }
    #[test]
    fn polygon_cells_preserve_coverage_masks_and_winding() {
        let polygons = vec![
            vec![[0., 0.], [0.6, 0.], [0.4, 1.], [0., 1.]],
            vec![[0.6, 0.], [1., 0.], [1., 1.], [0.4, 1.]],
        ];
        let p = Panel::new(
            Rect::new(0., 0., 25.4, 25.4),
            Axes::new([0., 1.], [0., 1.], "", ""),
        );
        let scale = Scale::new([0., 1.], Normalization::Linear, ColorMap::seismic()).unwrap();
        let mut mesh = PolygonMesh::new(polygons, vec![0.5, 0.5], 0).unwrap();
        let a = raster_polygons(&mesh, &p, &scale, 128.);
        assert!(
            a.pixels().all(|v| v[3] == 255),
            "Adjacent polygons must not leave seams"
        );
        mesh.polygons[1].reverse();
        assert_eq!(
            a,
            raster_polygons(&mesh, &p, &scale, 128.),
            "Winding must not change coverage"
        );
        mesh.values[1] = f64::NAN;
        let b = raster_polygons(&mesh, &p, &scale, 128.);
        assert_eq!(b.get_pixel(115, 64)[3], 0);
        assert_eq!(b.get_pixel(12, 64)[3], 255);
        assert!(PolygonMesh::new(vec![vec![[0., 0.], [1., 0.]]], vec![1.], 0).is_err());
        assert!(PolygonMesh::new(vec![], vec![1.], 0).is_err());
    }
    #[test]
    fn custom_ticks_validate_shape_and_keep_text() {
        let mut f = Figure::new(IntendedUse::ArticleWide);
        let mut a = Axes::new([0., 24.], [0., 1.], "", "");
        a.xticks = vec![0., 12., 24.];
        a.xticklabels = Some(vec!["06".into(), "18".into(), "06".into()]);
        f.panels.push(Panel::new(Rect::new(10., 10., 100., 40.), a));
        assert!(f.validate().is_ok());
        assert!(f.all_labels().contains(&"06".into()));
        f.panels[0]
            .axes
            .as_mut()
            .unwrap()
            .xticklabels
            .as_mut()
            .unwrap()
            .pop();
        assert!(f.validate().is_err());
    }
    #[test]
    fn nonuniform_cells_missing_values_and_contours() {
        let f = Field::new(vec![0.0, 1.0, 3.0], vec![0.0, 2.0], vec![1.0, f64::NAN], 0).unwrap();
        assert_eq!(f.sample(0.5, 1.0), 1.0);
        assert!(f.sample(2.0, 1.0).is_nan());
        let c = Contour::new(
            vec![0.0, 2.0],
            vec![0.0, 1.0],
            vec![0.0, 2.0, 1.0, 3.0],
            vec![0.0, 1.5, 3.0],
            0,
            false,
        )
        .unwrap();
        assert!((c.sample(0.5, 0.25) - 0.75).abs() < 1e-12);
        for seg in c.segments(1.5) {
            for p in seg {
                assert!((p[0] + p[1] - 1.5).abs() < 1e-12)
            }
        }
    }
    #[test]
    fn shared_curvilinear_cells_have_no_internal_seams() {
        let mesh = QuadMesh::new(
            2,
            1,
            vec![[0., 0.], [0.6, 0.], [1., 0.], [0., 1.], [0.4, 1.], [1., 1.]],
            vec![0.5, 0.5],
            0,
        )
        .unwrap();
        let panel = Panel::new(
            Rect::new(0., 0., 25.4, 25.4),
            Axes::new([0., 1.], [0., 1.], "", ""),
        );
        let scale = Scale::new([0., 1.], Normalization::Linear, ColorMap::diverging()).unwrap();
        let image = raster_mesh(&mesh, &panel, &scale, 128.);
        assert!(image.pixels().all(|p| p[3] == 255));
        let mut masked = mesh.clone();
        masked.values[1] = f64::NAN;
        let image = raster_mesh(&masked, &panel, &scale, 128.);
        assert_eq!(image.get_pixel(115, 64)[3], 0);
        assert_eq!(image.get_pixel(12, 64)[3], 255);
    }
    #[test]
    fn secondary_axis_maps_reciprocal_period_and_rejects_singular_ticks() {
        let axis = SecondaryAxis::new("Period", vec![4., 5., 10.], |p| {
            std::f64::consts::TAU / (60. * p)
        })
        .unwrap();
        assert!((axis.ticks[1].0 - 0.020943951023931952).abs() < 1e-14);
        assert!(axis.ticks[0].0 > axis.ticks[2].0);
        assert!(SecondaryAxis::new("Period", vec![0.], |p| 1. / p).is_err());
    }
    #[test]
    fn profiles_are_readable_and_golden() {
        let f = Figure::new(IntendedUse::ArticleColumn);
        assert_eq!(f.font_pt, 10.0);
        assert!((f.width_mm / f.height_mm - PHI).abs() < 1e-12);
        assert!(Figure::new(IntendedUse::Mobile).font_pt >= 12.0);
        assert_eq!(ticks([0.0, 10.0], 5), vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
    }
    #[test]
    fn default_legend_avoids_scatter_and_errors_but_not_grid() {
        use crate::{Legend, LegendEntry};
        let f = Figure::new(IntendedUse::ArticleWide);
        let mut p = Panel::new(
            Rect::new(10., 10., 40., 40.),
            Axes::new([0., 1.], [0., 1.], "", ""),
        );
        p.major_grid("#dddddd", 0.4);
        let legend = Legend::new(vec![LegendEntry::circle("Data", "green")]);
        assert!(legend.avoid_data && legend.avoid_layers.is_empty());
        assert!(f.legend_occupancy(&p, &legend).unwrap().iter().all(|v| !v));
        let scale = Scale::new([0., 1.], Normalization::Linear, ColorMap::diverging()).unwrap();
        p.scatter_colored(&[[0.8, 0.8]], &[0.5], &scale, 5., 1., "none")
            .unwrap();
        p.errorbars(
            &[[0.8, 0.8]],
            Some(&[[0.55, 0.95]]),
            Some(&[[0.65, 0.95]]),
            "green",
            1.,
            3.,
            1.,
        )
        .unwrap();
        let mask = f.legend_occupancy(&p, &legend).unwrap();
        assert!(mask[32 * 160 + 128], "Scatter centre must be protected");
        assert!(mask[32 * 160 + 96], "Errorbar stem must be protected");
        let occupied = |r: Rect| {
            mask.iter().enumerate().any(|(i, &value)| {
                let x = 10. + (i % 160) as f64 / 4.;
                let y = 10. + (i / 160) as f64 / 4.;
                value && x >= r.x && x <= r.x + r.width && y >= r.y && y <= r.y + r.height
            })
        };
        let preferred = Rect::new(33., 12., 15., 9.);
        assert!(occupied(preferred));
        let placed = legend.place(p.rect, 15., 9., &occupied).unwrap();
        assert!(!occupied(placed));
        assert!(placed.x != preferred.x || placed.y != preferred.y);
    }

    #[test]
    fn legend_protects_visible_cells_but_can_cover_background() {
        use crate::{Legend, LegendEntry};
        let mut f = Figure::new(IntendedUse::ArticleWide);
        f.scale(Scale::new([0., 1.], Normalization::Linear, ColorMap::diverging()).unwrap());
        let mut p = Panel::new(
            Rect::new(10., 20., 40., 40.),
            Axes::new([0., 1.], [0., 1.], "", ""),
        );
        p.push(Layer::Image(image::RgbaImage::from_pixel(
            2,
            2,
            image::Rgba([100, 100, 100, 255]),
        )));
        p.push(Layer::PolygonMesh(
            PolygonMesh::new(
                vec![
                    vec![[0., 0.], [0.5, 0.], [0.5, 1.], [0., 1.]],
                    vec![[0.5, 0.], [1., 0.], [1., 1.], [0.5, 1.]],
                ],
                vec![0.5, f64::NAN],
                0,
            )
            .unwrap(),
        ));
        let mut legend = Legend::new(vec![LegendEntry::circle("Data", "green")]);
        legend.avoid_layers = vec![1];
        let mask = f.legend_occupancy(&p, &legend).unwrap();
        assert!(mask[80 * 160 + 40]);
        assert!(
            !mask[80 * 160 + 120],
            "Masked data must not protect the cloud background"
        );
        legend.avoid_layers = vec![0];
        assert!(f.legend_occupancy(&p, &legend).unwrap().iter().all(|p| *p));
        legend.avoid_layers = vec![99];
        assert!(legend.validate(&p).is_err());
        legend.avoid_layers = vec![1];
        p.legend = Some(legend);
        f.panels.push(p);
        assert!(f.validate().is_ok());
        assert!(f.all_labels().contains(&"Data".into()));
    }

    #[test]
    fn filled_vector_path_preserves_transparency() {
        let mut f = Figure::new(IntendedUse::ArticleWide);
        let mut p = Panel::new(
            Rect::new(10., 10., 40., 40.),
            Axes::new([0., 1.], [0., 1.], "", ""),
        );
        p.axes_visible = false;
        let mut band = Line::new(vec![[0., 0.], [1., 0.], [1., 1.], [0., 0.]], "#123456");
        band.fill = Some("#abcdef".into());
        band.fill_opacity = 0.25;
        band.width_pt = 0.;
        p.push(Layer::Line(band));
        f.panels.push(p);
        let svg = f.svg().unwrap();
        assert!(svg.contains("fill=\"#abcdef\" fill-opacity=\"0.25\""));
        let legend = Legend::new(vec![LegendEntry::line("Data", "black")]);
        let occupancy = f.legend_occupancy(&f.panels[0], &legend).unwrap();
        assert!(occupancy[120 * 160 + 120]);
        if let Layer::Line(band) = &mut f.panels[0].layers[0] {
            band.fill_opacity = 1.1;
        }
        assert!(f.validate().is_err());
    }

    #[test]
    fn line_opacity_validates_exports_and_respects_invisible_data() {
        let mut f = Figure::new(IntendedUse::ArticleWide);
        let mut p = Panel::new(
            Rect::new(10., 10., 40., 40.),
            Axes::new([0., 1.], [0., 1.], "", ""),
        );
        p.axes_visible = false;
        let mut l = Line::new(vec![[0., 0.], [1., 1.]], "#333333");
        l.opacity = 0.2;
        p.push(Layer::Line(l));
        f.panels.push(p);
        assert!(f.svg().unwrap().contains("stroke-opacity=\"0.2\""));
        let legend = Legend::new(vec![LegendEntry::line("Ensemble", "#333333")]);
        if let Layer::Line(l) = &mut f.panels[0].layers[0] {
            l.opacity = 0.;
        }
        assert!(
            f.legend_occupancy(&f.panels[0], &legend)
                .unwrap()
                .iter()
                .all(|x| !x)
        );
        if let Layer::Line(l) = &mut f.panels[0].layers[0] {
            l.opacity = 1.1;
        }
        assert!(f.validate().is_err());
    }

    #[test]
    fn compact_legend_fills_rows_and_keeps_labels_apart() {
        let f = Figure::new(IntendedUse::ArticleWide);
        let p = Panel::new(
            Rect::new(10., 10., 100., 70.),
            Axes::new([0., 1.], [0., 1.], "", ""),
        );
        let mut legend = Legend::new(vec![
            LegendEntry::line("A", "red"),
            LegendEntry::line("B", "blue"),
            LegendEntry::line("C", "green"),
        ]);
        legend.columns = 2;
        legend.avoid_data = false;
        let glyphs = ["A", "B", "C"]
            .into_iter()
            .map(|s| {
                (
                    s.to_string(),
                    Glyph {
                        body: String::new(),
                        view_box: [0., 0., 10., 5.],
                        width_pt: 10.,
                        height_pt: 5.,
                    },
                )
            })
            .collect();
        let mut svg = String::new();
        let mut bounds = Vec::new();
        f.render_legend(&mut svg, &p, &legend, &glyphs, &mut 0, &mut bounds, &[])
            .unwrap();
        assert_eq!(bounds.len(), 3);
        assert_eq!(bounds[0].1[1], bounds[1].1[1]);
        assert!(bounds[1].1[0] > bounds[0].1[2]);
        assert!(bounds[2].1[1] > bounds[0].1[3]);
        assert_eq!(bounds[0].1[0], bounds[2].1[0]);
        legend.columns = 0;
        assert!(legend.validate(&p).is_err());
    }

    #[cfg(feature = "export")]
    #[test]
    fn rasterized_lines_preserve_transparency_and_layer_order() {
        let mut p = Panel::new(
            Rect::new(20., 30., 25.4, 25.4),
            Axes::new([0., 1.], [0., 1.], "", ""),
        );
        let mut line = Line::new(vec![[0., 0.5], [1., 0.5]], "#808080");
        line.opacity = 0.5;
        line.width_pt = 4.;
        p.push(Layer::Line(line.clone()));
        p.push(Layer::Line(line.clone()));
        p.push(Layer::Line(line));
        p.rasterize_lines(0..2, 100.).unwrap();
        assert_eq!(p.layers.len(), 2);
        assert!(matches!(p.layers[1], Layer::Line(_)));
        let Layer::Image(image) = &p.layers[0] else {
            panic!("expected rasterized ensemble");
        };
        assert_eq!(image.dimensions(), (100, 100));
        let pixel = image.get_pixel(50, 50);
        assert!(
            (pixel[0] as i16 - 128).abs() <= 1,
            "Must unpremultiply RGB before PNG encoding"
        );
        assert!(
            (pixel[3] as i16 - 192).abs() <= 1,
            "Overlapping curves accumulate opacity"
        );
        assert_eq!(image.get_pixel(0, 0)[3], 0);
        assert!(p.rasterize_lines(0..1, 100.).is_err());
    }
}
