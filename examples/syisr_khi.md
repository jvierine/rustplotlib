# HDF5 KHI scientific figures

`cargo run --release --features hdf5-examples --example syisr_khi -- INPUT.h5 OUTPUT_DIR`

Defaults: `examples/support/syisr_khi.h5`, `output/syisr-khi`.
The fixture contains three simulated Ri=0.08 snapshots (0,60,120 minutes),
not raw SYISR observations. The scientific solver/analysis remains separate.
The gallery MP4s contain all721 original10-second simulation checkpoints.

HDF5 schema:

- `x_edges_km`, `z_edges_km`: field cell edges in the mean-flow/descending-phase frame.
- `z_center_km`, `background_wind_m_s`: aligned zonal-wind profile.
- `frames/NNNN/electron_density_m3`, `ion_up_m_s`: `[nz,nx]` floating arrays.
- Frame attribute `time_s`; root attribute `ion_velocity_max_abs_m_s` sets a fixed signed colour scale.

Ion velocity is the upward collisional Lorentz-neutral drag solution in the
model frame. It is not neutral vertical wind. Prescribed descent of the phase
frame is not added as a physical ion velocity. Simulation density is shown
between1e10 and2e12 m^-3; larger values saturate. Full animation input uses the
same schema with721 frames. HDF5 provenance records the scenario and coordinate
frame. Fixed178mm page width and10pt LaTeX labels; animation pages deliberately
retain a fixed crop across frames. PNG/PDF exports are produced for each input.

Encode a complete series with:

```sh
ffmpeg -framerate 30 -i OUTPUT_DIR/ne/%04d.png -vf 'pad=ceil(iw/2)*2:ceil(ih/2)*2' -c:v libx264 -pix_fmt yuv420p -movflags +faststart density.mp4
ffmpeg -framerate 30 -i OUTPUT_DIR/ion_up/%04d.png -vf 'pad=ceil(iw/2)*2:ceil(ih/2)*2' -c:v libx264 -pix_fmt yuv420p -movflags +faststart ion_up.mp4
```

On this Mac's Conda HDF5 runtime, use `HDF5_DIR=/opt/anaconda3` to build and
`DYLD_FALLBACK_LIBRARY_PATH=/opt/anaconda3/lib` to run. Other machines should
use their installed HDF5 development package; no Conda-specific path is baked
into the example.
