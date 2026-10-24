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
    AboveCenter,
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
/// placement exists, export fails rather than hiding the protected data.
#[derive(Clone, Debug)]
pub struct Legend {
    pub entries: Vec<LegendEntry>,
    /// Entries fill rows left-to-right. Column widths follow their labels.
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

    pub(crate) fn place(
        &self,
        panel: Rect,
        width: f64,
        height: f64,
        occupied: &dyn Fn(Rect) -> bool,
    ) -> Result<Rect> {
        let inset = 2.;
        if matches!(self.position, LegendPosition::AboveCenter) {
            ensure!(width <= panel.width, "External legend wider than panel");
            return Ok(Rect::new(
                panel.x + (panel.width - width) / 2.,
                panel.y - height - 2.,
                width,
                height,
            ));
        }
        let left = panel.x + inset;
        let right = panel.x + panel.width - inset - width;
        let top = panel.y + inset;
        let bottom = panel.y + panel.height - inset - height;
        ensure!(
            right >= left && bottom >= top,
            "Legend does not fit inside panel"
        );
        let preferred = match self.position {
            LegendPosition::UpperLeft => [left, top],
            LegendPosition::UpperRight => [right, top],
            LegendPosition::LowerLeft => [left, bottom],
            LegendPosition::LowerRight => [right, bottom],
            LegendPosition::CenterRight => [right, (top + bottom) / 2.],
            LegendPosition::AboveCenter => unreachable!(),
        };
        let mut candidates = vec![
            preferred,
            [left, top],
            [right, top],
            [left, bottom],
            [right, bottom],
        ];
        let mut interior = vec![];
        for j in 0..=24 {
            for i in 0..=24 {
                interior.push([
                    left + (right - left) * i as f64 / 24.,
                    top + (bottom - top) * j as f64 / 24.,
                ]);
            }
        }
        interior.sort_by(|a, b| {
            let distance =
                |p: &[f64; 2]| (p[0] - preferred[0]).powi(2) + (p[1] - preferred[1]).powi(2);
            distance(a).total_cmp(&distance(b))
        });
        candidates.extend(interior);
        for [x, y] in candidates {
            let rect = Rect::new(x, y, width, height);
            if !self.avoid_data || !occupied(rect) {
                return Ok(rect);
            }
        }
        anyhow::bail!(
            "No legend placement avoids protected data; shorten the legend, enlarge the panel, or change avoid_layers"
        )
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
    fn compact_external_legend_leaves_the_data_panel_clear() {
        let mut legend = Legend::new(vec![LegendEntry::line("Receiver", "blue")]);
        legend.position = LegendPosition::AboveCenter;
        let r = legend
            .place(Rect::new(10., 20., 50., 60.), 40., 6., &|_| true)
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
        assert_eq!(legend.place(panel, 30., 12., &|_| true).unwrap().x, 12.);
        assert!(legend.place(panel, 200., 12., &|_| false).is_err());
    }
}
