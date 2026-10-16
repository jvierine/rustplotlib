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
and flat values. Curvilinear cells share one raster coverage buffer, preventing antialias seams. Dense
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

`Field::interpolate = true` enables bilinear interpolation at physical cell centres.
The default is flat cells. Missing cells remain missing; interpolation does not bridge them.
`ColorMap::listed` preserves a discrete color table exactly.

`Surface` renders `z=f(x,y)` with interpolated colors, shading and a depth buffer.
`Projection` provides orthographic and equirectangular coordinates, inverses,
graticules, horizon clipping and antimeridian path breaks. The HEIMDALL example
reuses its deployed WebGL2 renderer and frozen measured VTEC frames.

TIE-GCM secondary histories can be converted with
`conda run -n base python scripts/prepare_tiegcm.py history.nc --output model.h5`.
It uses ZG for interface fields and ZGMID (or ZG interpolated in log pressure)
for midpoint fields, with explicit unit conversion and no height extrapolation.
[NCAR grid conventions](https://www.hao.ucar.edu/modeling/tgcm/tiegcm2.0/userguide/html/grid.html).

`Layer::Image(RgbaImage)` fits a raster data view into a panel while keeping axes,
labels and legends in native LaTeX/vector output. Scientific gallery exports use
this for WebGL scenes; page controls are excluded.

`cartography::curved_section` maps shared distance/altitude cell edges onto a
spherical meridian. Coordinates and radius are in kilometres, with no vertical
exaggeration. See `examples/tiegcm.rs` for paired model data at common geometric
heights.

Label/colorbar and axis/panel collisions produce a warning and fail export by
default. `Figure::check_label_geometry=false` permits intentional overlaps;
label bounds and label-to-label checks remain independent.

`PolygonMesh::new(polygons, values, scale)` accepts independent finite-vertex
cells, such as projected HEALPix footprints. A shared scanline raster avoids
seams; missing scalar values remain transparent. Coordinates use the panel axes.

`Axes::xticklabels` and `yticklabels` optionally provide LaTeX strings, one per
numeric tick. These label calendar times without changing the plotted coordinates.
`Connector` links data positions in two panels through optional `via` waypoints
in figure millimetres from the top left. Add it to `Figure::connectors`.

## Legends

`Panel::legend` accepts a compact, frameless `Legend` with LaTeX labels and
vector line or filled-circle samples. The same placement is used in PDF,
SVG and raster exports.

```rust
let mut legend = Legend::new(vec![
    LegendEntry::line("Expected trajectory", "#003c9e"),
    LegendEntry::circle("Radar terminus", "#007a52"),
]);
legend.position = LegendPosition::UpperLeft;
legend.avoid_layers = vec![1, 3]; // protect observations and model, not clouds
panel.legend = Some(legend);
```

`avoid_data` defaults to true. `position` specifies the preferred corner; if
occupied, other corners and then interior positions are searched. An empty
`avoid_layers` list protects all non-image layers. Set explicit layer indices
to allow contextual images, coastlines or other background layers to be covered.
Protection uses finite, visible data coverage (including polygon footprints
and line widths), not just data-point centres, at a quarter-millimetre occupancy
resolution with an additional half-millimetre clearance. Missing/masked cells
remain available for placement. Export fails if the legend cannot fit without
covering protected data. `avoid_data = false` explicitly fixes the requested
corner. `padding_mm` and `background_opacity` control the restrained white backing.

`ColorMap::seismic()` and `inferno()` provide the standard 256-entry tables.
Listed colour bars use lookup-table image strips rather than large PDF stitching
functions, preserving their discrete colours and avoiding PDF-reader limits.

## Article Typography Contract

Prefer `ArticleLayout::new(linewidth_mm, body_font_pt)?.figure(width_fraction,
height_mm)?` for publication plots. Use the linewidth of the actual figure
environment, not the paper width: a column figure and a two-column figure have
different available widths. Obtain it from LaTeX with `\the\linewidth`; convert
TeX points to mm using `25.4 / 72.27`. Font sizes are TeX points, not PDF points.

The resulting figure uses the final physical width and body-sized text. Width
fractions change available plotting area, not typography. `grid` can determine
height, while widths and fonts remain bound. Exports reject mutations that break
the contract, including tight cropping. Lay out smaller figures anew; do not
scale all coordinates and text from a larger figure.

`save_for_article("map.pdf", "figures/map.pdf")` exports a PDF and a
`map.include.tex` companion. `latex_include("figures/map.pdf")` returns the same
snippet. It includes at the configured physical width and checks the article's
normal font and current linewidth at LaTeX compile time (tolerances 0.01 pt and
0.1 mm). It requires graphicx. Do not wrap the inclusion in an external scaling
transform: rustplotlib cannot police arbitrary downstream LaTeX transforms.

The article contract checks the global label size. Explicit size commands inside
user-supplied LaTeX labels, raster image text, and externally scaled containers
remain the author's responsibility. Old `Figure::new(IntendedUse::...)` APIs
remain unbound for compatibility; they do not imply a checked article layout.

## Web Typography Contract

`WebLayout::new(container_css_px, body_font_css_px)?.figure(height_css_px)?`
specifies dimensions in CSS pixels, independent of device pixel ratio. It
converts the CSS font into TeX points for the label renderer (72.27 pt per inch,
96 CSS px per inch). Font and width overrides and tight cropping are rejected.

`save_for_web("plot.svg", "assets/plot.svg", "Accessible plot description")`
exports SVG plus `plot.embed.html`. `html_embed(url, alt)` returns the embed.
The image width is relative to the inherited CSS text size in em: increasing
page text size increases plot text too. The keyboard-scrollable wrapper handles
narrow containers without reducing labels. Do not override the generated image
width or apply CSS transforms, `zoom`, or `max-width:100%` to it. The wrapper
may have max-width:100%; the plot image deliberately does not.

For responsive reflow, render a variant at each target container width with the
same body font size and recompute its panels using `grid` or your layout. Select
variants using container/media queries. This API does not automatically reflow
an already-rendered SVG. The `web_typography` example demonstrates two layouts.
Explicit label font commands and text baked into images remain outside the
contract, as with article plots.

## Physical-Size Markers

For line ensembles, `Line::opacity` (default 1) controls per-curve transparency.
Use thin strokes and low opacity to keep contextual trajectories subordinate
to the primary measurement distribution; overlapping curves accumulate density.
Opacity must lie in 0--1. Invisible curves do not block legend placement.

`Legend::columns` (default 1) creates a compact key with entries filled across
each row. Columns are sized to their own labels, with consistent symbol spacing.
Data avoidance uses the complete multi-column footprint.

`panel.rasterize_lines(start..end, dpi)?` replaces a dense contiguous line
ensemble by one transparent raster layer, preserving curve overlap and clipping.
Other layers remain vector. Use this after panel sizing for publication PDFs
that would otherwise contain thousands of vector paths. Indices after the
replaced group shift; set explicit legend protection indices afterwards.

`Panel::markers(&mut figure, &points, &MarkerStyle)` adds vector symbols at
data coordinates. `MarkerStyle` specifies `shape`, `size_pt` (full diameter or
cap length), and an RGB `color` in 0--1. Supported shapes are `Circle`, `Square`,
`Diamond`, `Triangle`, `HorizontalCap`, and `VerticalCap`. Sizes stay fixed in
physical points across unequal axis ranges and panel aspect ratios. Nonfinite
positions are skipped; invalid sizes and colors return errors. Error-bar stems
can use `Line` with the cap shapes at their endpoints. Symbols are ordinary
layers and participate in legend data avoidance.
