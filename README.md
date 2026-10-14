# rustplotlib

[Live gallery](https://jvierine.github.io/rustplotlib/gallery/index.html) · [Examples](gallery/README.md)

Rustplotlib is a plotting library inspired by Matplotlib, implemented in Rust.
Its purpose is clean scientific plotting with LaTeX math, for publication-quality
figures and interactive Rust/WASM + WebGL web pages.

The design priorities are:

- **Scientific publications:** readable text at the intended paper size, restrained layouts, aligned panels, clear colour bars, and tight exports.
- **LaTeX mathematics:** labels, ticks, symbols, and equations use genuine LaTeX; PDF preserves vector text and curves.
- **Interactive web plots:** Rust/WASM + WebGL views with camera controls, changing data, and animation playback.
- **Animation and extensibility:** render evolving simulations, export movies, and add plotting features without sacrificing scientific clarity.
- **Vibecodability:** a small, familiar API that makes figures easy to create, inspect, and revise from code, including AI-assisted coding.

Native Rust renders publication exports. Rust/WASM + WebGL supports interactive
browser views. LaTeX typesets the notation. The examples demonstrate both output
paths; feature coverage differs between the native and browser APIs.

```rust
use rustplotlib::*;

let mut figure = Figure::new(IntendedUse::ArticleColumn);
let panels = figure.grid(1, 1)?;
figure.panels[panels[0]].axes = Some(Axes::new(
    [0., 10.], [-1., 1.], r"$t\,[\mathrm{s}]$", r"$p'/p_0$",
));
figure.panels[panels[0]].push(Layer::Line(Line::new(points, "#245774")));
figure.save("figure.pdf")?;
```

## Scientific examples

Native scientific exports, interactive WebGL pages, and simulation animations.
Click a figure to open its PDF, animation, or interactive view.

| Curvilinear mesh | Acoustic–gravity dispersion |
|:--:|:--:|
| [![Curvilinear mesh](gallery/curvilinear.png)](gallery/curvilinear.pdf) | [![Acoustic–gravity dispersion](gallery/dispersion.png)](gallery/dispersion.pdf) |
| [PDF](gallery/curvilinear.pdf) · [Source](examples/gallery.rs) | [PDF](gallery/dispersion.pdf) · [Source](examples/gallery.rs) |

| KHI evolution and diagnostics | Dimant–Oppenheim density surface |
|:--:|:--:|
| [![KHI evolution and diagnostics](gallery/khi_summary.png)](gallery/khi_summary.pdf) | [![Dimant–Oppenheim density surface](gallery/dimant_surface.png)](gallery/dimant_surface.pdf) |
| [PDF](gallery/khi_summary.pdf) · [Source](examples/khi_summary.rs) | [PDF](gallery/dimant_surface.pdf) · [Source](examples/dimant_oppenheim.rs) |

| Kelvin–Helmholtz instability | Measured HEIMDALL TEC |
|:--:|:--:|
| [![Kelvin–Helmholtz instability](gallery/khi.png)](gallery/khi.gif) | [![Measured HEIMDALL TEC](gallery/heimdall.png)](https://jvierine.github.io/rustplotlib/heimdall.html) |
| [PDF](gallery/khi_t0.pdf) · [Source](examples/khi.rs) | [PDF](gallery/heimdall.pdf) · [Source](web/src/heimdall.ts) |

| GAMERA current paths | SMILE UVI on Earth |
|:--:|:--:|
| [![GAMERA current paths](gallery/currents.png)](https://jvierine.github.io/rustplotlib/currents.html) | [![SMILE UVI on Earth](gallery/smile.png)](https://jvierine.github.io/rustplotlib/smile.html) |
| [PDF](gallery/currents.pdf) · [Source](examples/scientific_views.rs) | [PDF](gallery/smile.pdf) · [Source](examples/scientific_views.rs) |

| TIE-GCM heating pulse | Damped oscillations at article-column size |
|:--:|:--:|
| [![TIE-GCM heating pulse](gallery/tiegcm.png)](gallery/tiegcm.pdf) | [![Damped oscillations at article-column size](gallery/single_column.png)](gallery/single_column.pdf) |
| [PDF](gallery/tiegcm.pdf) · [Source](examples/tiegcm.rs) | [PDF](gallery/single_column.pdf) · [Source](examples/publication.rs) |

| KHI electron-density animation | Ion drift and background wind |
|:--:|:--:|
| [![KHI electron-density animation](gallery/syisr_khi_density.gif)](gallery/syisr_khi_density.mp4) | [![Ion drift and background wind](gallery/syisr_khi_ion_up.gif)](gallery/syisr_khi_ion_up.mp4) |
| [PDF](gallery/syisr_khi_density.pdf) · [Source](examples/syisr_khi.rs) | [PDF](gallery/syisr_khi_ion_up.pdf) · [Source](examples/syisr_khi.rs) |

The KHI cases are simulated demonstrations, not observational fits.
[Browse all examples](gallery/README.md).

`ArticleColumn`, `ArticleWide`, `Mobile`, `DesktopHalf`, or custom dimensions.
Defaults use 10–13 pt type, consistent margins and golden-ratio panel geometry.
Panels can also be positioned freely in millimetres. Axes are optional.

- Rectilinear and curvilinear `pcolormesh`; `contour` and `contourf`.
- Shared or independent colorbars; linear, log and signed-asinh scales.
- PDF, PNG, JPEG and SVG. Missing values remain missing.
- Rust/WASM + WebGL2 field and volume renderers, playback and orbit controls.

## Run

Native exports require TeX Live/MacTeX (`latex`, `dvisvgm`, `mathpazo`).

```sh
cargo run --release --example publication
cargo test
```

Browser demo:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
npm install
npm run build
npm run serve
```

Open `http://127.0.0.1:8791`. The browser uses cached LaTeX SVG glyphs;
it does not need a TeX installation. Example fields are generated by
`examples/web_data.rs`; the vortex rings are analytic demonstration data.

See [the API notes](docs/API.md) for coordinates, normalization and embedding.

[Live gallery](https://jvierine.github.io/rustplotlib/gallery/index.html) · [Examples](gallery/README.md): KHI, HEIMDALL maps, spherical cutaways,
3-D functions and Dimant–Oppenheim density.

Scientific exports crop to visible content by default with 2 mm padding (`Figure::tight_export`, `Figure::export_padding_mm`). Set `tight_export=false` for a fixed page. Surface geometry fills its panel by default (`Surface::fit_to_view`); disable it to retain the unit-cube camera framing. HEIMDALL Scientific PNG exports the WebGL figure and labelled colour bar without page UI.

### HDF5 simulation rendering

The optional KHI example reads HDF5 fields and renders them with Rustplotlib;
the fluid solver is separate. The included compact fixture contains actual
model checkpoints at 0, 60 and 120 minutes. Complete MP4 examples retain all
721 checkpoints. No replacement background or packet mask is applied.

```sh
# Install an HDF5 development library (e.g. libhdf5-dev on Ubuntu).
cargo run --release --features hdf5-examples --example syisr_khi
```

[Input schema and animation instructions](examples/syisr_khi.md).

Secondary horizontal axes use `Axes::secondary_x` with
`SecondaryAxis::new(label, ticks, to_primary)`. The inverse coordinate mapping
positions ticks on the primary scale, including nonlinear reciprocal mappings.
See [the secondary-axis example](examples/secondary_axis.rs).
