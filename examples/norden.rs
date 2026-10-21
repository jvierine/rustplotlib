//! Rebuild the NORDEN observation/model comparison from its archived HDF5 data.
//! Usage: cargo run --release --features hdf5-examples --example norden -- PROPOSAL_ROOT
//! Requires Ghostscript to normalize PDF resources for XeLaTeX inclusion.
//! No observations are smoothed, inferred from colours, or replaced with synthetic data.
use anyhow::{Context, Result, ensure};
use rustplotlib::*;
use std::{fs, path::Path};

const VIEW: Projection = Projection::Orthographic {
    longitude: 20.,
    latitude: 60.,
};
const LIMIT: f64 = 0.031;
const TEAL: &str = "#007f83";
// Screen-space crop matches the author's selected view without changing camera
// or map scale. All layers, including the altitude curtain, share this crop.
const MAP_BOTTOM: f64 = -0.50;
const MAP_HEIGHT: f64 = 66. * (1.015 - MAP_BOTTOM) / 2.03;
const SAVED_HEIGHT: f64 = 66. - MAP_HEIGHT;

struct Grid {
    lat: Vec<f64>,
    lon: Vec<f64>,
    data: Vec<f64>,
    channels: usize,
}
impl Grid {
    fn read(path: &Path, key: &str, channels: usize) -> Result<Self> {
        let f = hdf5::File::open(path)?;
        let g = Self {
            lat: f.dataset("latitude_deg")?.read_raw()?,
            lon: f.dataset("longitude_deg")?.read_raw()?,
            data: f.dataset(key)?.read_raw()?,
            channels,
        };
        ensure!(
            g.data.len() == g.lat.len() * g.lon.len() * channels,
            "Grid shape mismatch: {key}"
        );
        Ok(g)
    }
    fn sample(&self, lon: f64, lat: f64, channel: usize, periodic: bool) -> f64 {
        let y = ((lat - self.lat[0]) / (self.lat[1] - self.lat[0]))
            .clamp(0., (self.lat.len() - 1) as f64);
        let mut x = (lon - self.lon[0]) / (self.lon[1] - self.lon[0]);
        if periodic {
            x = x.rem_euclid(self.lon.len() as f64);
        } else {
            x = x.clamp(0., (self.lon.len() - 1) as f64);
        }
        let (i, j) = (x.floor() as usize, y.floor() as usize);
        let i1 = if periodic {
            (i + 1) % self.lon.len()
        } else {
            (i + 1).min(self.lon.len() - 1)
        };
        let j1 = (j + 1).min(self.lat.len() - 1);
        let (u, v) = (x - i as f64, y - j as f64);
        let at = |a, b| self.data[(b * self.lon.len() + a) * self.channels + channel];
        // Same bilinear interpolation as the archived figure, including NaN masks.
        (1. - v) * ((1. - u) * at(i, j) + u * at(i1, j))
            + v * ((1. - u) * at(i, j1) + u * at(i1, j1))
    }
}

