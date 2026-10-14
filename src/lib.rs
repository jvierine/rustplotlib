//! LaTeX is the typesetter. Rust owns the data, geometry and output.
//!
//! Panels are positioned in millimetres. Layers, axes and colorbars are
//! independent objects; no particular plot layout or theme is mandatory.
pub mod color;
pub mod figure;
pub mod latex;
pub mod volume;
pub use color::{ColorMap, Normalization, Scale};
pub use figure::{
    Axes, Colorbar, Contour, Field, Figure, IntendedUse, Label, Layer, Line, Panel, QuadMesh, Rect,
    SecondaryAxis,
};
pub use latex::Latex;

pub mod cartography;
pub use cartography::Projection;

pub mod surface;
pub use surface::Surface;
