//! Vector markers sized in typographic points, independent of axis ranges.
use crate::{ColorMap, Figure, Layer, Line, Normalization, Panel, PolygonMesh, Scale};
use anyhow::{Result, ensure};

#[derive(Clone, Copy, Debug)]
pub enum MarkerShape {
    Circle,
    Square,
    Diamond,
    Triangle,
    HorizontalCap,
    VerticalCap,
}

#[derive(Clone, Debug)]
pub struct MarkerStyle {
    pub shape: MarkerShape,
    /// Full diameter or cap length, in points.
    pub size_pt: f64,
    pub color: [f64; 3],
}

impl Panel {
    /// Absolute asymmetric interval endpoints. Measurements are never connected.
    /// Caps have fixed typographic length, including on logarithmic axes.
    pub fn errorbars(
        &mut self,
        points: &[[f64; 2]],
        x_bounds: Option<&[[f64; 2]]>,
        y_bounds: Option<&[[f64; 2]]>,
        color: &str,
        width_pt: f64,
        cap_pt: f64,
        opacity: f64,
    ) -> Result<()> {
        ensure!(
            x_bounds.is_none_or(|b| b.len() == points.len())
                && y_bounds.is_none_or(|b| b.len() == points.len()),
            "Errorbar bounds/points mismatch"
        );
        ensure!(
            width_pt.is_finite()
                && width_pt >= 0.
                && cap_pt.is_finite()
                && cap_pt >= 0.
                && opacity.is_finite()
                && (0. ..=1.).contains(&opacity),
            "Invalid errorbar style"
        );
        let a = self
            .axes
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Errorbars require axes"))?;
        let dx = cap_pt * 25.4 / 72.27 / 2.
            * (a.xscale.transform(a.xlim[1]) - a.xscale.transform(a.xlim[0]))
            / self.rect.width;
        let dy = cap_pt * 25.4 / 72.27 / 2.
            * (a.yscale.transform(a.ylim[1]) - a.yscale.transform(a.ylim[0]))
            / self.rect.height;
        let mut vertices = Vec::new();
        for (i, &p) in points.iter().enumerate() {
            if !p.iter().all(|v| v.is_finite()) {
                continue;
            }
            if let Some(bounds) = x_bounds {
                let [lo, hi] = bounds[i];
                if lo.is_finite()
                    && hi.is_finite()
                    && a.xscale.transform(lo).is_finite()
                    && a.xscale.transform(hi).is_finite()
                {
                    vertices.extend([[lo, p[1]], [hi, p[1]], [f64::NAN; 2]]);
                    for x in [lo, hi] {
                        if cap_pt > 0. {
                            vertices.extend([
                                [x, a.yscale.inverse(a.yscale.transform(p[1]) - dy)],
                                [x, a.yscale.inverse(a.yscale.transform(p[1]) + dy)],
                                [f64::NAN; 2],
                            ]);
                        }
                    }
                }
            }
            if let Some(bounds) = y_bounds {
                let [lo, hi] = bounds[i];
                if lo.is_finite()
                    && hi.is_finite()
                    && a.yscale.transform(lo).is_finite()
                    && a.yscale.transform(hi).is_finite()
                {
                    vertices.extend([[p[0], lo], [p[0], hi], [f64::NAN; 2]]);
                    for y in [lo, hi] {
                        if cap_pt > 0. {
                            vertices.extend([
                                [a.xscale.inverse(a.xscale.transform(p[0]) - dx), y],
                                [a.xscale.inverse(a.xscale.transform(p[0]) + dx), y],
                                [f64::NAN; 2],
                            ]);
                        }
                    }
                }
            }
        }
        let mut line = Line::new(vertices, color);
        line.width_pt = width_pt;
        line.opacity = opacity;
        self.push(Layer::Line(line));
        Ok(())
    }

