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
