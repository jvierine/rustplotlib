// Scientific browser features live in Rustplotlib, not page-specific axes code.
#[derive(Deserialize)]
struct FieldData {
    x: Vec<f64>,
    y: Vec<f64>,
    values: Vec<Option<f64>>,
    limits: [f64; 2],
    #[serde(default = "default_palette")]
    palette: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    missing_value: Option<f64>,
}
fn default_palette() -> String {
    "turbo".into()
}
#[derive(Deserialize)]
struct SkyDensity {
    lonlat: Vec<[f64; 2]>,
    #[serde(default = "default_nside")]
    nside: usize,
    #[serde(default = "default_center")]
    central_longitude: f64,
    #[serde(default = "line_default")]
    west_positive: bool,
    #[serde(default)]
    limits: Option<[f64; 2]>,
}
fn default_nside() -> usize {
    32
}
fn default_center() -> f64 {
    270.
}
fn equal_aspect_rect(rect: [f64; 4], dx: f64, dy: f64) -> [f64; 4] {
    let scale = (rect[2] / dx).min(rect[3] / dy);
    let w = scale * dx;
    let h = scale * dy;
    [
        rect[0] + (rect[2] - w) / 2.,
        rect[1] + (rect[3] - h) / 2.,
        w,
        h,
    ]
}
fn mollweide(lon: f64, lat: f64, center: f64, west: bool) -> [f64; 2] {
    let mut d = (lon - center + 180.).rem_euclid(360.) - 180.;
    if west {
        d = -d;
    }
    let p = lat.clamp(-90., 90.).to_radians();
    let mut t = p;
    if lat.abs() >= 89.999999 {
        t = p.signum() * std::f64::consts::FRAC_PI_2;
    } else {
        for _ in 0..20 {
            let step = (2. * t + (2. * t).sin() - std::f64::consts::PI * p.sin())
                / (2. + 2. * (2. * t).cos());
            t -= step;
            if step.abs() < 1e-12 {
                break;
            }
        }
    }
    [
        2. * 2_f64.sqrt() / std::f64::consts::PI * d.to_radians() * t.cos(),
        2_f64.sqrt() * t.sin(),
    ]
}
fn inverse_mollweide(x: f64, y: f64, center: f64, west: bool) -> Option<[f64; 2]> {
    if x * x / 8. + y * y / 2. > 1. + 1e-10 {
        return None;
    }
    let t = (y / 2_f64.sqrt()).clamp(-1., 1.).asin();
    let d = if t.cos().abs() < 1e-12 {
        0.
    } else {
        std::f64::consts::PI * x / (2. * 2_f64.sqrt() * t.cos())
    };
    if d.abs() > std::f64::consts::PI + 1e-8 {
        return None;
    }
    Some([
        (center
            + if west {
                -d.to_degrees()
            } else {
                d.to_degrees()
            })
        .rem_euclid(360.),
        ((2. * t + (2. * t).sin()) / std::f64::consts::PI)
            .clamp(-1., 1.)
            .asin()
            .to_degrees(),
    ])
}
/// Standard HEALPix RING indexing; nside is a positive power of two.
fn healpix_ring(n: usize, lon: f64, lat: f64) -> usize {
    let n = n as i64;
    let z = lat.to_radians().sin();
    let za = z.abs();
    let tt = lon.rem_euclid(360.) / 90.;
    let np = 12 * n * n;
    let cap = 2 * n * (n - 1);
    if za <= 2. / 3. {
        let a = n as f64 * (0.5 + tt);
        let b = n as f64 * z * 0.75;
        let jp = (a - b).floor() as i64;
        let jm = (a + b).floor() as i64;
        let ir = n + 1 + jp - jm;
        let shift = 1 - (ir & 1);
        let ip = ((jp + jm - n + shift + 1) / 2).rem_euclid(4 * n);
        (cap + (ir - 1) * 4 * n + ip) as usize
    } else {
        let t = tt - tt.floor();
        let tmp = n as f64 * (3. * (1. - za)).sqrt();
        let ir = (t * tmp).floor() as i64 + ((1. - t) * tmp).floor() as i64 + 1;
        let ip = ((tt * ir as f64).floor() as i64).rem_euclid(4 * ir);
        (if z > 0. {
            2 * ir * (ir - 1) + ip
        } else {
            np - 2 * ir * (ir + 1) + ip
        }) as usize
    }
}
fn palette(value: f64, limits: [f64; 2], name: &str) -> String {
    let t = ((value - limits[0]) / (limits[1] - limits[0])).clamp(0., 1.);
    let anchors = if name == "plasma" {
        &[
            [13., 8., 135.],
            [126., 3., 168.],
            [204., 71., 120.],
            [248., 149., 64.],
            [240., 249., 33.],
        ][..]
    } else {
        &[
            [48., 18., 59.],
            [50., 95., 210.],
            [20., 190., 180.],
            [160., 230., 50.],
            [250., 180., 30.],
            [230., 65., 20.],
            [122., 4., 3.],
        ][..]
    };
    let x = t * (anchors.len() - 1) as f64;
    let i = (x.floor() as usize).min(anchors.len() - 2);
    let f = x - i as f64;
    let rgb: Vec<_> = (0..3)
        .map(|j| ((1. - f) * anchors[i][j] + f * anchors[i + 1][j]).round() as u8)
        .collect();
    format!("rgb({},{},{})", rgb[0], rgb[1], rgb[2])
}
impl SeriesView {
    fn fill_polygon(&self, s: &PlotSeries, color: &str) -> Result<(), JsValue> {
        let c = &self.ctx;
        c.save();
        c.begin_path();
        c.rect(self.rect[0], self.rect[1], self.rect[2], self.rect[3]);
        c.clip();
        c.begin_path();
        let mut first = true;
        for p in &s.points {
            if let [Some(x), Some(y)] = *p {
                if first {
                    c.move_to(self.px(x), self.py(y));
                    first = false;
                } else {
                    c.line_to(self.px(x), self.py(y));
                }
            }
        }
        c.close_path();
        c.set_fill_style_str(color);
        c.fill();
        c.restore();
        Ok(())
    }
    fn colorbar(&self, limits: [f64; 2], name: &str, label: &str) -> Result<(), JsValue> {
        let c = &self.ctx;
        let [l, t, w, h] = self.rect;
        let x = l + w + 14.;
        for i in 0..128 {
            c.set_fill_style_str(&palette(
                limits[0] + (limits[1] - limits[0]) * i as f64 / 127.,
                limits,
                name,
            ));
            c.fill_rect(x, t + h * (1. - (i + 1) as f64 / 128.), 12., h / 128. + 1.);
        }
        c.set_font("13px Palatino,serif");
        c.set_fill_style_str("#111");
        c.set_text_align("left");
        for v in nice_ticks(limits[0], limits[1], 4) {
            let y = t + h * (1. - (v - limits[0]) / (limits[1] - limits[0]));
            c.fill_text(&number_label(v, limits[1] - limits[0]), x + 17., y + 4.)?;
        }
        c.save();
        c.translate(self.width - 8., t + h / 2.)?;
        c.rotate(-std::f64::consts::FRAC_PI_2)?;
        c.set_text_align("center");
        c.fill_text(label, 0., 0.)?;
        c.restore();
        Ok(())
    }
    fn draw_field(&self, f: &FieldData) -> Result<(), JsValue> {
        if f.x.len() < 2
            || f.y.len() < 2
            || f.values.len() != (f.x.len() - 1) * (f.y.len() - 1)
            || !f.limits.iter().all(|v| v.is_finite())
            || f.limits[1] <= f.limits[0]
            || !f.x.iter().chain(f.y.iter()).all(|v| v.is_finite())
            || f.x.windows(2).chain(f.y.windows(2)).any(|p| p[1] <= p[0])
        {
            return Err(JsValue::from_str(
                "Invalid rectilinear field dimensions or color limits",
            ));
        }
        let c = &self.ctx;
        c.save();
        c.begin_path();
        c.rect(self.rect[0], self.rect[1], self.rect[2], self.rect[3]);
        c.clip();
        if let Some(v) = f.missing_value {
            c.set_fill_style_str(&palette(v, f.limits, &f.palette));
            c.fill_rect(self.rect[0], self.rect[1], self.rect[2], self.rect[3]);
        }
        for r in 0..f.y.len() - 1 {
            for j in 0..f.x.len() - 1 {
                if let Some(v) = f.values[r * (f.x.len() - 1) + j] {
                    if v.is_finite() {
                        let x = self.px(f.x[j]);
                        let y = self.py(f.y[r + 1]);
                        c.set_fill_style_str(&palette(v, f.limits, &f.palette));
                        c.fill_rect(
                            x,
                            y,
                            self.px(f.x[j + 1]) - x + 0.3,
                            self.py(f.y[r]) - y + 0.3,
                        );
                    }
                }
            }
        }
        c.restore();
        self.colorbar(f.limits, &f.palette, &f.label)
    }
    fn draw_bars(&self, s: &PlotSeries) -> Result<(), JsValue> {
        let c = &self.ctx;
        c.save();
        c.begin_path();
        c.rect(self.rect[0], self.rect[1], self.rect[2], self.rect[3]);
        c.clip();
        c.set_fill_style_str(&s.color);
        for p in &s.points {
            if let [Some(x), Some(y)] = *p {
                let a = self.py(0.);
                let b = self.py(y);
                c.fill_rect(
                    self.px(x - 0.32),
                    a.min(b),
                    self.px(x + 0.32) - self.px(x - 0.32),
                    (a - b).abs().max(1.),
                );
            }
        }
        c.restore();
        Ok(())
    }
    fn draw_errors(&self, s: &PlotSeries) -> Result<(), JsValue> {
        if s.errors.is_empty() {
            return Ok(());
        }
        let c = &self.ctx;
        c.save();
        c.begin_path();
        c.rect(self.rect[0], self.rect[1], self.rect[2], self.rect[3]);
        c.clip();
        c.set_stroke_style_str(&s.color);
        c.set_line_width(s.line_width.max(0.5));
        c.begin_path();
        for (p, e) in s.points.iter().zip(&s.errors) {
            if let ([Some(x), Some(y)], Some(e)) = (p, e) {
                if *e >= 0. {
                    let px = self.px(*x);
                    let a = self.py(y - e);
                    let b = self.py(y + e);
                    c.move_to(px, a);
                    c.line_to(px, b);
                    c.move_to(px - 3., a);
                    c.line_to(px + 3., a);
                    c.move_to(px - 3., b);
                    c.line_to(px + 3., b);
                }
            }
        }
        c.stroke();
        c.restore();
        Ok(())
    }
    fn render_sky(
        &mut self,
        spec: &SeriesSpec,
        sky: &SkyDensity,
        width: f64,
        height: f64,
        dpr: f64,
        viewport: &[f64],
    ) -> Result<Vec<f64>, JsValue> {
        if !sky.nside.is_power_of_two() || sky.nside > 128 || !sky.central_longitude.is_finite() {
            return Err(JsValue::from_str(
                "Invalid sky projection/HEALPix resolution",
            ));
        }
        self.width = width;
        self.height = height;
        self.canvas.set_width((width * dpr).round() as u32);
        self.canvas.set_height((height * dpr).round() as u32);
        let c = &self.ctx;
        c.set_transform(dpr, 0., 0., dpr, 0., 0.)?;
        c.set_fill_style_str("white");
        c.fill_rect(0., 0., width, height);
        let w = (width - 148.).min((height - 75.) * 2.);
        let h = w / 2.;
        self.rect = [52. + (width - 148. - w) / 2., 18., w, h];
        let r = 2_f64.sqrt();
        self.bounds = [
            -2. * r + viewport[0] * 4. * r,
            -2. * r + viewport[2] * 4. * r,
            -r + viewport[1] * 2. * r,
            -r + viewport[3] * 2. * r,
        ];
        let mut bins = vec![0_u32; 12 * sky.nside * sky.nside];
        for [lon, lat] in &sky.lonlat {
            if lon.is_finite() && lat.is_finite() && lat.abs() <= 90. {
                bins[healpix_ring(sky.nside, *lon, *lat)] += 1;
            }
        }
        let lim = sky
            .limits
            .unwrap_or([0., (*bins.iter().max().unwrap_or(&1)).max(1) as f64]);
        if !lim.iter().all(|v| v.is_finite()) || lim[1] <= lim[0] {
            return Err(JsValue::from_str("Invalid sky color limits"));
        }
        self.color_limits = lim;
        let [left, top, _, _] = self.rect;
        for iy in (0..h.ceil() as usize).step_by(2) {
            for ix in (0..w.ceil() as usize).step_by(2) {
                let x = self.bounds[0] + (ix as f64 + 1.) / w * (self.bounds[1] - self.bounds[0]);
                let y = self.bounds[3] - (iy as f64 + 1.) / h * (self.bounds[3] - self.bounds[2]);
                if let Some([lon, lat]) =
                    inverse_mollweide(x, y, sky.central_longitude, sky.west_positive)
                {
                    let count = bins[healpix_ring(sky.nside, lon, lat)];
                    c.set_fill_style_str(&palette(count as f64, lim, "turbo"));
                    c.fill_rect(left + ix as f64, top + iy as f64, 2.1, 2.1);
                }
            }
        }
        c.save();
        c.begin_path();
        c.rect(left, top, w, h);
        c.clip();
        c.set_line_width(0.6);
        c.set_stroke_style_str("#ffffff88");
        for lat in [-60., -30., 0., 30., 60.] {
            c.begin_path();
            for i in 0..=180 {
                let d = -179.999 + i as f64 * 359.998 / 180.;
                let p = mollweide(
                    sky.central_longitude + d,
                    lat,
                    sky.central_longitude,
                    sky.west_positive,
                );
                if i == 0 {
                    c.move_to(self.px(p[0]), self.py(p[1]));
                } else {
                    c.line_to(self.px(p[0]), self.py(p[1]));
                }
            }
            c.stroke();
        }
        for lon in (0..360).step_by(30) {
            c.begin_path();
            for i in 0..=90 {
                let p = mollweide(
                    lon as f64,
                    -89.999 + i as f64 * 179.998 / 90.,
                    sky.central_longitude,
                    sky.west_positive,
                );
                if i == 0 {
                    c.move_to(self.px(p[0]), self.py(p[1]));
                } else {
                    c.line_to(self.px(p[0]), self.py(p[1]));
                }
            }
            c.stroke();
        }
        c.set_font("13px Palatino,serif");
        c.set_fill_style_str("white");
        c.set_text_align("center");
        for (lon, label) in [
            (0., "0°"),
            (90., "90°"),
            (180., "180°"),
            (270., "270° · apex"),
        ] {
            let p = mollweide(lon, 0., sky.central_longitude, sky.west_positive);
            c.fill_text(
                label,
                self.px(p[0]).clamp(left + 18., left + w - 18.),
                self.py(p[1]) - 6.,
            )?;
        }
        c.restore();
        c.set_fill_style_str("#111");
        c.set_font("15px Palatino,serif");
        c.set_text_align("center");
        c.fill_text(&spec.xlabel, left + w / 2., top + h + 30.)?;
        c.save();
        c.translate(15., top + h / 2.)?;
        c.rotate(-std::f64::consts::FRAC_PI_2)?;
        c.fill_text(&spec.ylabel, 0., 0.)?;
        c.restore();
        c.set_text_align("right");
        c.set_font("12px Palatino,serif");
        for lat in [-60., -30., 0., 30., 60.] {
            let p = mollweide(
                sky.central_longitude,
                lat,
                sky.central_longitude,
                sky.west_positive,
            );
            let y = self.py(p[1]);
            if y >= top && y <= top + h {
                c.fill_text(&format!("{lat:.0}°"), left - 5., y + 4.)?;
            }
        }
        self.colorbar(lim, "turbo", "Meteors / HEALPix cell")?;
        Ok(self.rect.to_vec())
    }
}

