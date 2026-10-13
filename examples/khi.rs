//! Periodic incompressible Navier–Stokes in vorticity form, with passive dye.
//! Fourier differentiation, 2/3 dealiasing, explicit viscosity and RK4.
use anyhow::{Result, ensure};
use rustfft::{FftPlanner, num_complex::Complex64 as C};
use rustplotlib::*;
use std::{fs, sync::Arc};
const N: usize = 128;
const NU: f64 = 0.001;
struct Solver {
    forward: Arc<dyn rustfft::Fft<f64>>,
    inverse: Arc<dyn rustfft::Fft<f64>>,
    k: Vec<f64>,
}
impl Solver {
    fn new() -> Self {
        let mut p = FftPlanner::new();
        Self {
            forward: p.plan_fft_forward(N),
            inverse: p.plan_fft_inverse(N),
            k: (0..N)
                .map(|i| {
                    if i <= N / 2 {
                        i as f64
                    } else {
                        i as f64 - N as f64
                    }
                })
                .collect(),
        }
    }
    fn fft(&self, a: &[C], inverse: bool) -> Vec<C> {
        let mut a = a.to_vec();
        let f = if inverse {
            &self.inverse
        } else {
            &self.forward
        };
        for row in a.chunks_mut(N) {
            f.process(row);
        }
        let mut col = vec![C::default(); N];
        for x in 0..N {
            for y in 0..N {
                col[y] = a[y * N + x];
            }
            f.process(&mut col);
            for y in 0..N {
                a[y * N + x] = col[y];
            }
        }
        if inverse {
            for v in &mut a {
                *v /= (N * N) as f64;
            }
        }
        a
    }
    fn filter(&self, a: &mut [C]) {
        for y in 0..N {
            for x in 0..N {
                if self.k[x].abs() > (N / 3) as f64 || self.k[y].abs() > (N / 3) as f64 {
                    a[y * N + x] = C::default();
                }
            }
        }
    }
    fn rhs(&self, w: &[C], d: &[C]) -> (Vec<C>, Vec<C>) {
        let mut u = vec![C::default(); N * N];
        let mut v = u.clone();
        let mut wx = u.clone();
        let mut wy = u.clone();
        let mut dx = u.clone();
        let mut dy = u.clone();
        for y in 0..N {
            for x in 0..N {
                let i = y * N + x;
                let kx = self.k[x];
                let ky = self.k[y];
                let k2 = kx * kx + ky * ky;
                if k2 > 0.0 {
                    u[i] = C::new(0.0, ky / k2) * w[i];
                    v[i] = C::new(0.0, -kx / k2) * w[i];
                }
                wx[i] = C::new(0.0, kx) * w[i];
                wy[i] = C::new(0.0, ky) * w[i];
                dx[i] = C::new(0.0, kx) * d[i];
                dy[i] = C::new(0.0, ky) * d[i];
            }
        }
        let u = self.fft(&u, true);
        let v = self.fft(&v, true);
        let wx = self.fft(&wx, true);
        let wy = self.fft(&wy, true);
        let dx = self.fft(&dx, true);
        let dy = self.fft(&dy, true);
        let adv = |x: &[C], y: &[C]| {
            (0..N * N)
                .map(|i| C::new(-u[i].re * x[i].re - v[i].re * y[i].re, 0.0))
                .collect::<Vec<_>>()
        };
        let mut rw = self.fft(&adv(&wx, &wy), false);
        let mut rd = self.fft(&adv(&dx, &dy), false);
        self.filter(&mut rw);
        self.filter(&mut rd);
        for y in 0..N {
            for x in 0..N {
                let i = y * N + x;
                let k2 = self.k[x].powi(2) + self.k[y].powi(2);
                rw[i] -= NU * k2 * w[i];
                rd[i] -= NU * k2 * d[i];
            }
        }
        (rw, rd)
    }
    fn step(&self, w: &mut Vec<C>, d: &mut Vec<C>, dt: f64) {
        let (a, b) = self.rhs(w, d);
        let add =
            |v: &[C], r: &[C], t: f64| v.iter().zip(r).map(|(x, y)| x + t * y).collect::<Vec<_>>();
        let (c, e) = self.rhs(&add(w, &a, dt / 2.0), &add(d, &b, dt / 2.0));
        let (f, g) = self.rhs(&add(w, &c, dt / 2.0), &add(d, &e, dt / 2.0));
        let (h, i) = self.rhs(&add(w, &f, dt), &add(d, &g, dt));
        for k in 0..N * N {
            w[k] += dt / 6.0 * (a[k] + 2.0 * c[k] + 2.0 * f[k] + h[k]);
            d[k] += dt / 6.0 * (b[k] + 2.0 * e[k] + 2.0 * g[k] + i[k]);
        }
    }
}
fn figure(data: &[f64], time: f64) -> Result<Figure> {
    let mut f = Figure::new(IntendedUse::ArticleColumn);
    f.height_mm = 90.0;
    let scale = f.scale(Scale::new(
        [0., 1.],
        Normalization::Linear,
        ColorMap::estuary(),
    )?);
    let mut p = Panel::new(
        Rect::new(15., 12., 60., 60.),
        Axes::new(
            [0., std::f64::consts::TAU],
            [0., std::f64::consts::TAU],
            r"$x$",
            r"$y$",
        ),
    );
    let edges: Vec<_> = (0..=N)
        .map(|i| i as f64 / N as f64 * std::f64::consts::TAU)
        .collect();
    let mut field = Field::new(edges.clone(), edges, data.to_vec(), scale)?;
    field.interpolate = true;
    p.push(Layer::Pcolormesh(field));
    f.panels.push(p);
    let mut bar = Colorbar::new(
        Rect::new(15., 79., 60., 1.5),
        scale,
        r"Dye fraction $c$",
        vec![0., 0.5, 1.],
    );
    bar.horizontal = true;
    f.colorbars.push(bar);
    f.height_mm = 98.0;
    f.labels.push(Label::new(
        format!(r"Kelvin--Helmholtz $\quad t={time:.1}$"),
        45.,
        5.,
    ));
    Ok(f)
}
fn main() -> Result<()> {
    if std::env::args().any(|s| s == "--render-only") {
        let bytes = fs::read("output/khi/dye.f32")?;
        let values: Vec<f64> = bytes
            .chunks_exact(4)
            .map(|v| f32::from_le_bytes(v.try_into().unwrap()) as f64)
            .collect();
        for (k, field) in values.chunks_exact(N * N).enumerate() {
            let f = figure(field, k as f64 * 0.3)?;
            f.save(format!("output/khi/frames/{k:04}.png"))?;
            if [0, 20, 40].contains(&k) {
                f.save(format!("gallery/khi_t{}.pdf", k * 30))?;
            }
            if k == 40 {
                f.save("gallery/khi.png")?;
            }
        }
        return Ok(());
    }
    fs::create_dir_all("output/khi/frames")?;
    let s = Solver::new();
    let pi = std::f64::consts::PI;
    let mut dye = vec![C::default(); N * N];
    let mut w = dye.clone();
    for y in 0..N {
        let yy = (y as f64 + 0.5) / N as f64 * 2.0 * pi;
        let a = (yy - pi / 2.0) / 0.22;
        let b = (yy - 3.0 * pi / 2.0) / 0.22;
        for x in 0..N {
            let xx = (x as f64 + 0.5) / N as f64 * 2.0 * pi;
            let i = y * N + x;
            dye[i] = C::new(0.5 * (a.tanh() - b.tanh()), 0.0);
            w[i] = C::new(
                -(1.0 / a.cosh().powi(2) - 1.0 / b.cosh().powi(2)) / 0.22 + 0.04 * xx.cos(),
                0.0,
            );
        }
    }
    let mut w = s.fft(&w, false);
    let mut d = s.fft(&dye, false);
    s.filter(&mut w);
    s.filter(&mut d);
    // Exact single Fourier mode viscous decay is independent of nonlinear terms.
    let mut mode = vec![C::default(); N * N];
    mode[1] = C::new(1.0, 0.0);
    let mut zero = vec![C::default(); N * N];
    for _ in 0..100 {
        s.step(&mut mode, &mut zero, 0.01);
    }
    let decay_error = (mode[1].re - (-NU).exp()).abs();
    ensure!(decay_error < 1e-10, "Fourier diffusion test failed");
    let dt = 0.01;
    let mut diagnostics = vec![];
    let mut raw = vec![];
    for step in 0..=1200 {
        if step % 30 == 0 {
            let field = s.fft(&d, true).iter().map(|v| v.re).collect::<Vec<_>>();
            let omega = s.fft(&w, true);
            ensure!(field.iter().all(|v| v.is_finite()), "Nonfinite dye");
            let energy = w
                .iter()
                .enumerate()
                .filter(|(i, _)| *i > 0)
                .map(|(i, v)| {
                    let k2 = s.k[i % N].powi(2) + s.k[i / N].powi(2);
                    if k2 > 0.0 { v.norm_sqr() / k2 } else { 0.0 }
                })
                .sum::<f64>()
                / (2.0 * (N * N).pow(2) as f64);
            let enstrophy = omega.iter().map(|v| v.re * v.re).sum::<f64>() / (2.0 * (N * N) as f64);
            let mean = field.iter().sum::<f64>() / (N * N) as f64;
            diagnostics.push(serde_json::json!({"time":step as f64*dt,"energy":energy,"enstrophy":enstrophy,"mean_dye":mean,"mean_vorticity":w[0].re/(N*N) as f64}));
            raw.extend(field.iter().flat_map(|v| (*v as f32).to_le_bytes()));
            figure(&field, step as f64 * dt)?
                .save(format!("output/khi/frames/{:04}.png", step / 30))?;
            if [0, 600, 1200].contains(&step) {
                figure(&field, step as f64 * dt)?.save(format!("gallery/khi_t{step}.pdf"))?;
            }
            if step == 1200 {
                figure(&field, 12.0)?.save("gallery/khi.png")?;
            }
        }
        if step < 1200 {
            s.step(&mut w, &mut d, dt);
        }
    }
    fs::write("output/khi/dye.f32", raw)?;
    fs::write(
        "output/khi/diagnostics.json",
        serde_json::to_vec_pretty(
            &serde_json::json!({"shape":[N,N],"nu":NU,"dt":dt,"method":"2/3-dealiased Fourier vorticity Navier-Stokes; RK4; passive scalar diffusivity=nu","fourier_decay_error":decay_error,"samples":diagnostics}),
        )?,
    )?;
    println!("KHI completed: 41 frames; Fourier diffusion error {decay_error:e}");
    Ok(())
}
