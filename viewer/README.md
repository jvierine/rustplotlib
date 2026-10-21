
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

## Indexed 3D surfaces

MeshView(canvas) renders actual indexed triangles in WebGL2 with a perspective
camera, smooth vertex normals and a shared depth buffer. Geometry remains on the
GPU while the camera moves.

- set_mesh(xyz, triangles, values) accepts finite XYZ vertex coordinates,
  unsigned triangle indices and one scalar per vertex. Remove masked triangles
  and unused nonfinite vertices before uploading.
- set_colored_mesh(xyz, triangles, rgb) accepts per-vertex RGB, including
  physical surfaces and markers that must occlude one another correctly.
- set_lines(xyz_rgb) adds pairs of XYZ/RGB vertices in the same world/depth space.
- set_bounds(x0,x1,y0,y1,z0,z1) selects the camera target and normalization.
- set_camera(yaw,pitch,distance) uses radians, z up and a 45 degree field of view.
- set_axis_scale(x,y,z) is explicit graph scaling. Keep (1,1,1) for three
  physical spatial dimensions.
- set_scale(low,high) and set_colormap(rgb) control scalar colour mapping.
- project(x,y,z) returns normalized canvas x/y and camera depth for annotations.
  It uses the same camera equations as the shader.

Render a camera orbit by changing the camera, drawing the WebGL canvas into an
output canvas, and recording it with canvas.captureStream and MediaRecorder.
Fixed labels should occupy a separate, bounded annotation region; account for
projected geometry over the complete orbit when framing it. Projection and
mesh-input validation have native regression tests. Browser depth tests should
submit overlapping triangles in opposite orders and verify the same nearer
surface is visible.
