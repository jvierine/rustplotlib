//! Self-contained movie example; no source data or PNG frame sequence required.
use rustplotlib::*;
fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "streaming-animation.mp4".into());
    let options = AnimationOptions {
        width_px: 640,
        height_px: 360,
        input_fps: 8.,
        output_fps: 24.,
        ..Default::default()
    };
    let mut movie = AnimationWriter::new(path, options)?;
    for frame in 0..16 {
        let mut figure = Figure::new(IntendedUse::Custom {
            width_mm: 160.,
            font_pt: 10.,
        });
        figure.height_mm = 90.;
        let mut panel = Panel::new(
            Rect::new(20., 10., 130., 60.),
            Axes::new([0., 6.3], [-1.1, 1.1], "Phase (rad)", "Amplitude"),
        );
        let phase = frame as f64 * 0.2;
        panel.push(Layer::Line(Line::new(
            (0..100)
                .map(|i| {
                    let x = i as f64 * 6.3 / 99.;
                    [x, (x - phase).sin()]
                })
                .collect(),
            "#c63683",
        )));
        figure.panels.push(panel);
        movie.push(&figure)?;
    }
    movie.finish()
}
