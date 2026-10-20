//! Multivariate radar color mapping and native, three-variable color keys.
//! The two key slices show hue/value at zero width, and hue/saturation at
//! maximum power. Together they explain HSV without a misleading scalar bar.
use crate::{Axes, Figure, Label, Layer, Panel, Rect};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HsvColorKey {
    pub velocity_limit: f64,
    pub width_limit: f64,
    pub power_limits: [f64; 2],
    pub power_label: String,
}
impl HsvColorKey {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.velocity_limit.is_finite()
                && self.velocity_limit > 0.
                && self.width_limit.is_finite()
                && self.width_limit > 0.
                && self.power_limits.iter().all(|v| v.is_finite())
                && self.power_limits[1] > self.power_limits[0],
            "Invalid HSV limits"
        );
        Ok(())
    }
    /// Exact mapping used by FieldView's moment renderer: blue-negative,
    /// green-zero, red-positive; broad spectra desaturate toward white.
    pub fn rgb(&self, power: f64, velocity: f64, width: f64) -> [f64; 3] {
        let h = (2. / 3.)
            * (1. - ((velocity + self.velocity_limit) / (2. * self.velocity_limit)).clamp(0., 1.));
        let s = 1. - 0.85 * (width / self.width_limit).clamp(0., 1.);
        let v = ((power - self.power_limits[0]) / (self.power_limits[1] - self.power_limits[0]))
            .clamp(0., 1.);
        std::array::from_fn(|i| {
            let offset = [0., 2. / 3., 1. / 3.][i];
            let p = (((h + offset).fract() * 6. - 3.).abs() - 1.).clamp(0., 1.);
            v * (1. - s + s * p)
        })
    }
    /// Add two native axis-bearing slices in the supplied millimetre rectangles.
    /// Decorations are ordinary rustplotlib panels, not HTML/CSS approximations.
    pub fn add_to(&self, figure: &mut Figure, power_rect: Rect, width_rect: Rect) -> Result<()> {
        self.validate()?;
        for (kind, rect) in [power_rect, width_rect].into_iter().enumerate() {
            figure.labels.push(Label::new(
                if kind == 0 {
                    "Zero spectral width"
                } else {
                    "Maximum brightness"
                },
                rect.x + rect.width / 2.,
                rect.y - 4.,
            ));
            let ylim = if kind == 0 {
                self.power_limits
            } else {
                [0., self.width_limit]
            };
            let ylabel = if kind == 0 {
                self.power_label.clone()
            } else {
                r"Width (m s$^{-1}$)".into()
            };
            let mut axes = Axes::new(
                [-self.velocity_limit, self.velocity_limit],
                ylim,
                if kind == 1 {
                    r"Velocity (m s$^{-1}$)"
                } else {
                    ""
                },
                ylabel,
            );
            axes.xticks = vec![-self.velocity_limit, 0., self.velocity_limit];
            axes.yticks = if kind == 0 {
                vec![ylim[0], (ylim[0] + ylim[1]) / 2., ylim[1]]
            } else {
                vec![0., self.width_limit / 2., self.width_limit]
            };
            axes.show_x_axis = kind == 1 || power_rect.y == width_rect.y;
            let mut panel = Panel::new(rect, axes);
            let image = image::RgbaImage::from_fn(256, 96, |x, y| {
                let velocity = -self.velocity_limit + 2. * self.velocity_limit * x as f64 / 255.;
                let q = 1. - y as f64 / 95.;
                let rgb = if kind == 0 {
                    self.rgb(ylim[0] + q * (ylim[1] - ylim[0]), velocity, 0.)
                } else {
                    self.rgb(self.power_limits[1], velocity, q * self.width_limit)
                };
                image::Rgba([
                    (rgb[0] * 255.).round() as u8,
                    (rgb[1] * 255.).round() as u8,
                    (rgb[2] * 255.).round() as u8,
                    255,
                ])
            });
            panel.push(Layer::Image(image));
            figure.panels.push(panel);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn colors_match_shader_and_three_encodings() {
        let key = HsvColorKey {
            velocity_limit: 30.,
            width_limit: 10.,
            power_limits: [-30., 0.],
            power_label: "Power (dB)".into(),
        };
        for (power, velocity, expected) in [
            (0., -30., [0., 0., 1.]),
            (0., 0., [0., 1., 0.]),
            (0., 30., [1., 0., 0.]),
            (-30., 30., [0., 0., 0.]),
            (-15., 30., [0.5, 0., 0.]),
        ] {
            for (a, b) in key.rgb(power, velocity, 0.).iter().zip(expected) {
                assert!((a - b).abs() < 1e-12);
            }
        }
        assert!((key.rgb(0., 30., 10.)[1] - 0.85).abs() < 1e-12);
    }
}
