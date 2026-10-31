# Interactive fields: complete plots by default

Use `InteractiveFieldPlot`, not `FieldView`, for a scientific web plot.
`FieldView` is a low-level GPU texture renderer and deliberately does not own a
figure. It is not the default public figure API.

The complete API owns axes, automatic numeric/UTC ticks, axis labels, colorbars,
panel labels, a shared-axis stack, and navigation. All decorations are generated
by `rustplotlib::interactive::field_frame`, using the native Figure/Axes/Colorbar
objects and LaTeX typography. A page must not construct an independent HTML axis,
CSS gradient legend, or hand-written tick renderer around this component.

```ts
const plot = await InteractiveFieldPlot.mount(element, {
  id: 'echoes',
  spec: registeredNativeSpecification,
  colorLimits: [3, 15],
});
plot.panels[0].set_rectilinear_data(xEdges, yEdges, values);
```

Labels are mandatory. A frame endpoint defaults to `./api/frame` and accepts
only the registered plot ID, CSS width, normalized viewport and color limits.
Its trusted server-side specification supplies scientific labels, panel names,
physical bounds and optional UTC origin. Never accept arbitrary LaTeX or local
file paths from a browser request.

The default controls are wheel zoom at the pointer, drag-box zoom, Shift-drag
pan, double-click reset, Home/Escape reset, and a reset button. PlotViewport is
the single state object used for both data clipping and native decoration
generation. Every panel of a stack shares that viewport; only its bottom panel
has x-axis decorations. Changing a color limit updates the field and its native
colorbar together. Resize recomputes layout at a 16 CSS px font size, independently
of device pixel ratio.

While a changed viewport is being typeset, old axis decorations are hidden so
they cannot incorrectly label the new data view. Stale responses are discarded.
The host supplies arrays and scientific metadata, not tick positions or event
handlers. A missing native frame service fails visibly rather than silently
falling back to custom labels or an undecorated texture.

## Static-host series navigation

`InteractiveSeriesPlot` uses the shared `PlotControlGroup` and
`bindPlotNavigation` controller. Its default is **Scroll**: ordinary wheel and
single-finger touch gestures scroll the page, without capturing the pointer or
changing the viewport. Select **Zoom** for box zoom / pointer-centered wheel
zoom, or **Pan** for drag panning (Shift-drag also pans in Zoom mode).
Visible + / − buttons zoom about the center. **Reset view**, double-click,
Home or Escape restores the full viewport and returns to Scroll mode.
Cancellation clears a selection and releases pointer capture.

Several series plots can share one toolbar: create a `PlotControlGroup`, append
its `element`, and pass it as the `controls` mount option. Modes and toolbar
actions apply to all registered plots; drag navigation affects the touched
plot. Controls remain visible even with the legacy `compact` option. Dispose
unregisters a plot. `setHeight` resizes while preserving the data viewport.

Dense small multiples can request native `compact_axes`, `panel_label` and
`hide_x_labels` (only on non-bottom panels). Rust still owns axes, ticks, units
and clipping. The compact renderer supports 150×90 CSS px panels; the labels
remain in the specification for accessibility. Do not add HTML axes.

## Three-variable HSV keys

Use native `HsvColorKey` for moment images rather than a scalar palette bar.
It owns velocity hue, width desaturation and power value limits and generates
two linked native color-key panels: velocity/power at zero width and
velocity/width at maximum power. `rgb()` matches the moment shader exactly.
`HsvColorKey::add_to` is also available for ordinary saved Rust figures.
Registered interactive specifications can supply `hsv_key`; the complete plot
then uses a compact equal-aspect data panel beside this multivariable legend.
Update numerical frame parameters when hue/width limits change, so the native
key changes with the GPU image. Scientific labels and units are still native.

Click any native scalar colorbar or either HSV key slice to open the default
color-limit editor. Minimum/maximum sliders and numeric inputs update the GPU
immediately and request matching native decorations. Moment fields also get
velocity-hue and spectral-width controls. No per-page popup implementation or
click-position guesses are needed: hit regions come from the native figure.
`getColorLimits`, `getHsvLimits`, `setColorLimits` and `setHsvLimits` are public.
Moment-data calls automatically select the HSV key; scalar data selects the
scalar bar. Preserve per-plot styles across data refreshes instead of resetting
limits in the host on every redraw.
