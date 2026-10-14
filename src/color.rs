use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum Normalization {
    Linear,
    Log,
    Asinh { width: f64 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColorMap {
    pub stops: Vec<[f64; 3]>,
    #[serde(default)]
    pub listed: bool,
}
impl ColorMap {
    /// Blue-white-red seismic palette, with the standard 256-entry lookup table.
    pub fn seismic() -> Self {
        let continuous = Self {
            listed: false,
            stops: vec![
                [0., 0., 0.3],
                [0., 0., 1.],
                [1., 1., 1.],
                [1., 0., 0.],
                [0.5, 0., 0.],
            ],
        };
        Self {
            listed: true,
            stops: (0..256)
                .map(|i| {
                    let u = i as f64 / 255. * 4.;
                    let k = (u.floor() as usize).min(3);
                    let t = u - k as f64;
                    std::array::from_fn(|j| {
                        continuous.stops[k][j] * (1. - t) + continuous.stops[k + 1][j] * t
                    })
                })
                .collect(),
        }
    }
    /// Standard perceptually uniform inferno lookup table (Smith and van der Walt).
    pub fn inferno() -> Self {
        Self {
            listed: true,
            stops: crate::palettes::INFERNO.to_vec(),
        }
    }
    pub fn new(stops: Vec<[f64; 3]>) -> Result<Self> {
        ensure!(
            stops.len() >= 2
                && stops
                    .iter()
                    .flatten()
                    .all(|x| x.is_finite() && (0.0..=1.0).contains(x)),
            "Invalid color stops"
        );
        Ok(Self {
            stops,
            listed: false,
        })
    }
    pub fn diverging() -> Self {
        Self {
            listed: false,
            stops: vec![
                [0.13, 0.26, 0.45],
                [0.34, 0.58, 0.70],
                [0.97, 0.96, 0.93],
                [0.82, 0.46, 0.31],
                [0.49, 0.13, 0.18],
            ],
        }
    }
    pub fn sequential() -> Self {
        Self {
            listed: false,
            stops: vec![
                [0.97, 0.96, 0.91],
                [0.67, 0.78, 0.72],
                [0.31, 0.56, 0.55],
                [0.12, 0.31, 0.40],
                [0.05, 0.13, 0.23],
            ],
        }
    }
    pub fn ember() -> Self {
        Self {
            listed: false,
            stops: vec![
                [0.08, 0.08, 0.16],
                [0.35, 0.13, 0.31],
                [0.71, 0.24, 0.25],
                [0.97, 0.61, 0.28],
                [1.0, 0.94, 0.73],
            ],
        }
    }
    /// Preserve an exact discrete RGB lookup table, e.g. an existing paper's palette.
    pub fn listed(stops: Vec<[f64; 3]>) -> Result<Self> {
        let mut map = Self::new(stops)?;
        map.listed = true;
        Ok(map)
    }
    /// Custom high-chroma sequential palette for fine mixing structures.
    pub fn estuary() -> Self {
        Self {
            listed: false,
            stops: vec![
                [0.055, 0.065, 0.19],
                [0.13, 0.26, 0.64],
                [0.04, 0.65, 0.69],
                [0.54, 0.42, 0.77],
                [0.91, 0.38, 0.49],
                [0.99, 0.68, 0.40],
                [0.99, 0.90, 0.57],
            ],
        }
    }
    pub fn sample(&self, t: f64) -> [u8; 3] {
        if self.listed {
            let i = ((t.clamp(0.0, 1.0) * self.stops.len() as f64).floor() as usize)
                .min(self.stops.len() - 1);
            return self.stops[i].map(|x| (x * 255.0).round() as u8);
        }
        let u = t.clamp(0.0, 1.0) * (self.stops.len() - 1) as f64;
        let i = (u.floor() as usize).min(self.stops.len() - 2);
        let w = u - i as f64;
        std::array::from_fn(|k| {
            ((self.stops[i][k] * (1.0 - w) + self.stops[i + 1][k] * w) * 255.0).round() as u8
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scale {
    pub limits: [f64; 2],
    pub normalization: Normalization,
    pub colors: ColorMap,
}
impl Scale {
    pub fn new(limits: [f64; 2], normalization: Normalization, colors: ColorMap) -> Result<Self> {
        let s = Self {
            limits,
            normalization,
            colors,
        };
        s.validate()?;
        Ok(s)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.limits.iter().all(|x| x.is_finite()) && self.limits[1] > self.limits[0],
            "Color limits must be finite and increasing"
        );
        match self.normalization {
            Normalization::Log => ensure!(self.limits[0] > 0.0, "Log limits must be positive"),
            Normalization::Asinh { width } => ensure!(
                width.is_finite() && width > 0.0,
                "Asinh width must be positive"
            ),
            _ => {}
        }
        ColorMap::new(self.colors.stops.clone())?;
        Ok(())
    }
    pub fn transform(&self, x: f64) -> f64 {
        match self.normalization {
            Normalization::Linear => x,
            Normalization::Log => x.ln(),
            Normalization::Asinh { width } => (x / width).asinh(),
        }
    }
    /// Missing values and nonpositive log data are transparent, never zero.
    pub fn map(&self, x: f64) -> Option<[u8; 4]> {
        if !x.is_finite() || (self.normalization == Normalization::Log && x <= 0.0) {
            return None;
        }
        let [a, b] = self.limits.map(|v| self.transform(v));
        let rgb = self.colors.sample((self.transform(x) - a) / (b - a));
        Some([rgb[0], rgb[1], rgb[2], 255])
    }
    pub fn inverse_fraction(&self, t: f64) -> f64 {
        let [a, b] = self.limits.map(|v| self.transform(v));
        let x = a + t * (b - a);
        match self.normalization {
            Normalization::Linear => x,
            Normalization::Log => x.exp(),
            Normalization::Asinh { width } => width * x.sinh(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalizations_roundtrip_and_keep_missing_data() {
        for norm in [
            Normalization::Linear,
            Normalization::Log,
            Normalization::Asinh { width: 0.03 },
        ] {
            let s = Scale::new([0.01, 100.0], norm, ColorMap::sequential()).unwrap();
            for t in [0.0, 0.1, 0.5, 1.0] {
                let x = s.inverse_fraction(t);
                let [a, b] = s.limits.map(|v| s.transform(v));
                assert!(((s.transform(x) - a) / (b - a) - t).abs() < 1e-12);
            }
            assert!(s.map(f64::NAN).is_none());
        }
        assert!(Scale::new([-1.0, 1.0], Normalization::Log, ColorMap::diverging()).is_err());
    }
}