    /// Translucent confidence band; NaNs break the band instead of bridging gaps.
    pub fn uncertainty_band(
        &mut self,
        x: &[f64],
        lo: &[f64],
        hi: &[f64],
        color: &str,
        opacity: f64,
    ) -> Result<()> {
        ensure!(
            x.len() == lo.len() && lo.len() == hi.len(),
            "Band shape mismatch"
        );
        ensure!(
            opacity.is_finite() && (0. ..=1.).contains(&opacity),
            "Invalid band opacity"
        );
        let mut start = 0;
        while start < x.len() {
            if ![x[start], lo[start], hi[start]]
                .iter()
                .all(|v| v.is_finite())
            {
                start += 1;
                continue;
            }
            let mut stop = start + 1;
            while stop < x.len() && [x[stop], lo[stop], hi[stop]].iter().all(|v| v.is_finite()) {
                stop += 1;
            }
            if stop - start >= 2 {
                let mut points: Vec<_> = (start..stop).map(|i| [x[i], lo[i]]).collect();
                points.extend((start..stop).rev().map(|i| [x[i], hi[i]]));
                points.push(points[0]);
                let mut line = Line::new(points, color);
                line.width_pt = 0.;
                line.fill = Some(color.into());
                line.fill_opacity = opacity;
                self.push(Layer::Line(line));
            }
            start = stop;
        }
        Ok(())
    }

