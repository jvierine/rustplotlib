//! Compact, LaTeX-typeset legends with explicitly protected data layers.
use crate::{MarkerShape, Panel, Rect};
use anyhow::{Result, ensure};

#[derive(Clone, Copy, Debug)]
pub enum LegendPosition {
    UpperLeft,
    UpperRight,
    LowerLeft,
    LowerRight,
    CenterRight,
    /// Prefer top-center inside the axes, with the normal outside fallback.
    AboveCenter,
    /// Explicitly request placement above the axes, skipping the inside search.
    OutsideAboveCenter,
}

#[derive(Clone, Debug)]
pub enum LegendSymbol {
    Line {
        color: String,
        width_pt: f64,
        dash: Option<String>,
    },
    /// A key matching a plotted `MarkerStyle`; size is the full diameter.
    Marker {
        shape: MarkerShape,
        color: String,
        size_pt: f64,
    },
    Circle {
        color: String,
        radius_pt: f64,
    },
}

#[derive(Clone, Debug)]
pub struct LegendEntry {
    pub label: String,
    pub symbol: LegendSymbol,
}
impl LegendEntry {
    pub fn line(label: impl Into<String>, color: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            symbol: LegendSymbol::Line {
                color: color.into(),
                width_pt: 1.5,
                dash: None,
            },
        }
    }
    pub fn marker(label: impl Into<String>, shape: MarkerShape, color: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            symbol: LegendSymbol::Marker {
                shape,
                color: color.into(),
                size_pt: 5.2,
            },
        }
    }
    pub fn circle(label: impl Into<String>, color: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            symbol: LegendSymbol::Circle {
                color: color.into(),
                radius_pt: 2.6,
            },
        }
    }
}

/// A frameless white legend. `position` is preferred, not forced, when data
/// avoidance is enabled. By default, visible data are protected while images
/// and major grids are contextual backgrounds. Nonempty `avoid_layers` explicitly
/// selects protected layers, overriding those defaults. If no clear
/// placement exists inside, try clear space outside the axes. Export fails only
/// when neither placement can avoid protected content within the figure.
#[derive(Clone, Debug)]
pub struct Legend {
    pub entries: Vec<LegendEntry>,
    /// Preferred number of columns; alternate counts are tried before moving
    /// outside. Entries fill rows left-to-right; widths follow their labels.
    pub columns: usize,
    pub position: LegendPosition,
    /// Explicit opt-out for categories intentionally distinguished only by colour.
    /// Otherwise different labels must use different marker shapes, including
    /// legacy `Circle` keys. Change the plotted markers as well as their keys.
    pub allow_reused_marker_shapes: bool,
    pub avoid_data: bool,
    pub avoid_layers: Vec<usize>,
    pub padding_mm: f64,
    pub background_opacity: f64,
}
impl Legend {
    pub fn new(entries: Vec<LegendEntry>) -> Self {
        Self {
            entries,
            columns: 1,
            position: LegendPosition::UpperRight,
            allow_reused_marker_shapes: false,
            avoid_data: true,
            avoid_layers: vec![],
            padding_mm: 2.,
            background_opacity: 0.96,
        }
    }

    pub(crate) fn validate(&self, panel: &Panel) -> Result<()> {
        ensure!(!self.entries.is_empty(), "Empty legend");
        ensure!(
            self.columns > 0 && self.columns <= self.entries.len(),
            "Invalid legend column count"
        );
        ensure!(
            self.padding_mm.is_finite() && self.padding_mm >= 0.,
            "Invalid legend padding"
        );
        ensure!(
            self.background_opacity.is_finite() && (0.0..=1.0).contains(&self.background_opacity),
            "Invalid legend opacity"
        );
        ensure!(
            self.avoid_layers.iter().all(|i| *i < panel.layers.len()),
            "Unknown protected legend layer"
        );
        let mut marker_keys: Vec<(MarkerShape, &str)> = vec![];
        for entry in &self.entries {
            ensure!(!entry.label.is_empty(), "Empty legend label");
            let size = match &entry.symbol {
                LegendSymbol::Line { width_pt, .. } => *width_pt,
                LegendSymbol::Circle { radius_pt, .. } => *radius_pt,
                LegendSymbol::Marker { size_pt, .. } => *size_pt,
            };
            ensure!(size.is_finite() && size > 0., "Invalid legend symbol size");
            let shape = match &entry.symbol {
                LegendSymbol::Circle { .. } => Some(MarkerShape::Circle),
                LegendSymbol::Marker { shape, .. } => Some(*shape),
                LegendSymbol::Line { .. } => None,
            };
            if let Some(shape) = shape {
                if !self.allow_reused_marker_shapes {
                    for &(previous_shape, previous_label) in &marker_keys {
                        ensure!(
                            previous_shape != shape || previous_label == entry.label,
                            "Ambiguous legend: '{}' and '{}' both use {:?} markers. Use distinct MarkerShape values in the plotted data and LegendEntry::marker keys, or explicitly set allow_reused_marker_shapes = true for intentional colour-only categories",
                            previous_label,
                            entry.label,
                            shape
                        );
                    }
                }
                marker_keys.push((shape, &entry.label));
            }
        }
        Ok(())
    }

