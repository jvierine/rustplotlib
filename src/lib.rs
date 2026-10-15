//! LaTeX is the typesetter. Rust owns the data, geometry and output.
//!
//! Panels are positioned in millimetres. Layers, axes and colorbars are
//! independent objects; no particular plot layout or theme is mandatory.
pub mod color;
pub mod article;
pub use article::ArticleLayout;
pub mod web_layout;
pub use web_layout::WebLayout;
pub mod figure;
pub mod latex;
pub mod legend;
pub mod markers;
pub use markers::{MarkerShape, MarkerStyle};
pub mod volume;
pub use color::{ColorMap, Normalization, Scale};
pub use figure::{
    Axes, Colorbar, Connector, Contour, Field, Figure, IntendedUse, Label, Layer, Line, Panel,
    PolygonMesh, QuadMesh, Rect, SecondaryAxis,
};
pub use latex::Latex;
pub use legend::{Legend, LegendEntry, LegendPosition, LegendSymbol};
mod palettes;

pub mod cartography;
pub use cartography::Projection;

pub mod surface;
pub use surface::Surface;
