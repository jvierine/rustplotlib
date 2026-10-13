use anyhow::Result;
use rustplotlib::*;
use std::{fs, path::PathBuf};
// The private solver is not included: this example plots HDF5 model outputs.
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let input = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("examples/support/syisr_khi.h5");
    let out = PathBuf::from(
        args.get(2)
            .map(String::as_str)
            .unwrap_or("output/syisr-khi"),
    );
    let h = hdf5::File::open(input)?;
    let frames = h.group("frames")?;
    let mut keys = frames.member_names()?;
    keys.sort();
    let xedges = h.dataset("x_edges_km")?.read_raw::<f64>()?;
    let zedges = h.dataset("z_edges_km")?.read_raw::<f64>()?;
    let z = h.dataset("z_center_km")?.read_raw::<f64>()?;
    let wind = h.dataset("background_wind_m_s")?.read_raw::<f64>()?;
    let limit = (h.attr("ion_velocity_max_abs_m_s")?.read_scalar::<f64>()? / 5.)
        .ceil()
        .max(1.)
        * 5.;
    for mode in ["ne", "ion_up"] {
        fs::create_dir_all(out.join(mode))?;
    }
    for (k, key) in keys.iter().enumerate() {
        let g = frames.group(key)?;
        let t = g.attr("time_s")?.read_scalar::<f64>()?;
        for mode in ["ne", "ion_up"] {
            let values = g
                .dataset(if mode == "ne" {
                    "electron_density_m3"
                } else {
                    "ion_up_m_s"
                })?
                .read_raw::<f32>()?
                .iter()
                .map(|&v| v as f64)
                .collect();
            let mut fig = Figure::new(IntendedUse::ArticleWide);
            fig.height_mm = 105.;
            fig.raster_dpi = 180.;
            // Animated figures explicitly retain fixed page dimensions to avoid crop jitter.
            fig.tight_export = false;
            let (width, scale, label, ticks) = if mode == "ne" {
                (
                    151.,
                    fig.scale(Scale::new(
                        [1e10, 2e12],
                        Normalization::Log,
                        ColorMap::ember(),
                    )?),
                    r"$n_e\,[\mathrm{m}^{-3}]$",
                    vec![1e10, 1e11, 2e12],
                )
            } else {
                (
                    116.,
                    fig.scale(Scale::new(
                        [-limit, limit],
                        Normalization::Linear,
                        ColorMap::diverging(),
                    )?),
                    r"$v_{i,z}\,[\mathrm{m\,s}^{-1}]$",
                    vec![-limit, 0., limit],
                )
            };
            let mut ax = Axes::new(
                [xedges[0], *xedges.last().unwrap()],
                [zedges[0], *zedges.last().unwrap()],
                r"$x-U_0t\,[\mathrm{km}]$",
                r"$z-z_c(t)\,[\mathrm{km}]$",
            );
            ax.xticks = vec![-80., -40., 0., 40., 80.];
            ax.yticks = vec![-2., -1., 0., 1., 2.];
            let mut panel = Panel::new(Rect::new(19., 10., width, 67.), ax);
            let mut field = Field::new(xedges.clone(), zedges.clone(), values, scale)?;
            field.interpolate = true;
            panel.push(Layer::Pcolormesh(field));
            fig.panels.push(panel);
            let mut bar = Colorbar::new(Rect::new(25., 89., width - 12., 2.), scale, label, ticks);
            bar.horizontal = true;
            fig.colorbars.push(bar);
            fig.labels.push(Label::new(
                format!("$t={:.1}\\,\\mathrm{{min}}$", t / 60.),
                90.,
                4.,
            ));
            if mode == "ion_up" {
                let mut ax = Axes::new(
                    [-180., -20.],
                    [zedges[0], *zedges.last().unwrap()],
                    r"$U\,[\mathrm{m\,s}^{-1}]$",
                    "",
                );
                ax.xticks = vec![-175., -100., -25.];
                ax.yticks = vec![];
                let mut p = Panel::new(Rect::new(145., 10., 29., 67.), ax);
                p.push(Layer::Line(Line::new(
                    wind.iter().zip(&z).map(|(&u, &z)| [u, z]).collect(),
                    "#252525",
                )));
                fig.panels.push(p);
                fig.labels.push(Label::new("Background wind", 159., 4.));
            }
            fig.save(out.join(mode).join(format!("{k:04}.png")))?;
            fig.save(out.join(format!("{mode}_t{:.0}.pdf", t / 60.)))?;
        }
    }
    fs::write(
        out.join("provenance.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"script":"examples/syisr_khi.rs","source":input,"frames":keys.len(),"simulated":true,"observational_fit":false,"velocity":"upward ion drift, model frame; precomputed from magnetic Lorentz-neutral-drag balance","camera":"mean wind comoving","no_mask":true}),
        )?,
    )?;
    println!(
        "Rendered {} HDF5 checkpoints to {}",
        keys.len(),
        out.display()
    );
    Ok(())
}