fn line(panel: &mut Panel, points: Vec<[f64; 2]>, color: &str, width: f64) {
    let mut l = Line::new(points, color);
    l.width_pt = width;
    panel.push(Layer::Line(l));
}
fn geography(panel: &mut Panel, root: &Path) -> Result<()> {
    for lat in [0., 30., 60.] {
        let pts: Vec<_> = (0..=720).map(|i| [-180. + i as f64 * 0.5, lat]).collect();
        let mut l = VIEW.line(&pts, "#a5b0b6");
        l.width_pt = 0.25;
        panel.push(Layer::Line(l));
    }
    for lon in (-180..180).step_by(60) {
        let mut l = VIEW.line(&[[lon as f64, -90.], [lon as f64, 90.]], "#a5b0b6");
        l.width_pt = 0.25;
        panel.push(Layer::Line(l));
    }
    let geo: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        root.join("assets/tiegcm/ne_110m_coastline.geojson"),
    )?)?;
    for f in geo["features"].as_array().context("Missing coastlines")? {
        let g = &f["geometry"];
        let sets = if g["type"] == "MultiLineString" {
            g["coordinates"].as_array().unwrap().clone()
        } else {
            vec![g["coordinates"].clone()]
        };
        for set in sets {
            let pts: Vec<_> = set
                .as_array()
                .unwrap()
                .iter()
                .map(|p| [p[0].as_f64().unwrap(), p[1].as_f64().unwrap()])
                .collect();
            let mut l = VIEW.line(&pts, "#52646d");
            l.width_pt = 0.38;
            panel.push(Layer::Line(l));
        }
    }
    line(
        panel,
        (0..=720)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 720.;
                [a.cos(), a.sin()]
            })
            .collect(),
        "#7e8e96",
        0.45,
    );
    Ok(())
}
fn globe(rect: Rect) -> Panel {
    let mut p = Panel::new(rect, Axes::new([-1.015, 1.015], [MAP_BOTTOM, 1.015], "", ""));
    p.axes_visible = false;
    p
}
fn equivalent_currents(panel: &mut Panel, root: &Path, qd: &Grid) -> Result<(Vec<u64>, Vec<f64>)> {
    let f = hdf5::File::open(root.join("data/smile_equivalent_currents_20260724_0030.h5"))?;
    let lat = f.dataset("latitude_deg")?.read_raw::<f64>()?;
    let lon = f.dataset("longitude_deg")?.read_raw::<f64>()?;
    let north = f.dataset("current_north_A_per_km")?.read_raw::<f64>()?;
    let east = f.dataset("current_east_A_per_km")?.read_raw::<f64>()?;
    let mut ids = vec![];
    let mut vectors = vec![];
    for i in 0..lat.len() {
        // Uniform geographic thinning, not selection by current strength.
        // Only overlay the high-latitude UVI region. No opaque backing or halos.
        if lat[i] < 60. || qd.sample(lon[i], lat[i], 0, false) < 60.
            || ((lat[i] - 46.) / 2.).round() as i32 % 2 != 0
            || ((lon[i] + 75.) / 5.).round() as i32 % 2 != 0 { continue; }
        let Some(p) = VIEW.project([lon[i], lat[i]]) else { continue };
        let (la, dl) = (lat[i].to_radians(), (lon[i] - 20.).to_radians());
        let (s0, c0) = (60f64.to_radians().sin(), 60f64.to_radians().cos());
        // Orthographic projection of local north/east tangents. Same 0.4 km
        // per A/km vector scale as juha.no/smile, drawn on the map surface.
        let k = 0.4 / 6371.;
        let dx = k * (-north[i] * la.sin() * dl.sin() + east[i] * dl.cos());
        let dy = k * (north[i] * (c0 * la.cos() + s0 * la.sin() * dl.cos())
            + east[i] * s0 * dl.sin());
        let length = dx.hypot(dy);
        if length < 1e-12 { continue; }
        let tip = [p[0] + dx, p[1] + dy];
        let head = (length * 0.35).min(0.018);
        let (ux, uy) = (dx / length, dy / length);
        line(panel, vec![p, tip], "#246b64", 0.40);
        line(panel, vec![
            [tip[0] - head * ux - 0.45 * head * uy, tip[1] - head * uy + 0.45 * head * ux],
            tip,
            [tip[0] - head * ux + 0.45 * head * uy, tip[1] - head * uy - 0.45 * head * ux],
        ], "#246b64", 0.40);
        ids.push(i as u64);
        vectors.extend([p[0], p[1], dx, dy]);
    }
    Ok((ids, vectors))
}
fn decimal_year(unix: f64) -> f64 {
    let mut days = unix / 86400.;
    let mut year = 1970;
    loop {
        let n = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
            366.
        } else {
            365.
        };
        if days < n {
            return year as f64 + days / n;
        }
        days -= n;
        year += 1;
    }
}
fn main() -> Result<()> {
    let arg = std::env::args()
        .nth(1)
        .context("Supply the NORDEN proposal directory")?;
    let root = Path::new(&arg).canonicalize()?;
    let obs = root.join("data/smile_tec_20260724_30_dtec1hr.h5");
    let uvi = Grid::read(&obs, "uvi_display_rgb", 3)?;
    let qd = Grid {
        lat: (-90..=90).map(|v| v as f64).collect(),
        lon: (-180..=180).map(|v| v as f64).collect(),
        data: hdf5::File::open(&obs)?
            .dataset("quasi_dipole_latitude_deg")?
            .read_raw()?,
        channels: 1,
    };
    let model_path = root.join("data/norden_panel_120min.h5");
    let model = Grid::read(&model_path, "dtec_tecu", 1)?;
    ensure!(
        hdf5::File::open(&model_path)?
            .attr("elapsed_minute")?
            .read_raw::<f64>()?
            == vec![120.],
        "Wrong model frame"
    );
    let heat = Grid::read(
        &root.join("data/smile_mage_tiegcm_20260724_0030.h5"),
        "joule_heating_mw_m2",
        1,
    )?;
    ensure!(
        heat.data.iter().all(|v| v.is_finite() && *v >= 0.),
        "Invalid heating field"
    );
    let mut fig = Figure::new(IntendedUse::Custom {
        width_mm: 170.,
        font_pt: 9.,
    });
    fig.height_mm = 127. - SAVED_HEIGHT;
    fig.raster_dpi = 600.;
    fig.tight_export = false;
    fig.latex.cache = root.join("tmp/rustplotlib-latex");
    // Keep the default LaTeX text and math fonts, matching the publication examples.
    let tec = fig.scale(Scale::new(
        [-LIMIT, LIMIT],
        Normalization::Linear,
        ColorMap::seismic(),
    )?);
    let heating = fig.scale(Scale::new(
        [0., 20.],
        Normalization::Linear,
        ColorMap::inferno(),
    )?);
    let regional = fig.scale(Scale::new(
        [0.1, 10.],
        Normalization::Log,
        ColorMap::inferno(),
    )?);
    let mut left = globe(Rect::new(8., 9., 66., MAP_HEIGHT));
    let mut right = globe(Rect::new(92., 9., 66., MAP_HEIGHT));
    let n = 1600;
    let rows = (n as f64 * MAP_HEIGHT / 66.).round() as u32;
    let mut observed = image::RgbaImage::new(n, rows);
    let mut simulated = image::RgbaImage::new(n, rows);
    for y in 0..rows {
        for x in 0..n {
            let xy = [
                -1.015 + (x as f64 + 0.5) / n as f64 * 2.03,
                1.015 - (y as f64 + 0.5) / rows as f64 * (1.015 - MAP_BOTTOM),
            ];
            let Some([lon, lat]) = VIEW.inverse(xy) else {
                continue;
            };
            let lon = (lon + 180.).rem_euclid(360.) - 180.;
            let polar = qd.sample(lon, lat, 0, false) >= 60.;
            let mut color = [237, 240, 242, 255];
            if polar {
                let rgb: [f64; 3] = std::array::from_fn(|k| uvi.sample(lon, lat, k, false));
                if rgb.iter().all(|v| v.is_finite()) {
                    for k in 0..3 {
                        color[k] = (rgb[k].clamp(0., 1.) * 255.).round() as u8;
                    }
                }
            }
            observed.put_pixel(x, y, image::Rgba(color));
            let c = if polar {
                fig.scales[heating].map(heat.sample(lon, lat, 0, true))
            } else {
                fig.scales[tec].map(model.sample(lon, lat, 0, true))
            };
            if let Some(c) = c {
                simulated.put_pixel(x, y, image::Rgba(c));
            }
        }
    }
    left.push(Layer::Image(observed));
    right.push(Layer::Image(simulated));
    let hf = hdf5::File::open(root.join("data/heimdall_dtec_20260908_0049.h5"))?;
    let corners = hf.dataset("tec_corners_lonlat_deg")?.read_raw::<f64>()?;
    let values = hf.dataset("dtec_tecu")?.read_raw::<f64>()?;
    ensure!(
        corners.len() == values.len() * 8,
        "Invalid TEC corner shape"
    );
    let mut polys = vec![];
    let mut shown = vec![];
    let mut ids = vec![];
    for (i, (cell, &v)) in corners.chunks_exact(8).zip(&values).enumerate() {
        let pts: Vec<_> = cell.chunks_exact(2).map(|p| [p[0], p[1]]).collect();
        let lat = pts.iter().map(|p| p[1]).sum::<f64>() / 4.;
        let lon = pts
            .iter()
            .map(|p| p[0].to_radians().sin())
            .sum::<f64>()
            .atan2(pts.iter().map(|p| p[0].to_radians().cos()).sum::<f64>())
            .to_degrees();
        let visible = pts.iter().all(|p| {
            let (a, b) = (p[1].to_radians(), (p[0] - 20.).to_radians());
            60f64.to_radians().sin() * a.sin() + 60f64.to_radians().cos() * a.cos() * b.cos()
                > 0.005
        });
        if visible && v.is_finite() && qd.sample(lon, lat, 0, false).abs() < 60. {
            polys.push(pts.iter().map(|&p| VIEW.project(p).unwrap()).collect());
            shown.push(v);
            ids.push(i as u64);
        }
    }
    left.push(Layer::PolygonMesh(PolygonMesh::new(
        polys,
        shown.clone(),
        tec,
    )?));
    geography(&mut left, &root)?;
    geography(&mut right, &root)?;
    let (current_ids, current_vectors) = equivalent_currents(&mut left, &root, &qd)?;
    let orbit_file = hdf5::File::open(root.join("data/aissat1_figure_orbit.h5"))?;
    let orbit_lon = orbit_file.dataset("longitude_deg")?.read_raw::<f64>()?;
    let orbit_lat = orbit_file
        .dataset("geocentric_latitude_deg")?
        .read_raw::<f64>()?;
    let orbit: Vec<_> = orbit_lon
        .iter()
        .zip(&orbit_lat)
        .map(|(&lo, &la)| [lo, la])
        .collect();
    // White casing keeps the real ground track visible over both scientific layers.
    for (color, width) in [("#ffffff", 2.2), ("#145b7a", 1.1)] {
        let mut track = VIEW.line(&orbit, color);
        track.width_pt = width;
        left.push(Layer::Line(track));
    }
    let orbit_anchor = orbit
        .iter()
        .filter_map(|&p| VIEW.project(p))
        .min_by(|a, b| a[0].total_cmp(&b[0]))
        .context("No visible orbit")?;
    let footprint: Vec<_> = [[17., 68.], [17., 72.], [27., 72.], [27., 68.], [17., 68.]]
        .into_iter()
        .map(|p| VIEW.project(p).unwrap())
        .collect();
    line(&mut right, footprint, TEAL, 1.0);
    // Real paired TIE-GCM experiment, displayed as a separate altitude curtain.
    // This placement is illustrative, not co-registration with the MAGE globe.
    let hf = hdf5::File::open(root.join("data/tiegcm_meridian_120min.h5"))?;
    let lat = hf.dataset("latitude_edges_deg")?.read_raw::<f64>()?;
    let alt = hf.dataset("altitude_edges_km")?.read_raw::<f64>()?;
    let vals = hf.dataset("delta_temperature_K")?.read_raw::<f64>()?;
    let temperature = fig.scale(Scale::new(
        [-12.5, 12.5],
        Normalization::Linear,
        ColorMap::seismic(),
    )?);
    let ax = Axes::new([-1.445, 1.015], [MAP_BOTTOM, 1.015], "", "");
    let mut curtain = Panel::new(Rect::new(78.02, 9., 79.98, MAP_HEIGHT), ax);
    curtain.axes_visible = false;
    let mut corners = Vec::new();
    for &z in &alt {
        for &la in &lat {
            let theta = (la - 60.).to_radians();
            let r = 1.025 + (z - 100.) / 400. * 0.30;
            corners.push([-r * theta.cos(), r * theta.sin()]);
        }
    }
    curtain.push(Layer::QuadMesh(QuadMesh::new(
        lat.len() - 1,
        alt.len() - 1,
        corners,
        vals,
        temperature,
    )?));
    fig.panels.extend([left, right, curtain]);
    fig.labels.push(Label::new(r"TIE-GCM", 87., 12.));
    fig.labels.push(Label::new(r"120 min", 86., 17.));
    fig.labels.push(Label::new(r"100--500 km", 87., 22.));
    let mut cb = Colorbar::new(
        Rect::new(77., 78. - SAVED_HEIGHT, 16., 1.6),
        temperature,
        r"$\Delta T$ (K)",
        vec![-12.5, 0., 12.5],
    );
    cb.horizontal = true;
    fig.colorbars.push(cb);

    fig.labels.push(Label::new(
        r"\textbf{a)\quad SMILE UVI precipitation and GNSS TEC LSTIDs}",
        41.,
        4.,
    ));
    fig.labels.push(Label::new(
        r"\textbf{b)\quad MAGE Joule heating and LSTADs}",
        125.,
        4.,
    ));
    let mut e = Label::new("EISCAT3D", 146., 34.);
    e.color = TEAL.into();
    fig.labels.push(e);
    for (rect, id, label, ticks) in [
        (
            Rect::new(15., 78. - SAVED_HEIGHT, 50., 1.6),
            tec,
            r"$\delta\mathrm{TEC}$ (TECU; 10 min)",
            vec![-LIMIT, 0., LIMIT],
        ),
        (
            Rect::new(100., 78. - SAVED_HEIGHT, 51., 1.6),
            heating,
            r"Joule heating ($\mathrm{mW\,m^{-2}}$)",
            vec![0., 10., 20.],
        ),
    ] {
        let mut b = Colorbar::new(rect, id, label, ticks);
        b.horizontal = true;
        fig.colorbars.push(b);
    }
    let h = hdf5::File::open(root.join("data/aissat1_decay.h5"))?;
    let times = h.dataset("time_unix_s")?.read_raw::<f64>()?;
    let decay = h.dataset("da_dt_m_per_day")?.read_raw::<f64>()?;
    let points: Vec<_> = times
        .iter()
        .zip(&decay)
        .filter(|(t, v)| **t >= 1279497600. && v.is_finite())
        .map(|(t, v)| [decimal_year(*t), *v])
        .collect();
    let mut axes = Axes::new(
        [points[0][0], points.last().unwrap()[0]],
        [-260., 12.],
        "Year",
        r"$da/dt\;(\mathrm{m\,day^{-1}})$",
    );
    axes.xticks = vec![2015., 2020., 2025.];
    axes.yticks = vec![-200., -100., 0.];
    let mut drag = Panel::new(Rect::new(15., 99. - SAVED_HEIGHT, 63., 17.), axes);
    for y in [-200., -100., 0.] {
        line(&mut drag, vec![[2010., y], [2026., y]], "#d7dfe3", 0.3);
    }
    line(&mut drag, points.clone(), "#145b7a", 0.55);
    fig.panels.push(drag);
    fig.connectors.push(Connector {
        from_panel: 0,
        from: orbit_anchor,
        to_panel: 3,
        to: [points[0][0], 12.],
        via: vec![
            [3., 9. + (1.015 - orbit_anchor[1]) / 2.03 * 66.],
            [3., 90. - SAVED_HEIGHT],
            [15., 90. - SAVED_HEIGHT],
            [15., 98. - SAVED_HEIGHT],
        ],
        color: "#145b7a".into(),
        width_pt: 0.6,
    });
    fig.labels.push(Label::new(
        r"\textbf{c)\quad AISSat-1 orbital decay}",
        46.5,
        94.5 - SAVED_HEIGHT,
    ));
    let mut ax = Axes::new(
        [0., 24.],
        [70.7, 78.9],
        "7--8 Sep 2017 (UT)",
        r"Latitude ($^\circ$)",
    );
    ax.xticks = vec![0., 6., 12., 18., 24.];
    ax.xticklabels = Some(
        ["06", "12", "18", "00", "06"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    ax.yticks = vec![71., 75., 78.];
    let mut radar = Panel::new(Rect::new(100., 99. - SAVED_HEIGHT, 51., 17.), ax);
    radar.push(Layer::Image(
        image::open(root.join("assets/lokke_joule_heating_inset.png"))?.to_rgba8(),
    ));
    fig.panels.push(radar);
    fig.labels.push(Label::new(
        r"\textbf{d)\quad EISCAT VHF heating}",
        125.5,
        94.5 - SAVED_HEIGHT,
    ));
    fig.colorbars.push(Colorbar::new(
        Rect::new(154., 99. - SAVED_HEIGHT, 1.5, 17.),
        regional,
        r"$\mathrm{mW\,m^{-2}}$",
        vec![0.1, 1., 10.],
    ));
    fig.connectors.push(Connector {
        from_panel: 1,
        from: VIEW.project([22., 70.]).unwrap(),
        to_panel: 4,
        to: [24., 78.9],
        via: vec![
            [158., 38.],
            [166., 38.],
            [166., 94. - SAVED_HEIGHT],
            [154., 94. - SAVED_HEIGHT],
            [154., 98. - SAVED_HEIGHT],
        ],
        color: TEAL.into(),
        width_pt: 0.55,
    });
    let out = root.join("tmp/pdfs");
    fs::create_dir_all(&out)?;
    let native_pdf = out.join("figure1-native.pdf");
    fig.save(&native_pdf)?;
    // Some xdvipdfmx versions abort on ICC resources in svg2pdf output.
    // Normalize to device RGB, preserving vectors and raster resolution.
    let status = std::process::Command::new("gs")
        .args([
            "-q",
            "-dBATCH",
            "-dNOPAUSE",
            "-sDEVICE=pdfwrite",
            "-dCompatibilityLevel=1.5",
            "-sColorConversionStrategy=RGB",
            "-dProcessColorModel=/DeviceRGB",
            "-dOverrideICC",
            "-dAutoRotatePages=/None",
            "-dDownsampleColorImages=false",
            "-dDownsampleGrayImages=false",
            "-dDownsampleMonoImages=false",
        ])
        .arg(format!(
            "-sOutputFile={}",
            root.join("norden_smile_mage_globes.pdf").display()
        ))
        .arg(&native_pdf)
        .status()
        .context("Ghostscript (gs) is required for XeLaTeX-compatible PDF export")?;
    ensure!(status.success(), "PDF resource normalization failed");
    fig.save(out.join("norden_smile_mage_globes.png"))?;
    fig.save(out.join("norden_smile_mage_globes.svg"))?;
    let audit = hdf5::File::create(out.join("figure1-rustplotlib-validation.h5"))?;
    audit.new_dataset_builder().with_data(&current_ids).create("equivalent_current_source_indices")?;
    audit.new_dataset_builder().with_data(&current_vectors).create("equivalent_current_projected_vectors")?;
    audit.new_dataset_builder().with_data(&[MAP_BOTTOM, 1.015]).create("map_y_limits")?;
    audit.new_dataset_builder().with_data(&[170., fig.height_mm]).create("figure_size_mm")?;
    audit
        .new_dataset_builder()
        .with_data(&ids)
        .create("observed_tec_source_indices")?;
    audit
        .new_dataset_builder()
        .with_data(&shown)
        .create("observed_dtec_tecu")?;
    audit
        .new_dataset_builder()
        .with_data(&points.iter().map(|p| p[1]).collect::<Vec<_>>())
        .create("plotted_da_dt_m_per_day")?;
    audit
        .new_dataset_builder()
        .with_data(&[20., 60.])
        .create("view_longitude_latitude_deg")?;
    // Fixed probes cover longitude wrapping and polar grid extension, so the
    // proposal can compare interpolation with the previous SciPy renderer.
    let probes = [
        [-179.9, 89.],
        [179.9, 59.8],
        [20., 70.],
        [10.5, 45.2],
        [-75., -89.],
    ];
    for (name, grid, periodic) in [
        ("model_dtec_probes", &model, true),
        ("model_heating_probes", &heat, true),
        ("qd_latitude_probes", &qd, false),
    ] {
        audit
            .new_dataset_builder()
            .with_data(
                &probes
                    .iter()
                    .map(|p| grid.sample(p[0], p[1], 0, periodic))
                    .collect::<Vec<_>>(),
            )
            .create(name)?;
    }
    audit
        .new_dataset_builder()
        .with_data(&probes.iter().flatten().copied().collect::<Vec<_>>())
        .create("probe_lonlat_deg")?;
    println!(
        "Rendered {} observed TEC cells and {} unchanged decay samples; cropped globes with equivalent currents",
        ids.len(),
        points.len()
    );
    Ok(())
}
