use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// X varies fastest; samples are cell centres in a Cartesian box.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Volume {
    pub shape: [usize; 3],
    pub bounds: [[f64; 2]; 3],
    pub values: Vec<f32>,
}
impl Volume {
    pub fn new(shape: [usize; 3], bounds: [[f64; 2]; 3], values: Vec<f32>) -> Result<Self> {
        ensure!(
            shape.iter().all(|n| *n >= 2),
            "Volume requires at least two samples per axis"
        );
        let count = shape
            .into_iter()
            .try_fold(1usize, |a, b| a.checked_mul(b))
            .ok_or_else(|| anyhow::anyhow!("Volume dimensions overflow"))?;
        ensure!(values.len() == count, "Volume shape mismatch");
        ensure!(
            bounds
                .iter()
                .all(|b| b[0].is_finite() && b[1].is_finite() && b[1] > b[0]),
            "Invalid volume bounds"
        );
        Ok(Self {
            shape,
            bounds,
            values,
        })
    }
    pub fn bytes_le(&self) -> Vec<u8> {
        self.values.iter().flat_map(|x| x.to_le_bytes()).collect()
    }
}
