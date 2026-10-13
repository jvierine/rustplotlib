# HEIMDALL renderer

These modules are reused from Juha-Pekka Vierinen's HEIMDALL viewer:
`map-gpu.mjs`, `map-f16.mjs`, `frames.mjs`, `solar.mjs`,
`neighbour-fill.mjs`, and the colormap modules.

The TypeScript example is `web/src/heimdall.ts`. Data are frozen public
HEIMDALL VTEC frames for 25 September 2025; source URLs and hashes are in
`gallery/heimdall/provenance.json`. Natural Earth 110m country geometry
provides the coastlines and borders (public domain).

Crameri palette tables retain their upstream license in
`colormaps-LICENSE.txt`.