    /// Scalar-coloured circular measurements with a vector edge and opacity.
    /// Symbol dimensions are physical points, independent of data-axis scales.
    pub fn scatter_colored(
        &mut self,
        points: &[[f64; 2]],
        values: &[f64],
        scale: &Scale,
        diameter_pt: f64,
        opacity: f64,
        edge: &str,
    ) -> Result<()> {
        ensure!(
            points.len() == values.len(),
            "Scatter values/positions mismatch"
        );
        ensure!(
            diameter_pt.is_finite()
                && diameter_pt > 0.
                && opacity.is_finite()
                && (0. ..=1.).contains(&opacity),
            "Invalid scatter size/opacity"
        );
        let a = self
            .axes
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Scatter requires axes"))?;
        let radius = diameter_pt * 25.4 / 72.27 / 2.;
        let dx = radius * (a.xscale.transform(a.xlim[1]) - a.xscale.transform(a.xlim[0]))
            / self.rect.width;
        let dy = radius * (a.yscale.transform(a.ylim[1]) - a.yscale.transform(a.ylim[0]))
            / self.rect.height;
        for (&p, &value) in points.iter().zip(values) {
            let Some(c) = scale.map(value) else {
                continue;
            };
            if !p.iter().all(|v| v.is_finite()) {
                continue;
            }
            let polygon = (0..=32)
                .map(|i| {
                    let angle = i as f64 * std::f64::consts::TAU / 32.;
                    [
                        a.xscale
                            .inverse(a.xscale.transform(p[0]) + dx * angle.cos()),
                        a.yscale
                            .inverse(a.yscale.transform(p[1]) + dy * angle.sin()),
                    ]
                })
                .collect();
            let mut symbol = Line::new(polygon, edge);
            symbol.width_pt = 0.25;
            symbol.fill = Some(format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]));
            symbol.fill_opacity = opacity;
            symbol.opacity = opacity;
            self.layers.push(Layer::Line(symbol));
        }
        Ok(())
    }

    /// Add vector symbols at data coordinates with a fixed physical size.
    pub fn markers(
        &mut self,
        figure: &mut Figure,
        points: &[[f64; 2]],
        style: &MarkerStyle,
    ) -> Result<()> {
        ensure!(
            style.size_pt.is_finite() && style.size_pt > 0.,
            "Invalid marker size"
        );
        ensure!(
            style
                .color
                .iter()
                .all(|v| v.is_finite() && (0. ..=1.).contains(v)),
            "Invalid marker color"
        );
        let axes = self
            .axes
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Markers require axes"))?;
        let r = style.size_pt * 25.4 / 72. / 2.;
        let dx = r * (axes.xscale.transform(axes.xlim[1]) - axes.xscale.transform(axes.xlim[0]))
            / self.rect.width;
        let dy = r * (axes.yscale.transform(axes.ylim[1]) - axes.yscale.transform(axes.ylim[0]))
            / self.rect.height;
        let vertices: Vec<[f64; 2]> = match style.shape {
            MarkerShape::HorizontalCap => vec![[-1., 0.], [1., 0.]],
            MarkerShape::VerticalCap => vec![[0., -1.], [0., 1.]],
            MarkerShape::Square => vec![[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]],
            MarkerShape::Diamond => vec![[-1., 0.], [0., -1.], [1., 0.], [0., 1.]],
            MarkerShape::Triangle => vec![[0., 1.], [-0.866, -0.5], [0.866, -0.5]],
            MarkerShape::Circle => (0..32)
                .map(|i| {
                    let a = i as f64 * std::f64::consts::TAU / 32.;
                    [a.cos(), a.sin()]
                })
                .collect(),
        };
        let polygons: Vec<Vec<_>> = points
            .iter()
            .filter(|p| {
                p.iter().all(|v| v.is_finite())
                    && axes.xscale.transform(p[0]).is_finite()
                    && axes.yscale.transform(p[1]).is_finite()
            })
            .map(|p| {
                vertices
                    .iter()
                    .map(|v| {
                        [
                            axes.xscale.inverse(axes.xscale.transform(p[0]) + dx * v[0]),
                            axes.yscale.inverse(axes.yscale.transform(p[1]) + dy * v[1]),
                        ]
                    })
                    .collect()
            })
            .collect();
        if polygons.is_empty() {
            return Ok(());
        }
        if vertices.len() == 2 {
            let c = style.color.map(|v| (v * 255.).round() as u8);
            let color = format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
            for p in polygons {
                let mut l = Line::new(p, &color);
                l.width_pt = 0.6;
                self.push(Layer::Line(l));
            }
        } else {
            let scale = figure.scale(Scale::new(
                [0., 1.],
                Normalization::Linear,
                ColorMap::new(vec![style.color, style.color])?,
            )?);
            let values = vec![0.5; polygons.len()];
            self.push(Layer::PolygonMesh(PolygonMesh::new(
                polygons, values, scale,
            )?));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Axes, IntendedUse, Rect};
    #[test]
    fn asymmetric_errors_are_independent_stems_with_physical_caps() {
        let mut p = Panel::new(
            Rect::new(0., 0., 90., 50.),
            Axes::new([1., 1000.], [0., 10.], "", ""),
        );
        p.axes
            .as_mut()
            .unwrap()
            .set_xscale(crate::AxisScale::Log10)
            .unwrap();
        p.errorbars(
            &[[10., 2.], [100., 8.]],
            Some(&[[8., 17.], [40., 120.]]),
            None,
            "#333333",
            0.6,
            4.,
            0.5,
        )
        .unwrap();
        let Layer::Line(l) = &p.layers[0] else {
            panic!("Expected error stems")
        };
        assert_eq!(&l.points[0..2], &[[8., 2.], [17., 2.]]);
        assert!(l.points[2][0].is_nan());
        assert_eq!(&l.points[9..11], &[[40., 8.], [120., 8.]]);
        assert!(
            ((p.data_to_figure(l.points[3])[1] - p.data_to_figure(l.points[4])[1]).abs()
                - 4. * 25.4 / 72.27)
                .abs()
                < 1e-10
        );
    }
    #[test]
    fn confidence_bands_do_not_bridge_missing_samples() {
        let mut p = Panel::new(
            Rect::new(0., 0., 90., 50.),
            Axes::new([0., 4.], [0., 5.], "", ""),
        );
        p.uncertainty_band(
            &[0., 1., 2., 3., 4.],
            &[1., 1., f64::NAN, 1., 1.],
            &[2., 2., f64::NAN, 2., 2.],
            "#999999",
            0.4,
        )
        .unwrap();
        assert_eq!(p.layers.len(), 2);
        for layer in &p.layers {
            let Layer::Line(l) = layer else {
                panic!("Expected filled band")
            };
            assert_eq!(l.fill_opacity, 0.4);
        }
    }
    #[test]
    fn physical_marker_size_and_invalid_input() {
        let mut f = Figure::new(IntendedUse::ArticleWide);
        let mut p = Panel::new(
            Rect::new(0., 0., 100., 50.),
            Axes::new([0., 10.], [0., 20.], "", ""),
        );
        let mut s = MarkerStyle {
            shape: MarkerShape::HorizontalCap,
            size_pt: 6.,
            color: [0., 0., 0.],
        };
        p.markers(&mut f, &[[5., 10.], [f64::NAN, 0.]], &s).unwrap();
        if let Layer::Line(l) = &p.layers[0] {
            assert!(((l.points[1][0] - l.points[0][0]) * 10. - 6. * 25.4 / 72.).abs() < 1e-12);
        } else {
            panic!("expected cap");
        }
        assert_eq!(p.layers.len(), 1);
        s.size_pt = -1.;
        assert!(p.markers(&mut f, &[[0., 0.]], &s).is_err());
    }
}
