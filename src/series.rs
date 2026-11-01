//! Distinct default line series, with matching legend keys.
use crate::{Layer, Legend, LegendEntry, LegendSymbol, Line, Panel};

/// The six-series default cycle varies both colour and dash pattern.
/// Dash lengths use figure millimetres, like `Line::dash`, so the patterns
/// stay the same size on logarithmic axes and across different data ranges.
#[derive(Clone, Debug, PartialEq)]
pub struct LineStyle {
    pub color: String,
    pub width_pt: f64,
    pub dash: Option<String>,
}

impl Default for LineStyle {
    fn default() -> Self {
        Self::for_series(0)
    }
}

impl LineStyle {
    pub fn for_series(index: usize) -> Self {
        let (color, dash) = [
            ("#0072B2", None),
            ("#D55E00", Some("2.4 1.2")),
            ("#009E73", Some("0.6 1.0")),
            ("#CC79A7", Some("2.4 0.9 0.6 0.9")),
            ("#333333", Some("4.0 1.2")),
            ("#E69F00", Some("2.4 0.8 0.6 0.8 0.6 0.8")),
        ][index % 6];
        Self {
            color: color.into(),
            width_pt: 1.4,
            dash: dash.map(str::to_owned),
        }
    }

    pub fn line(&self, points: Vec<[f64; 2]>) -> Line {
        let mut line = Line::new(points, self.color.clone());
        line.width_pt = self.width_pt;
        line.dash = self.dash.clone();
        line
    }
}

impl Line {
    /// Use the default style for a stable category index across panels.
    pub fn series(points: Vec<[f64; 2]>, index: usize) -> Self {
        LineStyle::for_series(index).line(points)
    }

    /// Copy the actual colour, width and dash into a legend key.
    pub fn legend(&self, label: impl Into<String>) -> LegendEntry {
        LegendEntry {
            label: label.into(),
            symbol: LegendSymbol::Line {
                color: self.color.clone(),
                width_pt: self.width_pt,
                dash: self.dash.clone(),
            },
        }
    }
}

impl Panel {
    /// Plot a labelled line with the next default colour and dash pattern.
    /// Each panel has an independent cycle. Explicit lines, reference lines,
    /// grids, markers and fills do not consume it. The matching legend is
    /// created automatically and uses normal data-avoiding placement.
    pub fn plot(&mut self, points: Vec<[f64; 2]>, label: impl Into<String>) -> &mut Self {
        let line = Line::series(points, self.next_line_series);
        self.next_line_series += 1;
        let entry = line.legend(label);
        self.push(Layer::Line(line));
        match &mut self.legend {
            Some(legend) => legend.entries.push(entry),
            None => self.legend = Some(Legend::new(vec![entry])),
        }
        self
    }
}
