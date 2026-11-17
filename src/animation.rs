//! Fixed-canvas MP4 animation, streamed to FFmpeg without intermediate images.
//! Requires the `export` feature and an FFmpeg executable with libx264.
use crate::Figure;
use anyhow::{Context, Result, ensure};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};

/// Encoder settings. Input fps controls scientific-frame playback; output fps
/// controls video cadence. FFmpeg repeats/drops frames when these differ.
#[derive(Clone, Debug)]
pub struct AnimationOptions {
    pub width_px: u32,
    pub height_px: u32,
    pub input_fps: f64,
    pub output_fps: f64,
    pub crf: u8,
    pub preset: String,
    pub threads: usize,
    pub ffmpeg: PathBuf,
}
impl Default for AnimationOptions {
    fn default() -> Self {
        Self {
            width_px: 1920,
            height_px: 1080,
            input_fps: 24.,
            output_fps: 24.,
            crf: 18,
            preset: "medium".into(),
            threads: 2,
            ffmpeg: "ffmpeg".into(),
        }
    }
}
impl AnimationOptions {
    fn validate(&self) -> Result<()> {
        ensure!(
            self.width_px > 0
                && self.height_px > 0
                && self.width_px % 2 == 0
                && self.height_px % 2 == 0,
            "H.264 yuv420p dimensions must be positive and even"
        );
        ensure!(
            self.input_fps.is_finite()
                && self.input_fps > 0.
                && self.output_fps.is_finite()
                && self.output_fps > 0.,
            "Invalid frame rate"
        );
        ensure!(
            self.crf <= 51 && self.threads > 0,
            "Invalid CRF or thread count"
        );
        ensure!(
            [
                "ultrafast",
                "superfast",
                "veryfast",
                "faster",
                "fast",
                "medium",
                "slow",
                "slower",
                "veryslow"
            ]
            .contains(&self.preset.as_str()),
            "Unsupported x264 preset"
        );
        Ok(())
    }
}

/// Owns one encoder process. Call `push` once per scientific frame and then
/// `finish` to check encoding success. Only one raster is held at a time.
///
/// Figure canvas geometry is retained (no content-dependent trimming); a
/// different aspect ratio is letterboxed in white, never stretched. No labels,
/// credits or provenance are added. Store scientific metadata separately.
pub struct AnimationWriter {
    options: AnimationOptions,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    frames: usize,
}
impl AnimationWriter {
    pub fn new(path: impl AsRef<Path>, options: AnimationOptions) -> Result<Self> {
        options.validate()?;
        let mut child = Command::new(&options.ffmpeg)
            .args([
                "-y",
                "-v",
                "error",
                "-f",
                "rawvideo",
                "-pixel_format",
                "rgba",
                "-video_size",
            ])
            .arg(format!("{}x{}", options.width_px, options.height_px))
            .arg("-framerate")
            .arg(options.input_fps.to_string())
            .args(["-i", "pipe:0", "-an", "-vf"])
            .arg(format!("fps={},format=yuv420p", options.output_fps))
            .args(["-c:v", "libx264", "-threads"])
            .arg(options.threads.to_string())
            .arg("-preset")
            .arg(&options.preset)
            .arg("-crf")
            .arg(options.crf.to_string())
            .args(["-movflags", "+faststart"])
            .arg(path.as_ref())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .context("Starting FFmpeg animation encoder")?;
        let stdin = child.stdin.take().context("Missing encoder input pipe")?;
        Ok(Self {
            options,
            child: Some(child),
            stdin: Some(stdin),
            frames: 0,
        })
    }
    pub fn push(&mut self, figure: &Figure) -> Result<()> {
        let svg = figure.svg()?;
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default())?;
        let mut pix = resvg::tiny_skia::Pixmap::new(self.options.width_px, self.options.height_px)
            .context("Invalid animation raster dimensions")?;
        pix.fill(resvg::tiny_skia::Color::WHITE);
        let scale = (self.options.width_px as f32 / tree.size().width())
            .min(self.options.height_px as f32 / tree.size().height());
        let dx = (self.options.width_px as f32 - tree.size().width() * scale) / 2.;
        let dy = (self.options.height_px as f32 - tree.size().height() * scale) / 2.;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale).post_translate(dx, dy),
            &mut pix.as_mut(),
        );
        self.stdin
            .as_mut()
            .context("Animation encoder already closed")?
            .write_all(pix.data())
            .context("Streaming frame to FFmpeg")?;
        self.frames += 1;
        Ok(())
    }
    pub fn frame_count(&self) -> usize {
        self.frames
    }
    pub fn finish(mut self) -> Result<()> {
        drop(self.stdin.take());
        let status = self
            .child
            .take()
            .context("Animation encoder already closed")?
            .wait()?;
        ensure!(self.frames > 0, "Cannot encode an empty animation");
        ensure!(
            status.success(),
            "FFmpeg animation encoder failed: {status}"
        );
        Ok(())
    }
}
impl Drop for AnimationWriter {
    fn drop(&mut self) {
        drop(self.stdin.take());
        // An abandoned/failed writer must not leave an encoder or zombie behind.
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reject_invalid_settings_before_spawning() {
        for options in [
            AnimationOptions {
                width_px: 0,
                ..Default::default()
            },
            AnimationOptions {
                height_px: 7,
                ..Default::default()
            },
            AnimationOptions {
                input_fps: f64::NAN,
                ..Default::default()
            },
            AnimationOptions {
                input_fps: 0.,
                ..Default::default()
            },
            AnimationOptions {
                crf: 52,
                ..Default::default()
            },
        ] {
            assert!(options.validate().is_err());
        }
    }
}
