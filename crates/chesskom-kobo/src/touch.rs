//! Reading taps from the Kobo touchscreen (Linux evdev).
//!
//! ⚠️ This is the one layer that can't be validated without the device. Kobo
//! models differ in (a) which `/dev/input/eventN` is the touchscreen, (b) the
//! `input_event` record size, and (c) how raw touch coordinates map to screen
//! pixels (axis swap/inversion, and the raw coordinate range). All of these are
//! configurable via environment variables so the mapping can be calibrated on the
//! device without recompiling — run with `--calibrate` to print raw taps and dial
//! them in. Defaults are a reasonable starting point for a portrait Kobo.
//!
//! Env vars:
//!   CHESSKOM_TOUCH_DEV     device path (default: autodetect, else /dev/input/event1)
//!   CHESSKOM_INPUT_STRUCT  input_event size in bytes (default 16; some builds 24)
//!   CHESSKOM_TOUCH_MAX_X   raw X range (default = screen width)
//!   CHESSKOM_TOUCH_MAX_Y   raw Y range (default = screen height)
//!   CHESSKOM_TOUCH_SWAP    "1" to swap X/Y
//!   CHESSKOM_TOUCH_INVX    "1" to invert X after scaling
//!   CHESSKOM_TOUCH_INVY    "1" to invert Y after scaling

use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;

// evdev type/code constants we care about.
const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const SYN_REPORT: u16 = 0x00;
const BTN_TOUCH: u16 = 0x14a;
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_MT_POSITION_X: u16 = 0x35;
const ABS_MT_POSITION_Y: u16 = 0x36;
const ABS_MT_TRACKING_ID: u16 = 0x39;

/// Calibration mapping from raw touch coordinates to screen pixels.
#[derive(Clone, Copy)]
pub struct Calibration {
    pub screen_w: u32,
    pub screen_h: u32,
    pub max_x: u32,
    pub max_y: u32,
    pub swap: bool,
    pub inv_x: bool,
    pub inv_y: bool,
}

impl Calibration {
    pub fn from_env(screen_w: u32, screen_h: u32) -> Calibration {
        let get = |k: &str| std::env::var(k).ok();
        let num = |k: &str, d: u32| get(k).and_then(|v| v.parse().ok()).unwrap_or(d);
        let flag = |k: &str| get(k).as_deref() == Some("1");
        Calibration {
            screen_w,
            screen_h,
            max_x: num("CHESSKOM_TOUCH_MAX_X", screen_w),
            max_y: num("CHESSKOM_TOUCH_MAX_Y", screen_h),
            swap: flag("CHESSKOM_TOUCH_SWAP"),
            inv_x: flag("CHESSKOM_TOUCH_INVX"),
            inv_y: flag("CHESSKOM_TOUCH_INVY"),
        }
    }

    /// Map a raw touch (rx, ry) to screen pixels (clamped to bounds).
    pub fn map(&self, rx: i32, ry: i32) -> (u32, u32) {
        let (rx, ry) = if self.swap { (ry, rx) } else { (rx, ry) };
        let mut sx = (rx as i64 * self.screen_w as i64 / self.max_x.max(1) as i64) as i32;
        let mut sy = (ry as i64 * self.screen_h as i64 / self.max_y.max(1) as i64) as i32;
        if self.inv_x {
            sx = self.screen_w as i32 - 1 - sx;
        }
        if self.inv_y {
            sy = self.screen_h as i32 - 1 - sy;
        }
        (
            sx.clamp(0, self.screen_w as i32 - 1) as u32,
            sy.clamp(0, self.screen_h as i32 - 1) as u32,
        )
    }
}

/// Blocking reader that yields a raw (x, y) each time a finger lifts (a tap).
pub struct TouchReader {
    file: File,
    rec: usize,
    cur_x: i32,
    cur_y: i32,
    have_pos: bool,
}

impl TouchReader {
    pub fn open() -> io::Result<TouchReader> {
        let dev = choose_device();
        let file = File::open(&dev).map_err(|e| {
            io::Error::new(e.kind(), format!("opening touch device {}: {e}", dev.display()))
        })?;
        let rec = std::env::var("CHESSKOM_INPUT_STRUCT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(16usize);
        Ok(TouchReader {
            file,
            rec,
            cur_x: 0,
            cur_y: 0,
            have_pos: false,
        })
    }

    /// Block until a tap (finger-up) is detected; return its raw coordinates.
    pub fn next_tap(&mut self) -> io::Result<(i32, i32)> {
        let mut buf = [0u8; 32];
        loop {
            self.file.read_exact(&mut buf[..self.rec])?;
            // type/code/value are always the final 8 bytes of the record,
            // regardless of the timeval size in front of them.
            let base = self.rec - 8;
            let etype = u16::from_le_bytes([buf[base], buf[base + 1]]);
            let code = u16::from_le_bytes([buf[base + 2], buf[base + 3]]);
            let value = i32::from_le_bytes([buf[base + 4], buf[base + 5], buf[base + 6], buf[base + 7]]);

            match etype {
                EV_ABS => match code {
                    ABS_X | ABS_MT_POSITION_X => {
                        self.cur_x = value;
                        self.have_pos = true;
                    }
                    ABS_Y | ABS_MT_POSITION_Y => {
                        self.cur_y = value;
                        self.have_pos = true;
                    }
                    ABS_MT_TRACKING_ID if value == -1 => {
                        if self.have_pos {
                            return Ok((self.cur_x, self.cur_y));
                        }
                    }
                    _ => {}
                },
                EV_KEY if code == BTN_TOUCH && value == 0 => {
                    if self.have_pos {
                        return Ok((self.cur_x, self.cur_y));
                    }
                }
                EV_SYN if code == SYN_REPORT => {}
                _ => {}
            }
        }
    }
}

/// Pick the touch device: env override, else the first plausible event node.
fn choose_device() -> PathBuf {
    if let Ok(p) = std::env::var("CHESSKOM_TOUCH_DEV") {
        return PathBuf::from(p);
    }
    // Kobo touchscreens are commonly event1; fall back to scanning for any node.
    let default = PathBuf::from("/dev/input/event1");
    if default.exists() {
        return default;
    }
    if let Ok(entries) = std::fs::read_dir("/dev/input") {
        for e in entries.flatten() {
            let p = e.path();
            if p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("event"))
                .unwrap_or(false)
            {
                return p;
            }
        }
    }
    default
}
