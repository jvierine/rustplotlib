//! Overlap-safe categorical markers: open symbols, shared halo pass, no jitter.
use crate::{Layer, Legend, LegendEntry, LegendSymbol, Line, MarkerShape, MarkerStyle, Panel};
use anyhow::{Result, ensure};

/// Categories retain their measured coordinates even when symbols overlap.
/// Use distinct shapes: coincident identical outlines cannot encode two labels.
pub struct MarkerGroup<'a> {
    pub label: &'a str,
    pub points: &'a [[f64; 2]],
    pub style: MarkerStyle,
}

impl Panel {
    /// Draw categories with open symbols and return a matching legend.
    /// All white halos are painted before any coloured outlines, so a later
    /// category's halo cannot erase an earlier category's outline. Marker
    /// interiors remain transparent, including on heatmaps. No coordinates are
    /// moved, aggregated or discarded to resolve overlap. Distinct shapes are
    /// required; colour alone cannot resolve exact coincident outlines.
    pub fn marker_groups(&mut self, groups: &[MarkerGroup<'_>]) -> Result<Legend> {
        ensure!(!groups.is_empty(), "No marker groups");
        let axes = self
            .axes
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Markers require axes"))?;
        let mut paths = Vec::new();
        let mut entries = Vec::new();
        for (index, group) in groups.iter().enumerate() {
            let s = &group.style;
            ensure!(!group.label.is_empty(), "Empty marker label");
            ensure!(
                s.size_pt.is_finite() && s.size_pt > 0.,
                "Invalid marker size"
            );
            ensure!(
                s.color
                    .iter()
                    .all(|v| v.is_finite() && (0. ..=1.).contains(v)),
                "Invalid marker colour"
            );
            ensure!(
                !groups[..index].iter().any(|g| g.style.shape == s.shape),
                "Overlapping categories require distinct marker shapes"
            );
            let vertices: Vec<[f64; 2]> = match s.shape {
                MarkerShape::Circle => (0..64)
                    .map(|i| {
                        let a = i as f64 * std::f64::consts::TAU / 64.;
                        [a.cos(), a.sin()]
                    })
                    .collect(),
                MarkerShape::Square => vec![[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]],
                MarkerShape::Diamond => vec![[-1., 0.], [0., -1.], [1., 0.], [0., 1.]],
                MarkerShape::Triangle => vec![[0., 1.], [-0.866, -0.5], [0.866, -0.5]],
                MarkerShape::HorizontalCap => vec![[-1., 0.], [1., 0.]],
                MarkerShape::VerticalCap => vec![[0., -1.], [0., 1.]],
            };
            let radius = s.size_pt * 25.4 / 72.27 / 2.;
            let dx = radius
                * (axes.xscale.transform(axes.xlim[1]) - axes.xscale.transform(axes.xlim[0]))
                / self.rect.width;
            let dy = radius
                * (axes.yscale.transform(axes.ylim[1]) - axes.yscale.transform(axes.ylim[0]))
                / self.rect.height;
            let mut points = Vec::new();
            for p in group.points {
                let x = axes.xscale.transform(p[0]);
                let y = axes.yscale.transform(p[1]);
                if !x.is_finite() || !y.is_finite() {
                    continue;
                }
                let start = points.len();
                for v in &vertices {
                    points.push([
                        axes.xscale.inverse(x + dx * v[0]),
                        axes.yscale.inverse(y + dy * v[1]),
                    ]);
                }
                if vertices.len() > 2 {
                    points.push(points[start]);
                }
                points.push([f64::NAN; 2]);
            }
            let c = s.color.map(|v| (v * 255.).round() as u8);
            let color = format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
            entries.push(LegendEntry {
                label: group.label.into(),
                symbol: LegendSymbol::OutlinedMarker {
                    shape: s.shape,
                    color: color.clone(),
                    size_pt: s.size_pt,
                    width_pt: 1.1,
                },
            });
            paths.push((points, color));
        }
        // Two passes are essential: interleaved white/colour strokes erase
        // coincident categories even when all interiors are transparent.
        for (points, _) in &paths {
            let mut line = Line::new(points.clone(), "#ffffff");
            line.width_pt = 2.3;
            self.push(Layer::Line(line));
        }
        for (points, color) in paths {
            let mut line = Line::new(points, color);
            line.width_pt = 1.1;
            self.push(Layer::Line(line));
        }
        Ok(Legend::new(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Axes, AxisScale, Rect};
    fn panel() -> Panel {
        Panel::new(
            Rect::new(0., 0., 100., 70.),
            Axes::new([1., 100.], [0., 10.], "", ""),
        )
    }
    fn group(shape: MarkerShape, color: [f64; 3]) -> MarkerGroup<'static> {
        MarkerGroup {
            label: if shape == MarkerShape::Circle {
                "A"
            } else {
                "B"
            },
            points: &[[10., 5.]],
            style: MarkerStyle {
                shape,
                size_pt: 8.,
                color,
            },
        }
    }
    #[test]
    fn exact_overlap_preserves_open_paths_and_all_halos_precede_colours() {
        let mut p = panel();
        p.axes
            .as_mut()
            .unwrap()
            .set_xscale(AxisScale::Log10)
            .unwrap();
        let legend = p
            .marker_groups(&[
                group(MarkerShape::Circle, [0., 0., 1.]),
                group(MarkerShape::Diamond, [1., 0., 0.]),
            ])
            .unwrap();
        assert_eq!(p.layers.len(), 4);
        for (i, layer) in p.layers.iter().enumerate() {
            let Layer::Line(l) = layer else { panic!() };
            assert!(l.fill.is_none());
            assert_eq!(
                l.color,
                if i < 2 {
                    "#ffffff"
                } else if i == 2 {
                    "#0000ff"
                } else {
                    "#ff0000"
                }
            );
            let coords: Vec<_> = l
                .points
                .iter()
                .filter(|p| p.iter().all(|v| v.is_finite()))
                .map(|v| p.data_to_figure(*v))
                .collect();
            for d in 0..2 {
                let lo = coords.iter().map(|q| q[d]).fold(f64::INFINITY, f64::min);
                let hi = coords
                    .iter()
                    .map(|q| q[d])
                    .fold(f64::NEG_INFINITY, f64::max);
                assert!(((lo + hi) / 2. - p.data_to_figure([10., 5.])[d]).abs() < 1e-10);
                assert!((hi - lo - 8. * 25.4 / 72.27).abs() < 1e-10);
            }
        }
        assert!(
            legend
                .entries
                .iter()
                .all(|e| matches!(e.symbol, LegendSymbol::OutlinedMarker { size_pt: 8., .. }))
        );
    }
    #[test]
    fn ambiguous_shapes_and_invalid_styles_fail_without_partial_mutation() {
        let mut p = panel();
        let mut b = group(MarkerShape::Circle, [1., 0., 0.]);
        assert!(
            p.marker_groups(&[group(MarkerShape::Circle, [0., 0., 1.]), b])
                .is_err()
        );
        assert!(p.layers.is_empty());
        b = group(MarkerShape::Diamond, [1., 0., 0.]);
        b.style.size_pt = f64::NAN;
        assert!(
            p.marker_groups(&[group(MarkerShape::Circle, [0., 0., 1.]), b])
                .is_err()
        );
        assert!(p.layers.is_empty());
    }
}
