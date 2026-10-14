# One-panel TIE-GCM animation

Retains the static `tiegcm` example and adds a separate animation.

```sh
conda run -n base python scripts/prepare_tiegcm_gallery.py
cargo run --release --example tiegcm_animation
ffmpeg -framerate 4 -i output/tiegcm-animation/%04d.png -vf 'pad=ceil(iw/2)*2:ceil(ih/2)*2:color=white' -c:v libx264 -crf 18 -pix_fmt yuv420p -movflags +faststart gallery/tiegcm_animation.mp4
```

Uses all 49 paired model outputs at five-minute cadence (0–240 min).
No temporal interpolation. Spatial interpolation matches the static example.
The fixed ±12.5 K scale, spherical geometry, missing-height coverage and
latitude ticks remain unchanged. HDF5 scientific source: `gallery/tiegcm/section.h5`.
Display metadata is recorded in `output/tiegcm-animation/provenance.json`.
