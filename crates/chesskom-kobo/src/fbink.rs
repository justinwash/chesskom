//! Present a rendered board on the Kobo e-ink panel via FBInk.
//!
//! Why FBInk instead of writing `/dev/fb0` and firing our own refresh ioctl:
//! Kobo models are split across multiple SoC families (i.MX6 and Allwinner/sunxi),
//! and the e-ink *waveform refresh* interface differs between them — the ioctl
//! struct and update modes that work on an old Clara HD are not what a Clara BW
//! wants. FBInk (github.com/NiLuJe/FBInk) is the maintained C tool that already
//! handles every model's framebuffer format and refresh path. Shelling out to it
//! is the most reliable way to get correct pixels *and* a clean refresh on-device
//! without guessing hardware details we can't test here.
//!
//! A native, dependency-free direct-framebuffer backend is a planned follow-up
//! (see README); it needs on-device iteration to get the refresh right per model.

use chesskom_render::Canvas;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// How to push an image to the panel.
pub struct Fbink {
    /// Path to the fbink binary (usually just "fbink" on PATH, or a full path).
    pub binary: String,
    /// Clear the screen before drawing (avoids ghosting from previous content).
    pub clear: bool,
    /// Where to stage the PNG we hand to fbink.
    pub scratch: PathBuf,
}

impl Default for Fbink {
    fn default() -> Fbink {
        Fbink {
            binary: "fbink".to_string(),
            clear: true,
            scratch: PathBuf::from("/tmp/chesskom-board.png"),
        }
    }
}

impl Fbink {
    /// Encode `canvas` to PNG and display it centered on the panel.
    pub fn present(&self, canvas: &Canvas) -> io::Result<()> {
        let bytes = chesskom_render::png::encode_grayscale(canvas);
        std::fs::write(&self.scratch, bytes)?;
        self.present_file(&self.scratch)
    }

    /// Display an existing PNG file centered on the panel, with a full refresh.
    pub fn present_file(&self, png: &Path) -> io::Result<()> {
        if self.clear {
            // Best-effort clear; ignore failure so a missing feature doesn't abort.
            let _ = Command::new(&self.binary).arg("-c").status();
        }
        let g = format!(
            "file={},halign=CENTER,valign=CENTER",
            png.display()
        );
        let status = Command::new(&self.binary)
            .arg("-g")
            .arg(&g)
            .arg("-f") // full (flashing) refresh — best contrast for a whole board
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "fbink exited with status {status}; is FBInk installed on the device?"
            )))
        }
    }

    /// Print a short line of text to the panel (e.g. a status/error message).
    /// Used by the upcoming interactive layer for on-screen prompts.
    #[allow(dead_code)]
    pub fn print(&self, msg: &str) -> io::Result<()> {
        Command::new(&self.binary).arg(msg).status()?;
        Ok(())
    }
}