    /// Search inside first; the renderer additionally tries alternate column
    /// counts before calling `place_outside`. `occupied` includes label geometry;
    /// the caller omits data from it when `avoid_data` is disabled.
    #[cfg(test)]
    pub(crate) fn place(
        &self,
        panel: Rect,
        width: f64,
        height: f64,
        occupied: &dyn Fn(Rect) -> bool,
    ) -> Result<Rect> {
        self.place_inside(panel, width, height, occupied)
            .or_else(|| self.place_outside(panel, width, height, occupied))
            .ok_or_else(|| anyhow::anyhow!("No legend placement avoids protected content"))
    }

    pub(crate) fn place_inside(
        &self,
        panel: Rect,
        width: f64,
        height: f64,
        occupied: &dyn Fn(Rect) -> bool,
    ) -> Option<Rect> {
        if matches!(self.position, LegendPosition::OutsideAboveCenter) {
            return None;
        }
        let inset = 2.;
        let left = panel.x + inset;
        let right = panel.x + panel.width - inset - width;
        let top = panel.y + inset;
        let bottom = panel.y + panel.height - inset - height;
        if right < left || bottom < top {
            return None;
        }
        let preferred = match self.position {
            LegendPosition::UpperLeft => [left, top],
            LegendPosition::UpperRight => [right, top],
            LegendPosition::LowerLeft => [left, bottom],
            LegendPosition::LowerRight => [right, bottom],
            LegendPosition::CenterRight => [right, (top + bottom) / 2.],
            LegendPosition::AboveCenter => [(left + right) / 2., top],
            LegendPosition::OutsideAboveCenter => unreachable!(),
        };
        let candidates = [
            preferred,
            [left, top],
            [right, top],
            [left, bottom],
            [right, bottom],
        ];
        for [x, y] in candidates {
            let r = Rect::new(x, y, width, height);
            if !occupied(r) {
                return Some(r);
            }
        }
        // At least the occupancy mask's 0.25 mm resolution: a narrow clear
        // pocket should not be missed just because the panel is large.
        let nx = ((right - left) / 0.25).ceil().max(1.) as usize;
        let ny = ((bottom - top) / 0.25).ceil().max(1.) as usize;
        let mut interior = Vec::with_capacity((nx + 1) * (ny + 1));
        for j in 0..=ny {
            for i in 0..=nx {
                interior.push([
                    left + (right - left) * i as f64 / nx as f64,
                    top + (bottom - top) * j as f64 / ny as f64,
                ]);
            }
        }
        interior.sort_by(|a, b| {
            let d = |p: &[f64; 2]| (p[0] - preferred[0]).powi(2) + (p[1] - preferred[1]).powi(2);
            d(a).total_cmp(&d(b))
        });
        interior
            .into_iter()
            .map(|[x, y]| Rect::new(x, y, width, height))
            .find(|&r| !occupied(r))
    }