#[cfg(test)]
mod diagnostics_tests {
    use super::*;
    #[test]
    fn equal_axes_preserve_metric_distances() {
        let r = equal_aspect_rect([20., 10., 400., 200.], 70., 70.);
        assert!((r[2] - r[3]).abs() < 1e-12);
        assert_eq!(r, [120., 10., 200., 200.]);
    }
    #[test]
    fn projection_round_trips_and_apex_is_central() {
        for lat in [-89., -60., 0., 60., 89.] {
            for lon in [0., 45., 90., 180., 270., 359.] {
                let p = mollweide(lon, lat, 270., true);
                let q = inverse_mollweide(p[0], p[1], 270., true).unwrap();
                assert!((q[1] - lat).abs() < 1e-8);
                assert!(((q[0] - lon + 180.).rem_euclid(360.) - 180.).abs() < 1e-8);
            }
        }
        assert!(mollweide(270., 0., 270., true)[0].abs() < 1e-12);
        assert!(mollweide(0., 0., 270., true)[0] < 0.);
    }
    #[test]
    fn healpix_ring_indices_are_bounded_and_periodic() {
        for n in [1, 2, 4, 32] {
            for lat in -90..=90 {
                for lon in 0..360 {
                    let h = healpix_ring(n, lon as f64, lat as f64);
                    assert!(h < 12 * n * n);
                    assert_eq!(h, healpix_ring(n, lon as f64 + 360., lat as f64));
                }
            }
        }
        assert_eq!(healpix_ring(1, 0., 0.), 4);
        assert_eq!(healpix_ring(1, 90., 0.), 5);
    }
}
