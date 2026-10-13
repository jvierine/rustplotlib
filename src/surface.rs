//! Depth-tested, shaded surfaces for functions z=f(x,y).
use crate::Scale;
use anyhow::{Result, ensure};
#[derive(Clone, Debug)]
pub struct Surface {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub z: Vec<f64>,
    pub zlim: [f64; 2],
    pub scale: usize,
    pub yaw: f64,
    pub elevation: f64,
    pub shading: bool,
}
impl Surface {
    pub fn new(
        x: Vec<f64>,
        y: Vec<f64>,
        z: Vec<f64>,
        zlim: [f64; 2],
        scale: usize,
    ) -> Result<Self> {
        ensure!(
            x.len() > 1 && y.len() > 1 && z.len() == x.len() * y.len(),
            "Surface dimensions do not match"
        );
        ensure!(
            x.iter().chain(&y).all(|v| v.is_finite())
                && x.windows(2).all(|p| p[1] > p[0])
                && y.windows(2).all(|p| p[1] > p[0]),
            "Surface coordinates must increase"
        );
        ensure!(
            zlim.iter().all(|v| v.is_finite()) && zlim[1] > zlim[0],
            "Invalid surface height limits"
        );
        Ok(Self {
            x,
            y,
            z,
            zlim,
            scale,
            yaw: -0.65,
            elevation: 0.55,
            shading: true,
        })
    }
    /// Normalized screen x/y and camera depth. Useful for placing 3-D axes.
    pub fn project(&self, p: [f64; 3]) -> [f64; 3] {
        let x = 2.0 * (p[0] - self.x[0]) / (self.x[self.x.len() - 1] - self.x[0]) - 1.0;
        let y = 2.0 * (p[1] - self.y[0]) / (self.y[self.y.len() - 1] - self.y[0]) - 1.0;
        let z = 2.0 * (p[2] - self.zlim[0]) / (self.zlim[1] - self.zlim[0]) - 1.0;
        let right = self.yaw.cos() * x - self.yaw.sin() * y;
        let forward = self.yaw.sin() * x + self.yaw.cos() * y;
        [
            0.5 + 0.28 * right,
            0.5 - 0.28 * (-self.elevation.sin() * forward + self.elevation.cos() * z),
            self.elevation.cos() * forward + self.elevation.sin() * z,
        ]
    }
    pub(crate) fn raster(&self, w: u32, h: u32, scale: &Scale) -> image::RgbaImage {
        let nx = self.x.len();
        let ny = self.y.len();
        let mut img = image::RgbaImage::new(w, h);
        let mut depth = vec![f64::NEG_INFINITY; (w * h) as usize];
        let points: Vec<_> = (0..ny)
            .flat_map(|j| {
                (0..nx).map(move |i| {
                    let p = self.project([self.x[i], self.y[j], self.z[j * nx + i]]);
                    [p[0] * w as f64, p[1] * h as f64, p[2]]
                })
            })
            .collect();
        let edge = |a: [f64; 3], b: [f64; 3], x: f64, y: f64| {
            (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])
        };
        for j in 0..ny - 1 {
            for i in 0..nx - 1 {
                let a = j * nx + i;
                for ids in [[a, a + 1, a + nx + 1], [a, a + nx + 1, a + nx]] {
                    if ids.iter().any(|&id| !self.z[id].is_finite()) {
                        continue;
                    }
                    let v = ids.map(|id| points[id]);
                    let area = edge(v[0], v[1], v[2][0], v[2][1]);
                    if area.abs() < 1e-12 {
                        continue;
                    }
                    let x0 = v
                        .iter()
                        .map(|p| p[0])
                        .fold(f64::INFINITY, f64::min)
                        .floor()
                        .clamp(0.0, w as f64) as u32;
                    let x1 = v
                        .iter()
                        .map(|p| p[0])
                        .fold(f64::NEG_INFINITY, f64::max)
                        .ceil()
                        .clamp(0.0, w as f64) as u32;
                    let y0 = v
                        .iter()
                        .map(|p| p[1])
                        .fold(f64::INFINITY, f64::min)
                        .floor()
                        .clamp(0.0, h as f64) as u32;
                    let y1 = v
                        .iter()
                        .map(|p| p[1])
                        .fold(f64::NEG_INFINITY, f64::max)
                        .ceil()
                        .clamp(0.0, h as f64) as u32;
                    let dzx = (self.z[a + 1] - self.z[a]) / (self.x[i + 1] - self.x[i]);
                    let dzy = (self.z[a + nx] - self.z[a]) / (self.y[j + 1] - self.y[j]);
                    let light = if self.shading {
                        0.60 + 0.40
                            * ((-0.3 * dzx - 0.4 * dzy + 0.866)
                                / (1.0 + dzx * dzx + dzy * dzy).sqrt())
                            .max(0.0)
                    } else {
                        1.0
                    };
                    for y in y0..y1 {
                        for x in x0..x1 {
                            let xx = x as f64 + 0.5;
                            let yy = y as f64 + 0.5;
                            let b = [
                                edge(v[1], v[2], xx, yy) / area,
                                edge(v[2], v[0], xx, yy) / area,
                                edge(v[0], v[1], xx, yy) / area,
                            ];
                            if b.iter().any(|q| *q < -1e-10) {
                                continue;
                            }
                            let z = (0..3).map(|k| b[k] * v[k][2]).sum::<f64>();
                            let id = (y * w + x) as usize;
                            if z <= depth[id] {
                                continue;
                            }
                            let value = (0..3).map(|k| b[k] * self.z[ids[k]]).sum::<f64>();
                            if let Some(mut color) = scale.map(value) {
                                for c in &mut color[..3] {
                                    *c = (*c as f64 * light).round() as u8;
                                }
                                img.put_pixel(x, y, image::Rgba(color));
                                depth[id] = z;
                            }
                        }
                    }
                }
            }
        }
        img
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorMap, Normalization};
    #[test]
    fn missing_surface_mask_and_coverage() {
        let scale = Scale::new([0., 1.], Normalization::Linear, ColorMap::ember()).unwrap();
        let mut s = Surface::new(vec![0., 1.], vec![0., 1.], vec![0.5; 4], [0., 1.], 0).unwrap();
        let img = s.raster(200, 200, &scale);
        assert!(img.pixels().filter(|p| p[3] > 0).count() > 1000);
        s.z.fill(f64::NAN);
        assert!(s.raster(200, 200, &scale).pixels().all(|p| p[3] == 0));
    }
}
