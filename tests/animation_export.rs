#![cfg(feature = "export")]
use rustplotlib::*;
use std::process::Command;

#[test]
#[ignore = "requires FFmpeg/ffprobe with libx264 and the normal LaTeX export tools"]
fn streamed_mp4_preserves_frame_cadence_and_decodes() -> anyhow::Result<()> {
    let dir = std::env::temp_dir().join(format!("rustplotlib-animation-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("cadence.mp4");
    let mut writer = AnimationWriter::new(
        &path,
        AnimationOptions {
            width_px: 320,
            height_px: 180,
            input_fps: 8.,
            output_fps: 24.,
            ..Default::default()
        },
    )?;
    for y in [0.3, 0.7] {
        let mut figure = Figure::new(IntendedUse::Custom {
            width_mm: 160.,
            font_pt: 10.,
        });
        figure.height_mm = 90.;
        let mut panel = Panel::new(
            Rect::new(20., 10., 130., 60.),
            Axes::new([0., 1.], [0., 1.], "Time", "Power"),
        );
        panel.push(Layer::Line(Line::new(vec![[0., y], [1., y]], "#c63683")));
        figure.panels.push(panel);
        writer.push(&figure)?;
    }
    assert_eq!(writer.frame_count(), 2);
    writer.finish()?;
    let probe = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,nb_frames,duration",
            "-of",
            "json",
        ])
        .arg(&path)
        .output()?;
    assert!(probe.status.success());
    let info: serde_json::Value = serde_json::from_slice(&probe.stdout)?;
    let stream = &info["streams"][0];
    assert_eq!(stream["width"], 320);
    assert_eq!(stream["height"], 180);
    assert_eq!(stream["nb_frames"], "6");
    assert_eq!(stream["duration"], "0.250000");
    assert!(
        Command::new("ffmpeg")
            .args(["-v", "error", "-i"])
            .arg(&path)
            .args(["-f", "null", "-"])
            .status()?
            .success()
    );
    assert!(
        std::fs::read_dir(&dir)?.all(|p| p.unwrap().path().extension().is_none_or(|e| e != "png"))
    );
    std::fs::remove_dir_all(dir)?;
    Ok(())
}
