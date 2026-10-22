
### Reusable plot navigation

`PlotViewport` is a Rust/Wasm normalized-axis controller usable with `FieldView`
and `SeriesView`. `zoom(x, y, factor)` preserves the cursor anchor, `pan(dx, dy)`
moves by fractions of the visible span, and `reset()` restores `[0,0,1,1]`.
`bounds()` returns `[xmin,ymin,xmax,ymax]`; pass these to `FieldView.set_view`
and transform them through physical axis limits for labels or curve axes.
Connect Ctrl/Meta + wheel events to zoom (leave ordinary wheel scrolling to the page), pointer drags to pan, and `contextmenu` to reset
with `preventDefault()`. Share one controller to link comparison panels.
The minimum visible span is 0.001 and navigation stays inside the full domain.

`zoom_box(x0, y0, x1, y1)` selects a rectangle relative to the current view,
accepting either drag direction. Empty/non-finite selections are ignored.

Use the shared `viewer/js/navigation.js` gesture bridge rather than duplicating
handlers in each application:

```js
import {bindPlotNavigation} from './navigation.js';
const stop = bindPlotNavigation(canvas, viewport, () => {
  field.set_view(...viewport.bounds());
  redrawAxisLabels();
});
```

Cmd/Ctrl + drag shows a selection rectangle and zooms on release. Escape cancels.
Plain drag pans; Cmd/Ctrl + wheel zooms; right-click resets. Plain wheel scrolling
is never intercepted. An optional fourth callback supplies the axes rectangle
for canvases containing margins, such as `SeriesView`.


`FieldView.set_reduction("max")` and `HeatmapView.set_reduction("max")` preserve
isolated observations and narrow peaks when many source cells fit into a screen
pixel. Every intersecting source cell contributes to a finite maximum; a fully
missing footprint stays white. Both regular and rectilinear rasters are supported.
The default remains `"nearest"`. Reduction affects display only, not source data,
coordinates, normalization, or the colorbar. Choose maximum reduction for sparse
RTIs or peak-sensitive spectra, rather than interpolating gaps or changing the
underlying time bins.

`HeatmapView.set_palette("plasma")`, `set_markers(true)`, and
`add_band(low, high, label)` provide a shared scientific palette, visible sparse
marks, and labeled physical-coordinate regions. The same palette drives the
field and its colorbar; point marks and bands follow zoom and pan.

`HeatmapView.update_rows(firstRow, values)` updates complete texture rows without
replacing a viewport. `set_rectilinear_data(xCenters, yCenters, values)` preserves
irregular physical sampling. Free offscreen heatmaps to release their owned
WebGL contexts.

`bindColorbarEditor(axesCanvas, heatmap, {automaticLimits, onChange})` opens the
shared numeric limit editor when the colorbar is clicked. It accepts the typed
array returned by WASM `scale_limits()`.

`SeriesView.set_time_axis()` adds UTC ticks before drawing daily line data.
`SeriesView.begin_map(width,height,dpr,west,east,south,north)` fits a Mercator map
with equal projected x/y scale; subsequent series and labels use geographic
longitude/latitude degrees. Labels choose nearby free space to reduce overlap.
The projection is limited to 85 degrees latitude.
