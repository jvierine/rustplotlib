//! Port of jvierine/sabmod, commit bb2388d4f45dba6aaa0e944f29bdf686e54a30d4.
//! Dimant & Oppenheim (2017), doi:10.1002/2017JA023963, Eqs. 45–48.
//! Returns ne/n_star, NOT a fitted or absolute electron density.
use std::f64::consts::PI;
pub struct Density {
    quad: Vec<(f64, f64)>,
}
impl Density {
    pub fn new(n: usize) -> Self {
        let mut quad = vec![];
        for i in 0..n {
            let mut x = (PI * (i as f64 + 0.75) / (n as f64 + 0.5)).cos();
            let mut derivative;
            loop {
                let mut p0 = 1.0;
                let mut p1 = x;
                for k in 2..=n {
                    let p = ((2 * k - 1) as f64 * x * p1 - (k - 1) as f64 * p0) / k as f64;
                    p0 = p1;
                    p1 = p;
                }
                derivative = n as f64 * (x * p1 - p0) / (x * x - 1.0);
                let next = x - p1 / derivative;
                if (next - x).abs() < 1e-15 {
                    x = next;
                    break;
                }
                x = next;
            }
            quad.push((
                (x + 1.0) * PI / 4.0,
                2.0 / ((1.0 - x * x) * derivative * derivative) * PI / 4.0,
            ));
        }
        Self { quad }
    }
    pub fn at(&self, z: f64, r: f64) -> f64 {
        let radius = z.hypot(r);
        if radius == 0.0 {
            return f64::NAN;
        }
        let ct = z / radius;
        let c = ct.abs();
        let q = radius.powf(2.0 / 3.0);
        let c23 = c.powf(2.0 / 3.0);
        let f1 = (2.0 * PI / 3.0).sqrt() / radius * libm::erf(1.5f64.sqrt() * (radius * c).cbrt())
            - (-1.5 * q * c23).exp()
                * ((4.0 - PI) * c
                    / (2.0 * (1.0 + (4.0 - PI).powi(2) * q * c23 / (2.0 * PI)).sqrt())
                    + 2.0 * c.cbrt() / q);
        let f2 = (2.0 * PI / 3.0).sqrt() / radius * libm::erf((1.5 * q).sqrt())
            - (1.0 + 2.0 / q) * (-1.5 * q).exp();
        // xi=c+(1-c)sin²(t) removes the integrable endpoint singularity.
        let f3 = if c >= 1.0 {
            0.0
        } else {
            self.quad
                .iter()
                .map(|&(t, w)| {
                    let xi = c + (1.0 - c) * t.sin().powi(2);
                    let xi23 = xi.powf(2.0 / 3.0);
                    let kernel = (1.0 + 2.0 * q * xi23 / PI).sqrt() * (-1.5 * q * xi23).exp();
                    let angle = if c == 0.0 {
                        0.0
                    } else {
                        (c * ((1.0 - c) * t.cos().powi(2) * (1.0 + xi)).sqrt()
                            / (xi * (1.0 - c * c).sqrt()))
                        .clamp(0.0, 1.0)
                        .asin()
                    };
                    w * kernel
                        * ((t.sin().powi(2) * (xi + c) / (t.cos().powi(2) * (1.0 + xi))).sqrt()
                            + c * angle)
                        * 2.0
                        * (1.0 - c)
                        * t.sin()
                        * t.cos()
                })
                .sum::<f64>()
        };
        (c * f1 + ct * f2 + f3) / radius
    }
}
