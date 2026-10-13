# Reproduce the gallery

```sh
cargo run --release --example publication
cargo run --release --example gallery
cargo run --release --example surfaces
cargo run --release --example dimant_oppenheim
cargo run --release --example khi
cargo run --release --example khi_summary
npm run build
npm run serve
```

Open `/gallery/index.html`. The browser examples are `/heimdall.html` and
`/atmosphere.html`; the WebGL fields and volume viewer are at `/`.

KHI solves periodic incompressible Navier–Stokes in vorticity form on a 128²
Fourier grid, with 2/3 dealiasing, RK4, viscosity and dye diffusivity 0.001.
The double shear layer has width 0.22, with a 0.04 sinusoidal transverse
velocity perturbation. The run saves 41 frames through t=12. Bilinear display
interpolation leaves the numerical arrays unchanged. The exact Fourier decay
check, mean dye, energy and enstrophy are archived in `gallery/khi.h5`.
This is a numerical visualization example, not a resolution-converged benchmark.

Dimant–Oppenheim uses a Rust port of `jvierine/sabmod` at
`bb2388d4f45dba6aaa0e944f29bdf686e54a30d4`. Coordinates, the 8π display factor,
logarithmic limits and color table match that example. A transformed
Gauss–Legendre integral removes the endpoint singularity. Tests compare 64/128
nodes, the axis limit and cylindrical symmetry; reference points were checked
against the supplied SciPy implementation. `gallery/dimant_oppenheim.h5` records
normalization, palette extraction and numerical checks.

The HEIMDALL example reuses its WebGL renderer with nine frozen measured VTEC
frames (12:00–14:00 UTC, including the selected 12:59 frame). Source URLs and
hashes are archived with the data. Neighbour fill remains optional.

Spherical cutaways currently use analytic fields. The TIE-GCM converter is
`scripts/prepare_tiegcm.py`; it requires a secondary history containing
geometric heights. No TIE-GCM output is presented as part of the synthetic demo.
