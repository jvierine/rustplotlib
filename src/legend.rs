//! Compact, LaTeX-typeset legends with explicitly protected data layers.
use crate::{Panel, Rect};
use anyhow::{Result, ensure};

#[derive(Clone, Copy, Debug)]
pub enum LegendPosition {
    UpperLeft,
    UpperRight,
    LowerLeft,
    LowerRight,
}

#[derive(Clone, Debug)]
pub enum LegendSymbol {
    Line {
        color: String,
        width_pt: f64,
        dash: Option<String>,
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
/// avoidance is enabled. Only `avoid_layers` are protected: a background image
/// or contextual map boundaries may deliberately be covered. If no clear
/// placement exists, export fails rather than hiding the protected data.
#[derive(Clone, Debug)]
pub struct Legend {
    pub entries: Vec<LegendEntry>,
    pub position: LegendPosition,
    pub avoid_data: bool,
    pub avoid_layers: Vec<usize>,
    pub padding_mm: f64,
    pub background_opacity: f64,
}
impl Legend {
    pub fn new(entries: Vec<LegendEntry>) -> Self {
        Self {
            entries,
            position: LegendPosition::UpperRight,
            avoid_data: true,
            avoid_layers: vec![],
            padding_mm: 2.,
            background_opacity: 0.96,
        }
    }

    pub(crate) fn validate(&self, panel: &Panel) -> Result<()> {
        ensure!(!self.entries.is_empty(), "Empty legend");
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
        for entry in &self.entries {
            ensure!(!entry.label.is_empty(), "Empty legend label");
            let size = match &entry.symbol {
                LegendSymbol::Line { width_pt, .. } => *width_pt,
                LegendSymbol::Circle { radius_pt, .. } => *radius_pt,
            };
            ensure!(size.is_finite() && size > 0., "Invalid legend symbol size");
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
