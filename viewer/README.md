
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
