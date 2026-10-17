//! Axis coordinates are independent of scalar colour normalization.
use anyhow::{Result, ensure};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum AxisScale {
    #[default]
    Linear,
    Log10,
}

impl AxisScale {
    pub fn transform(self, value: f64) -> f64 {
        match self {
            Self::Linear => value,
            Self::Log10 => value.log10(),
        }
    }
    pub fn inverse(self, value: f64) -> f64 {
        match self {
            Self::Linear => value,
            Self::Log10 => 10_f64.powf(value),
        }
    }
    pub fn validate(self, limits: [f64; 2]) -> Result<()> {
        ensure!(
            limits.iter().all(|v| v.is_finite()) && limits[1] > limits[0],
            "Invalid axis limits"
        );
        ensure!(
            self != Self::Log10 || limits[0] > 0.,
            "Logarithmic axes require positive limits"
        );
        Ok(())
    }
    pub fn ticks(self, limits: [f64; 2]) -> (Vec<f64>, Vec<f64>) {
        if self == Self::Linear {
            return (crate::figure::ticks(limits, 5), vec![]);
        }
        if self.validate(limits).is_err() {
            return (vec![], vec![]);
        }
        let mut major = vec![];
        let mut minor = vec![];
        for exponent in limits[0].log10().floor() as i32..=limits[1].log10().ceil() as i32 {
            let base = 10_f64.powi(exponent);
            for multiple in 1..10 {
                let value = base * multiple as f64;
                if value >= limits[0] && value <= limits[1] {
                    if multiple == 1 {
                        major.push(value);
                    } else {
                        minor.push(value);
                    }
                }
            }
        }
        (major, minor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn logarithmic_decades_and_minor_ticks() {
        let (major, minor) = AxisScale::Log10.ticks([1., 1000.]);
        assert_eq!(major, vec![1., 10., 100., 1000.]);
        assert_eq!(minor.len(), 24);
        assert!(AxisScale::Log10.validate([0., 100.]).is_err());
        assert!(
            (AxisScale::Log10.inverse(AxisScale::Log10.transform(0.003)) - 0.003).abs() < 1e-12
        );
    }
}