    pub(crate) fn place_outside(
        &self,
        panel: Rect,
        width: f64,
        height: f64,
        occupied: &dyn Fn(Rect) -> bool,
    ) -> Option<Rect> {
        let x = panel.x + (panel.width - width) / 2.;
        let y = panel.y + (panel.height - height) / 2.;
        // The callback checks the figure boundary, labels, neighboring panels,
        // colorbars and other legends. Never silently cover them as a fallback.
        for step in 0..=200 {
            let gap = 2. + step as f64 * 0.5;
            let mut candidates = vec![Rect::new(x, panel.y - height - gap, width, height)];
            if !matches!(self.position, LegendPosition::OutsideAboveCenter) {
                candidates.extend([
                    Rect::new(panel.x + panel.width + gap, y, width, height),
                    Rect::new(panel.x - width - gap, y, width, height),
                    Rect::new(x, panel.y + panel.height + gap, width, height),
                ]);
            }
            if let Some(r) = candidates.into_iter().find(|&r| !occupied(r)) {
                return Some(r);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_panel() -> Panel {
        Panel::new(
            Rect::new(0., 0., 80., 60.),
            crate::Axes::new([0., 1.], [0., 1.], "x", "y"),
        )
    }
    #[test]
    fn rejects_same_shape_even_with_different_colours_and_sizes() {
        let legend = Legend::new(vec![
            LegendEntry::circle("Optical", "#444444"),
            LegendEntry::marker("Fitted epoch state", MarkerShape::Circle, "#111111"),
        ]);
        let error = legend.validate(&test_panel()).unwrap_err().to_string();
        assert!(error.contains("Optical") && error.contains("Fitted epoch state"));
        assert!(error.contains("Ambiguous legend"));
        let legacy = Legend::new(vec![
            LegendEntry::circle("A", "gray"),
            LegendEntry::circle("B", "black"),
        ]);
        assert!(legacy.validate(&test_panel()).is_err());
    }
    #[test]
    fn distinct_shapes_pass_and_intentional_reuse_requires_opt_out() {
        let mut legend = Legend::new(vec![
            LegendEntry::circle("Optical", "gray"),
            LegendEntry::marker("Fitted epoch state", MarkerShape::Diamond, "black"),
            LegendEntry::marker("TLE epoch", MarkerShape::Square, "purple"),
        ]);
        legend.validate(&test_panel()).unwrap();
        legend
            .entries
            .push(LegendEntry::circle("Other optical", "red"));
        assert!(legend.validate(&test_panel()).is_err());
        legend.allow_reused_marker_shapes = true;
        legend.validate(&test_panel()).unwrap();
    }
    #[test]
    fn above_center_searches_inside_before_falling_back() {
        let mut legend = Legend::new(vec![LegendEntry::line("Model", "blue")]);
        legend.position = LegendPosition::AboveCenter;
        let p = Rect::new(20., 40., 100., 60.);
        let inside = legend.place(p, 30., 10., &|_| false).unwrap();
        assert_eq!([inside.x, inside.y], [55., 42.]);
        let outside = legend
            .place(p, 30., 10., &|r| r.y + r.height > p.y)
            .unwrap();
        assert!(outside.y + outside.height < p.y);
    }

    #[test]
    fn finds_small_empty_pocket_between_standard_positions() {
        let legend = Legend::new(vec![LegendEntry::line("Model", "blue")]);
        let p = Rect::new(0., 0., 100., 80.);
        let empty = |r: Rect| (r.x - 43.125).abs() < 0.2 && (r.y - 37.125).abs() < 0.2;
        let r = legend.place_inside(p, 20., 10., &|r| !empty(r)).unwrap();
        assert!(empty(r));
    }

    #[test]
    fn compact_external_legend_leaves_the_data_panel_clear() {
        let mut legend = Legend::new(vec![LegendEntry::line("Receiver", "blue")]);
        legend.position = LegendPosition::OutsideAboveCenter;
        let r = legend
            .place(Rect::new(10., 20., 50., 60.), 40., 6., &|r| r.y >= 20.)
            .unwrap();
        assert_eq!(r.x, 15.);
        assert!(r.y + r.height < 20.);
    }
    #[test]
    fn prefers_requested_corner_but_moves_away_from_data() {
        let mut legend = Legend::new(vec![LegendEntry::line("Model", "blue")]);
        legend.position = LegendPosition::UpperLeft;
        let panel = Rect::new(10., 20., 100., 70.);
        let free = legend.place(panel, 30., 12., &|_| false).unwrap();
        assert_eq!([free.x, free.y], [12., 22.]);
        let moved = legend.place(panel, 30., 12., &|r| r.x < 55.).unwrap();
        assert!(moved.x >= 55.);
        assert!(legend.place(panel, 30., 12., &|_| true).is_err());
        legend.avoid_data = false;
        assert_eq!(legend.place(panel, 30., 12., &|_| false).unwrap().x, 12.);
        assert!(legend.place_inside(panel, 200., 12., &|_| false).is_none());
    }
}
