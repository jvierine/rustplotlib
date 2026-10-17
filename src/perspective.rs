//! Metric 3-D perspective projection for native vector scenes.
use anyhow::{ensure, Result};

#[derive(Clone, Debug)]
pub struct PerspectiveCamera {
    eye: [f64; 3],
    right: [f64; 3],
    up: [f64; 3],
    backward: [f64; 3],
    tangent: f64,
    aspect: f64,
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(a: [f64; 3]) -> Result<[f64; 3]> {
    let norm = dot(a, a).sqrt();
    ensure!(
        norm.is_finite() && norm > 1e-12,
        "Degenerate camera direction"
    );
    Ok(a.map(|v| v / norm))
}
impl PerspectiveCamera {
    /// Right-handed look-at camera. FOV is vertical, in degrees.
    pub fn new(
        eye: [f64; 3],
        target: [f64; 3],
        up: [f64; 3],
        vertical_fov_deg: f64,
        aspect: f64,
    ) -> Result<Self> {
        ensure!(
            eye.iter().chain(&target).chain(&up).all(|v| v.is_finite()),
            "Nonfinite camera"
        );
        ensure!(
            vertical_fov_deg > 0. && vertical_fov_deg < 180. && aspect.is_finite() && aspect > 0.,
            "Invalid camera field of view or aspect"
        );
        let backward = unit(std::array::from_fn(|i| eye[i] - target[i]))?;
        let right = unit(cross(up, backward))?;
        let up = cross(backward, right);
        Ok(Self {
            eye,
            right,
            up,
            backward,
            tangent: (vertical_fov_deg.to_radians() / 2.).tan(),
            aspect,
        })
    }
    /// Normalized x/right, y/down and positive camera distance. Points behind
    /// the camera are absent; viewport clipping belongs to the scene panel.
    pub fn project(&self, point: [f64; 3]) -> Option<[f64; 3]> {
        if !point.iter().all(|v| v.is_finite()) {
            return None;
        }
        let q = std::array::from_fn(|i| point[i] - self.eye[i]);
        let depth = -dot(q, self.backward);
        if depth <= 1e-9 {
            return None;
        }
        Some([
            0.5 + dot(q, self.right) / (2. * depth * self.tangent * self.aspect),
            0.5 - dot(q, self.up) / (2. * depth * self.tangent),
            depth,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metric_camera_has_correct_orientation_and_perspective() {
        let c = PerspectiveCamera::new([0., 0., 10.], [0.; 3], [0., 1., 0.], 90., 2.).unwrap();
        assert_eq!(c.project([0.; 3]).unwrap(), [0.5, 0.5, 10.]);
        let a = c.project([2., 2., 0.]).unwrap();
        assert!((a[0] - 0.55).abs() < 1e-12 && (a[1] - 0.4).abs() < 1e-12);
        assert!((c.project([2., 0., 5.]).unwrap()[0] - 0.6).abs() < 1e-12);
        assert!(c.project([0., 0., 11.]).is_none());
        assert!(c.project([f64::NAN, 0., 0.]).is_none());
    }
    #[test]
    fn invalid_or_degenerate_cameras_are_rejected() {
        assert!(PerspectiveCamera::new([0.; 3], [0.; 3], [0., 1., 0.], 42., 1.).is_err());
        assert!(PerspectiveCamera::new([0., 0., 1.], [0.; 3], [0., 0., 1.], 42., 1.).is_err());
        assert!(PerspectiveCamera::new([0., 0., 1.], [0.; 3], [0., 1., 0.], 180., 1.).is_err());
    }
}
