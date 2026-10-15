//! Vector markers sized in typographic points, independent of axis ranges.
use crate::{ColorMap, Figure, Layer, Line, Normalization, Panel, PolygonMesh, Scale};
use anyhow::{ensure, Result};

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
        let dx = r * (axes.xlim[1] - axes.xlim[0]) / self.rect.width;
        let dy = r * (axes.ylim[1] - axes.ylim[0]) / self.rect.height;
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
            .filter(|p| p.iter().all(|v| v.is_finite()))
            .map(|p| {
                vertices
                    .iter()
                    .map(|v| [p[0] + dx * v[0], p[1] + dy * v[1]])
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
