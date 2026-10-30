# Browser diagnostic figures

`InteractiveSeriesPlot` owns the axes, ticks, legends and default interaction.
The native `SeriesView.render_plot` accepts these optional figure fields:

- `bars: true`, `categories: string[]`: categorical vertical bars. Each series
  supplies its own fill color, so thresholds and health meanings stay with the
  application rather than being silently imposed by the plotting library.
- `sky: {lonlat, nside, central_longitude, west_positive}`: Mollweide equal-area
  sky density, with native HEALPix RING binning, graticule, degree labels and a
  counts-per-cell colorbar. Coordinates remain longitude/latitude in degrees.
  Defaults are NSIDE 32, central longitude 270 degrees, west-positive. Set the
  center/orientation explicitly for a different physical coordinate convention.
- `field: {x,y,values,limits,palette,label,missing_value?}`: rectilinear scalar
  field; x/y are strictly increasing cell edges and values are row-major. Nulls
  remain missing; an optional display-only missing color value does not alter
  those values. Colorbar labels and ticks are rendered natively.
- Per series, `errors` contains symmetric vertical half-lengths in y-axis units;
  `line_width` sets CSS-pixel stroke width. Error bars are not automatically
  interpreted as standard errors or confidence intervals.
- `equal_aspect: true` preserves metric distances in position plots; a series
  `fill` color closes/fills a polygon, for footprints or uncertainty regions.

Use `InteractiveSeriesPlot.mount(root,spec,{height:220,compact:true})` to omit
the repeated toolbar inside a compact dashboard. Zoom, pan, keyboard reset and
double-click reset remain enabled. The default still displays the toolbar.

The browser computes and renders these display operations in Rust/WASM. No
server image rendering, radar signal processing, or page-specific axis labels
are needed. A map's density color is counts per equal-area HEALPix cell, not
flux or a selection-corrected radiant distribution.
