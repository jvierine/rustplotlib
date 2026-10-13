# Gallery

Generate the additional examples with `cargo run --release --example gallery`.
Native figures are exported at 300 dpi and their intended physical size. HEIMDALL uses measured VTEC; the atmospheric cutaways use analytic fields.

## Field primitives

pcolormesh, contourf, contour and independent color scales.

![Field primitives](fields.png)

[PDF](fields.pdf) · [PNG](fields.png) · [SVG](fields.svg) · [Source](../examples/publication.rs)

## Article column

85 mm wide with 10 pt LaTeX type.

![Article column](single_column.png)

[PDF](single_column.pdf) · [PNG](single_column.png) · [SVG](single_column.svg) · [Source](../examples/publication.rs)

## Keogram with gaps

Shared signed-asinh scale, masked samples and a propagation guide.

![Keogram with gaps](keogram.png)

[PDF](keogram.pdf) · [PNG](keogram.png) · [SVG](keogram.svg) · [Source](../examples/gallery.rs)

## Curvilinear mesh

Physical quadrilateral corners, with equal x/y scale.

![Curvilinear mesh](curvilinear.png)

[PDF](curvilinear.pdf) · [PNG](curvilinear.png) · [SVG](curvilinear.svg) · [Source](../examples/gallery.rs)

## Mathematical annotations

Acoustic and gravity branches of an analytic dispersion relation.

![Mathematical annotations](dispersion.png)

[PDF](dispersion.pdf) · [PNG](dispersion.png) · [SVG](dispersion.svg) · [Source](../examples/gallery.rs)

## Color normalization

Linear, signed-asinh and logarithmic magnitude views.

![Color normalization](normalizations.png)

[PDF](normalizations.pdf) · [PNG](normalizations.png) · [SVG](normalizations.svg) · [Source](../examples/gallery.rs)

## Mobile preset

13 pt type at mobile-page width.

![Mobile preset](mobile.png)

[PDF](mobile.pdf) · [PNG](mobile.png) · [SVG](mobile.svg) · [Source](../examples/gallery.rs)

## Desktop half-window

12 pt type for a wider on-screen figure.

![Desktop half-window](desktop.png)

[PDF](desktop.pdf) · [PNG](desktop.png) · [SVG](desktop.svg) · [Source](../examples/gallery.rs)

## Small multiples and diagnostics

![Small multiples and diagnostics](khi_summary.png)

KHI evolution with directly labelled energy and enstrophy. [Source](../examples/khi_summary.rs)

## Functions of two variables

![Functions of two variables](surfaces.png)

Depth-tested, shaded 3-D surfaces. [Source](../examples/surfaces.rs)

## Dimant--Oppenheim density

![Dimant--Oppenheim density](dimant_oppenheim.png)

Logarithmic density cross-section and directly labelled profiles. [Source](../examples/dimant_oppenheim.rs)

## Meteor density surface

![Meteor density surface](dimant_surface.png)

Log-height surface of the same density function. [Source](../examples/dimant_oppenheim.rs)

## Kelvin--Helmholtz instability

![Kelvin--Helmholtz instability](khi.gif)

Periodic viscous Navier--Stokes with passive dye. [Source](../examples/khi.rs)

## HEIMDALL cartography

![HEIMDALL cartography](heimdall.png)

Measured VTEC; shared HEIMDALL WebGL2 renderer, globe and flat-map views. [Source](../web/src/heimdall.ts)

## Spherical cutaways

![Spherical cutaways](atmosphere.png)

Interactive temperature, density and wind demonstration fields. [Source](../examples/atmosphere.rs)

## Volume renderer

![Volume](volume.gif)

[Source](../examples/web_data.rs)
