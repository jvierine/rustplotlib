//! Geographic coordinates are longitude/latitude in degrees. Projection output
//! is dimensionless; an orthographic globe has unit radius.
use crate::Line;

#[derive(Clone, Copy, Debug)]
pub enum Projection {
    /// Equal-area full-sky projection. West-positive supports astronomical
    /// longitude displays without modifying the input scientific coordinates.
    Hammer { central_longitude: f64, west_positive: bool },
    Equirectangular { central_longitude: f64 },
    Orthographic { longitude: f64, latitude: f64 },
}
impl Projection {
    pub fn project(self, lonlat: [f64; 2]) -> Option<[f64; 2]> {
        let [lon, lat] = lonlat;
        if !lon.is_finite() || !lat.is_finite() || lat.abs() > 90.0 {
            return None;
        }
        match self {
            Self::Hammer { central_longitude, west_positive } => {
                let raw = lon-central_longitude;
                let mut d = (raw+180.).rem_euclid(360.)-180.;
                if d == -180. && raw > 0. { d = 180.; }
                if west_positive { d = -d; }
                let a = d.to_radians()/2.;
                let p = lat.to_radians();
                let denominator = (1.+p.cos()*a.cos()).sqrt();
                Some([2.*2_f64.sqrt()*p.cos()*a.sin()/denominator,
                    2_f64.sqrt()*p.sin()/denominator])
            }
            Self::Equirectangular { central_longitude } => Some([
                (lon - central_longitude + 180.0).rem_euclid(360.0) - 180.0,
                lat,
            ]),
            Self::Orthographic {
                longitude,
                latitude,
            } => {
                let d = (lon - longitude).to_radians();
                let p = lat.to_radians();
                let p0 = latitude.to_radians();
                let depth = p0.sin() * p.sin() + p0.cos() * p.cos() * d.cos();
                (depth >= -1e-12).then_some([
                    p.cos() * d.sin(),
                    p0.cos() * p.sin() - p0.sin() * p.cos() * d.cos(),
                ])
            }
        }
    }
    pub fn inverse(self, xy: [f64; 2]) -> Option<[f64; 2]> {
        let [x, y] = xy;
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        match self {
            Self::Hammer { central_longitude, west_positive } => {
                if x*x/8.+y*y/2. > 1.+1e-12 { return None; }
                let z = (1.-x*x/16.-y*y/4.).max(0.).sqrt();
                let mut d = 2.*(z*x).atan2(2.*(2.*z*z-1.));
                if west_positive { d = -d; }
                Some([(central_longitude+d.to_degrees()).rem_euclid(360.),
                    (z*y).clamp(-1.,1.).asin().to_degrees()])
            }
            Self::Equirectangular { central_longitude } => {
                (y.abs() <= 90.0).then_some([x + central_longitude, y])
            }
            Self::Orthographic {
                longitude,
                latitude,
            } => {
                let r = x.hypot(y);
                if r > 1.0 + 1e-12 {
                    return None;
                }
                if r < 1e-14 {
                    return Some([longitude, latitude]);
                }
                let c = r.min(1.0).asin();
                let p0 = latitude.to_radians();
                let lat = (c.cos() * p0.sin() + y * c.sin() * p0.cos() / r)
                    .clamp(-1.0, 1.0)
                    .asin();
                let lon = longitude.to_radians()
                    + (x * c.sin()).atan2(r * p0.cos() * c.cos() - y * p0.sin() * c.sin());
                Some([lon.to_degrees(), lat.to_degrees()])
            }
        }
    }
    /// Densify geographic segments before clipping the horizon or map seam.
    /// NaNs break paths, preserving missing regions rather than joining them.
    pub fn line(self, coordinates: &[[f64; 2]], color: &str) -> Line {
        let mut points = Vec::new();
        let mut previous: Option<[f64; 2]> = None;
        let mut previous_longitude: Option<f64> = None;
        for segment in coordinates.windows(2) {
            let a = segment[0];
            let b = segment[1];
            let dl = (b[0] - a[0] + 180.0).rem_euclid(360.0) - 180.0;
            let n = (dl.abs().max((b[1] - a[1]).abs()) / 0.25).ceil().max(1.0) as usize;
            for k in 0..=n {
                let t = k as f64 / n as f64;
                match self.project([a[0] + dl * t, a[1] + (b[1] - a[1]) * t]) {
                    Some(p) => {
                        if let Self::Hammer { central_longitude, .. } = self {
                            let longitude = (a[0]+dl*t-central_longitude+180.).rem_euclid(360.)-180.;
                            if previous_longitude.is_some_and(|last| (longitude-last).abs() > 180.) {
                                points.push([f64::NAN; 2]);
                            }
                            previous_longitude = Some(longitude);
                        }
                        if previous.is_some_and(|q| {
                            matches!(self, Self::Equirectangular { .. })
                                && (p[0] - q[0]).abs() > 180.0
                        }) {
                            points.push([f64::NAN; 2]);
                        }
                        points.push(p);
                        previous = Some(p);
                    }
                    None => {
                        points.push([f64::NAN; 2]);
                        previous = None;
                    }
                }
            }
        }
        Line::new(points, color)
    }
    pub fn graticule(self, spacing: f64, color: &str) -> Vec<Line> {
        assert!(spacing > 0.0 && spacing.is_finite());
        let mut lines = Vec::new();
        let mut lon = -180.0;
        while lon < 180.0 {
            lines.push(self.line(&[[lon, -90.0], [lon, 90.0]], color));
            lon += spacing;
        }
        let mut lat = -90.0 + spacing;
        while lat < 90.0 {
            let ring: Vec<_> = (0..=360).map(|k| [k as f64 - 180.0, lat]).collect();
            lines.push(self.line(&ring, color));
            lat += spacing;
        }
        lines
    }

