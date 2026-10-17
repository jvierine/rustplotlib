# Turbo Colormap Polynomial

Copyright 2019 Google LLC. Licensed under Apache License 2.0; a copy is in
`licenses/Apache-2.0-Turbo.txt`.

Colormap design: Anton Mikhailov. GLSL approximation: Ruofei Du.
Source: https://gist.github.com/mikhailov-work/0d177465a8151eb6ede1768d51d476c7

`ColorMap::turbo` adapts the published polynomial coefficients to Rust and
samples them into a continuous native colour map. It does not use Matplotlib.
