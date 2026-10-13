# Validation

`cargo test` checks contour interpolation, nonuniform cells, missing-data masks,
normalization/inverse ticks, intended-use defaults, LaTeX caching, all four export
formats, physical PDF width, PDF/raster colorbar agreement, and rejection of
cropped or overlapping labels.

`npm run test:web` checks nonuniform GPU cell widths against known colors (within
one 8-bit channel level), missing-data masking, frame changes, asinh scaling,
volume orbit controls, playback, PNG download, and desktop/mobile rendering.

The first four-panel figure took 1.06 s including uncached LaTeX typesetting;
cached-label PNG export took about 70–115 ms on the development Apple Silicon laptop.
Later cached PDF exports were about 60–80 ms. These are small-example timings,
not a general speed comparison with other plotting libraries. Browser tests use
SwiftShader for reproducibility; their command-submission timing is not GPU
completion latency.

The exact PDF was rasterized with `pdftoppm` and inspected, as were the native
PNG and desktop/mobile browser screenshots. Figure generation:
`examples/publication.rs`; browser data: `examples/web_data.rs`; volume GIF:
`scripts/capture_volume.mjs`. Examples use synthetic data.

Contour interpolation is piecewise linear on a consistent triangular split.
General curvilinear GPU plots, automatic contour labels, and selectable PDF text
are not implemented in this version. Native curvilinear plots are supported.