    /// A full parallel with the seam at the projection's own antimeridian.
    pub fn parallel(self, latitude: f64, color: &str) -> Line {
        let center = match self {
            Self::Hammer { central_longitude, .. } | Self::Equirectangular { central_longitude } => central_longitude,
            Self::Orthographic { longitude, .. } => longitude,
        };
        self.line(&(0..=1440).map(|i| [center-180.+i as f64/4., latitude]).collect::<Vec<_>>(), color)
    }

    pub fn meridian(self, longitude: f64, color: &str) -> Line {
        self.line(&[[longitude,-90.],[longitude,90.]], color)
    }

    /// Native projection frame; no external plotting-library path input.
    pub fn boundary(self, color: &str) -> Line {
        match self {
            Self::Hammer { .. } => Line::new((0..=1440).map(|i| {
                let a = i as f64*std::f64::consts::TAU/1440.;
                [2.*2_f64.sqrt()*a.cos(), 2_f64.sqrt()*a.sin()]
            }).collect(), color),
            Self::Orthographic { .. } => Line::new((0..=1440).map(|i| {
                let a = i as f64*std::f64::consts::TAU/1440.; [a.cos(),a.sin()]
            }).collect(), color),
            Self::Equirectangular { .. } => Line::new(vec![[-180.,-90.],[180.,-90.],[180.,90.],[-180.,90.],[-180.,-90.]], color),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hammer_round_trip_orientation_and_boundary() {
        let p = Projection::Hammer {central_longitude:270.,west_positive:true};
        for longitude in [0.,60.,90.,120.,180.,240.,270.,300.,359.] {
            for latitude in [-85.,-45.,0.,45.,85.] {
                let xy = p.project([longitude,latitude]).unwrap();
                assert!(xy[0]*xy[0]/8.+xy[1]*xy[1]/2. <= 1.+1e-12);
                let q = p.inverse(xy).unwrap();
                let error = (q[0]-longitude+180.).rem_euclid(360.)-180.;
                assert!(error.abs() < 1e-9 && (q[1]-latitude).abs() < 1e-9);
            }
        }
        assert!(p.project([0.,0.]).unwrap()[0] < 0.);
        assert_eq!(p.project([270.,0.]).unwrap(),[0.,0.]);
        assert!(p.project([180.,0.]).unwrap()[0] > 0.);
        assert!(p.inverse([3.,0.]).is_none());
        for point in p.boundary("black").points {
            assert!((point[0]*point[0]/8.+point[1]*point[1]/2.-1.).abs() < 1e-12);
        }
    }
    #[test]
    fn round_trip_and_horizon() {
        let p = Projection::Orthographic {
            longitude: -60.0,
            latitude: 40.0,
        };
        for q in [[-60.0, 40.0], [-30.0, 10.0], [-90.0, 60.0]] {
            let r = p.inverse(p.project(q).unwrap()).unwrap();
            assert!((r[0] - q[0]).abs() < 1e-10 && (r[1] - q[1]).abs() < 1e-10);
        }
        assert!(p.project([120.0, -40.0]).is_none());
        assert!(p.inverse([2.0, 0.0]).is_none());
    }
    #[test]
    fn antimeridian_break() {
        let p = Projection::Equirectangular {
            central_longitude: 0.0,
        };
        let l = p.line(&[[179.0, 0.0], [-179.0, 0.0]], "black");
        assert!(l.points.iter().any(|q| q[0].is_nan()));
    }
}

/// A vertical section following a spherical Earth, with distance measured along
/// the reference surface. Returns shared-corner quadrilaterals in kilometres.
/// The radial geometry is physical; no vertical exaggeration is applied.
pub fn curved_section(
    radius_km: f64,
    distance_edges: &[f64],
    altitude_edges: &[f64],
    values: Vec<f64>,
    scale: usize,
) -> anyhow::Result<crate::QuadMesh> {
    anyhow::ensure!(
        radius_km.is_finite() && radius_km > 0.,
        "Invalid reference radius"
    );
    anyhow::ensure!(
        distance_edges.len() > 1
            && altitude_edges.len() > 1
            && distance_edges.windows(2).all(|p| p[1] > p[0])
            && altitude_edges.windows(2).all(|p| p[1] > p[0]),
        "Section coordinates must increase"
    );
    anyhow::ensure!(
        distance_edges
            .iter()
            .chain(altitude_edges)
            .all(|x| x.is_finite())
            && altitude_edges[0] + radius_km > 0.,
        "Invalid section coordinates"
    );
    let centre = (distance_edges[0] + distance_edges[distance_edges.len() - 1]) * 0.5;
    let corners = altitude_edges
        .iter()
        .flat_map(|h| {
            distance_edges.iter().map(move |d| {
                let a = (d - centre) / radius_km;
                [
                    (radius_km + h) * a.sin(),
                    (radius_km + h) * a.cos() - radius_km,
                ]
            })
        })
        .collect();
    crate::QuadMesh::new(
        distance_edges.len() - 1,
        altitude_edges.len() - 1,
        corners,
        values,
        scale,
    )
}
