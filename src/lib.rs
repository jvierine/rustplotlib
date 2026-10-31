//! LaTeX is the typesetter. Rust owns the data, geometry and output.
//!
//! Panels are positioned in millimetres. Layers, axes and colorbars are
//! independent objects; no particular plot layout or theme is mandatory.
pub mod annotation;
pub mod color;
pub mod format;
pub use format::scientific_text;
pub mod axis_scale;
pub use axis_scale::AxisScale;
pub mod article;
pub use article::ArticleLayout;
pub mod web_layout;
pub use web_layout::WebLayout;
pub mod figure;
pub mod latex;
pub mod legend;
pub mod marker_groups;
pub mod markers;
pub use marker_groups::MarkerGroup;
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
pub mod perspective;
pub use perspective::PerspectiveCamera;
