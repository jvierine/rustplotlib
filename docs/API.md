# API notes

`Figure` is a collection of independently positioned panels, labels and
colorbars. `grid` is an optional layout helper. It chooses golden-ratio data
rectangles and reserves margins based on the intended font size. There is no
mandatory dashboard, plot taxonomy or global colorbar.

All figure geometry is in millimetres; all type and line widths are in points.
Article-column width defaults to 85 mm with 10 pt type; article-wide to 178 mm
with 10 pt type. Mobile and desktop-half presets use 13 and 12 pt respectively.
Change the public properties or supply `IntendedUse::Custom` when a journal or
page needs different dimensions. Export at the intended inclusion width.

Exports reject cropped and overlapping label bounding boxes by default.
`check_label_bounds` and `check_label_overlap` can be disabled for intentional
overlays. These checks complement visual inspection; they cannot detect whether
an annotation obscures an important feature of the data.

## Layers and colorbars

`Field::new(x_edges, y_edges, values, scale_id)` requires strictly increasing
rectilinear cell edges. Values are indexed `j * nx + i`, with x varying fastest
and y increasing upwards. A `QuadMesh` has explicit physical corner coordinates
and flat values. Curvilinear SVG/PDF cells are vector polygons; dense
rectilinear fields are rasterized to the figure's `raster_dpi`, default 300.

`Contour::new(x, y, values, levels, scale_id, filled)` takes **nodal** values,
not cell edges. Contours use a consistent lower-left/upper-right triangular
split and linear interpolation. This is a defined piecewise-linear interpolant,
not a smoothed spline. Filled contours rasterize that same interpolant into
explicit level bands; contour lines remain vectors. A missing cell corner masks
that cell. Values outside filled-contour levels are transparent.

A colorbar refers to a scale ID, not a panel. Reuse a scale ID for shared limits
and colors, or create independent scales. Colorbar ticks are physical data
values; their positions use the same normalization as the field. Logarithmic
scales require positive limits and mask nonpositive data. Asinh scales require
an explicit physical width. Values outside scale limits clip to end colors.

## LaTeX

Labels accept LaTeX directly: `$\omega=\nabla\times\boldsymbol{u}$`.
`latex::text()` escapes ordinary text. Set `figure.latex.preamble` to use a
different font family or packages. Defaults are Palatino text and mathematics
via `mathpazo`, with AMS mathematics. All uncached labels are compiled together;
glyph outlines are cached by label, preamble and point size. Delete the cache
after upgrading TeX or fonts. No browser math approximation is used.

Inputs are trusted LaTeX and compilation always disables shell escape. PDF/SVG
use outlined glyphs: consistent appearance without relying on installed viewer
fonts, but label text is not selectable. PNG/JPEG use the same SVG scene.

## Web embedding

The `rustplotlib-web` crate exposes `FieldView` and `VolumeView` through
`wasm-bindgen`. It has no dependency on a particular page framework. Create a
view from a canvas, upload float32 data, and connect your own controls. The
included TypeScript only loads data and wires controls; rendering is in Rust
and WebGL2 shaders.

`FieldView.set_data(nx, ny, values)` accepts a regular texture grid.
`set_rectilinear_data(x_edges, y_edges, values)` preserves nonuniform cell
widths in physical coordinates through GPU edge lookup. Curvilinear polygons
are currently available in native exports, not this texture viewer.
`set_scale` changes color normalization; `set_view` selects normalized data
coordinates without reallocating the texture. Any coordinate axes supplied by
the host must follow that view. The demo keeps its physical extent fixed and
provides frame playback and a coordinate/value probe.

Both GPU viewers accept `set_colormap(rgb)` with 2–32 RGB stops in [0,1]. The
field scale setter selects a built-in palette; apply your custom map afterwards.

`VolumeView.set_data(nx, ny, nz, values)` uploads an R32F 3-D texture. X varies
fastest, then y, then z. NaN volume values are rejected. The current viewer
defaults to a unit cube; `set_box_extent` preserves physical box proportions. `set_camera` changes
yaw/pitch; `set_transfer` controls value limits, optical density and cutoff.
Trilinear interpolation is explicit, so no float-filtering extension is needed.
The volume renderer is scalar direct-volume rendering, not a fluid solver.

Browser compilation requires no LaTeX. Generate or cache label SVGs on a machine
with TeX, then serve them with the WASM package. See `web/src/app.ts` for the
minimal host and `scripts/build_web.sh` for packaging.
