// Ayuz - Unofficial Control Center for Asus Laptops
// Copyright (C) 2026 Guido Philipp
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see https://www.gnu.org/licenses/.

//! Native ASUS NumberPad backend.
//!
//! Two responsibilities, tied together in [`run_loop`]:
//!
//! 1. **LED control** - the NumberPad LEDs are toggled by writing a 13-byte
//!    "magic packet" to the touchpad's I2C slave (`0x15` or `0x38`). The
//!    bus number is discovered by parsing `/proc/bus/input/devices`.
//! 2. **Touch interception** - while in *Active* mode the touchpad is
//!    `EVIOCGRAB`-ed, and taps are translated to numeric-keypad **keycodes**
//!    (`KEY_KP0`..`KEY_KPENTER`) emitted through a `uinput` virtual device.
//!    Emitting *keycodes*, not characters, lets the compositor apply the
//!    user's keyboard layout (German, Lithuanian, ...) natively. Touches that
//!    turn out to be drags, gestures or clicks go back to the compositor via
//!    [`crate::services::numberpad_pointer`], so the cursor keeps working
//!    while the grid is lit, as it does on the ASUS firmware.
//!
//! Activation is two-tiered (matches the UI):
//! - `shutdown` (one channel): when fired, the loop exits and ungrabs.
//! - `active_rx` (another channel): toggled at runtime via the IPC socket
//!   (`ayuz --toggle-numberpad`). `false` = Idle (LEDs off, no grab),
//!   `true` = Active (LEDs on, grab, emit keys).

use std::fs;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use evdev::uinput::VirtualDevice;
use evdev::{
    AbsoluteAxisCode, AttributeSet, Device, EventSummary, EventType, InputEvent, KeyCode,
    SynchronizationCode,
};
use tokio::sync::{mpsc, watch};

use crate::services::evdev_runner::{find_touchpad, open_event_stream, touchpad_abs_bounds};
use crate::services::numberpad_layouts::{self, Layout};
use crate::services::numberpad_pointer::PointerRelay;
use crate::sys_paths::{DEV_UINPUT, PROC_BUS_INPUT_DEVICES, SYS_PRODUCT_NAME};

/// Linux ioctl request number for `I2C_SLAVE_FORCE` (set slave address even
/// if another kernel driver currently holds the device). The plain
/// `I2C_SLAVE` ioctl (`0x0703`) fails with `EBUSY` because `i2c_hid` is
/// permanently bound to the touchpad. The vendor LED register we target
/// lives outside the HID conversation, so taking the slot by force is safe
/// and matches what the upstream Python `asus-numberpad-driver` does.
const I2C_SLAVE_FORCE: libc::c_ulong = 0x0706;

/// The 13-byte LED-control packet. Bytes 0..11 are constant, byte 11 is the
/// state, byte 12 is the terminator `0xad`.
const PACKET_HEADER: [u8; 11] = [
    0x05, 0x00, 0x3d, 0x03, 0x06, 0x00, 0x07, 0x00, 0x0d, 0x14, 0x03,
];
const PACKET_TERMINATOR: u8 = 0xad;

/// Sent once on activation to unlock the controller before the enable byte.
const STATE_UNLOCK: u8 = 0x60;
/// LEDs on.
const STATE_ENABLE: u8 = 0x01;
/// LEDs off.
const STATE_DISABLE: u8 = 0x00;

/// Result of a startup hardware probe. The component renders different UI
/// for each variant.
#[derive(Debug, Clone)]
pub enum NumberpadStatus {
    /// Hardware is present and accessible; the feature can be enabled.
    Ok,
    /// No ELAN/ASUE/ASUP/ASUF touchpad was found in `/proc/bus/input/devices`.
    NoHardware,
    /// I2C bus could not be opened. Usually `i2c-dev` module is not loaded.
    I2cUnavailable(String),
    /// Either `/dev/i2c-N` or `/dev/uinput` is not writable by the current
    /// user. The string names the offending device path so the UI can render
    /// an actionable hint.
    PermissionDenied { device: String },
}

/// Locates the `/dev/i2c-N` bus and the slave address for the ASUS-family
/// touchpad on this system, by parsing `/proc/bus/input/devices`.
///
/// We can't derive this from the `evdev::Device` returned by
/// `find_touchpad()` because the evdev API doesn't expose the device's
/// underlying sysfs path. The two pieces of information come from different
/// kernel surfaces, so we keep this private parser focused on the i2c side
/// and let `find_touchpad()` handle the evdev side.
fn detect_i2c_target() -> Option<(PathBuf, u16)> {
    let contents = fs::read_to_string(PROC_BUS_INPUT_DEVICES).ok()?;

    // A block in /proc/bus/input/devices is separated by blank lines and
    // contains lines like:
    //   I: Bus=0018 Vendor=04f3 Product=...
    //   N: Name="ASUE140D:00 04F3:319F Touchpad"
    //   S: Sysfs=/devices/pci0000:00/.../i2c-7/...
    for block in contents.split("\n\n") {
        let mut name_line: Option<&str> = None;
        let mut sysfs_line: Option<&str> = None;
        for line in block.lines() {
            if let Some(rest) = line.strip_prefix("N: ") {
                name_line = Some(rest);
            } else if let Some(rest) = line.strip_prefix("S: ") {
                sysfs_line = Some(rest);
            }
        }

        let name = match name_line {
            Some(n) => n,
            None => continue,
        };
        if !name.contains("Touchpad") {
            continue;
        }
        // ASUS NumberPad-capable controllers identify themselves via these
        // family prefixes inside the device name string.
        let family_match = name.contains("ASUE")
            || name.contains("ELAN")
            || name.contains("ASUP")
            || name.contains("ASUF");
        if !family_match {
            continue;
        }
        // Upstream Python driver excludes these explicitly as false positives.
        if name.contains("9009") || name.contains("9008") {
            continue;
        }

        // Pick I2C address. The handful of ASUF models the upstream driver
        // recognises use a non-default slave address.
        let i2c_addr: u16 = if name.contains("ASUF1416")
            || name.contains("ASUF1205")
            || name.contains("ASUF1204")
        {
            0x38
        } else {
            0x15
        };

        // Extract the i2c bus number from the Sysfs path (e.g. ".../i2c-7/...").
        let bus = sysfs_line.and_then(parse_i2c_bus)?;
        let i2c_path = PathBuf::from(format!("/dev/i2c-{}", bus));

        return Some((i2c_path, i2c_addr));
    }
    None
}

/// Parses `i2c-N` from a sysfs path. Returns the bus number `N`.
fn parse_i2c_bus(sysfs: &str) -> Option<u32> {
    // Walk segments separated by `/` and pick the last `i2c-<digits>` one.
    sysfs
        .split('/')
        .filter_map(|seg| seg.strip_prefix("i2c-").and_then(|rest| rest.parse::<u32>().ok()))
        .next_back()
}

/// Verifies write access to the given path without modifying it.
fn is_writable(path: &Path) -> bool {
    // Returns true only if open(O_WRONLY) succeeds. We do not actually
    // write anything; the file is closed on drop.
    fs::OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .is_ok()
}

/// Inspects the system to decide whether the NumberPad feature is available.
/// Cheap enough to call on every UI start.
pub async fn probe() -> NumberpadStatus {
    let Some((i2c_path, _)) = detect_i2c_target() else {
        return NumberpadStatus::NoHardware;
    };

    if !i2c_path.exists() {
        return NumberpadStatus::I2cUnavailable(i2c_path.display().to_string());
    }
    if !is_writable(&i2c_path) {
        return NumberpadStatus::PermissionDenied {
            device: i2c_path.display().to_string(),
        };
    }
    if !Path::new(DEV_UINPUT).exists() {
        return NumberpadStatus::I2cUnavailable(DEV_UINPUT.to_string());
    }
    if !is_writable(Path::new(DEV_UINPUT)) {
        return NumberpadStatus::PermissionDenied {
            device: DEV_UINPUT.to_string(),
        };
    }

    NumberpadStatus::Ok
}

/// Writes one 13-byte LED-control packet to the touchpad's I2C slave.
fn i2c_send(i2c_path: &Path, addr: u16, state: u8) -> std::io::Result<()> {
    let mut file = fs::OpenOptions::new().write(true).open(i2c_path)?;
    // SAFETY: I2C_SLAVE_FORCE just stores the address in the kernel-side fd
    // state, even when another driver (here: i2c_hid) has claimed the device.
    let rc = unsafe { libc::ioctl(file.as_raw_fd(), I2C_SLAVE_FORCE, addr as libc::c_int) };
    if rc < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let mut packet = [0u8; 13];
    packet[..11].copy_from_slice(&PACKET_HEADER);
    packet[11] = state;
    packet[12] = PACKET_TERMINATOR;
    file.write_all(&packet)?;
    Ok(())
}

/// Builds the virtual uinput keyboard that emits numpad keycodes.
fn build_virtual_device() -> std::io::Result<VirtualDevice> {
    let keys: AttributeSet<KeyCode> = AttributeSet::from_iter([
        KeyCode::KEY_KP0,
        KeyCode::KEY_KP1,
        KeyCode::KEY_KP2,
        KeyCode::KEY_KP3,
        KeyCode::KEY_KP4,
        KeyCode::KEY_KP5,
        KeyCode::KEY_KP6,
        KeyCode::KEY_KP7,
        KeyCode::KEY_KP8,
        KeyCode::KEY_KP9,
        KeyCode::KEY_KPDOT,
        KeyCode::KEY_KPENTER,
        KeyCode::KEY_KPPLUS,
        KeyCode::KEY_KPMINUS,
        KeyCode::KEY_KPASTERISK,
        KeyCode::KEY_KPSLASH,
        KeyCode::KEY_BACKSPACE,
        KeyCode::KEY_NUMLOCK,
        KeyCode::KEY_KPEQUAL,
        KeyCode::KEY_LEFTSHIFT,
        KeyCode::KEY_5,
    ]);
    VirtualDevice::builder()?
        .name("Ayuz NumberPad")
        .with_keys(&keys)?
        .build()
}

/// Reads `SYS_PRODUCT_NAME` for layout lookup, trimming whitespace.
fn read_product_name() -> String {
    fs::read_to_string(SYS_PRODUCT_NAME)
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Fractional bounds of the top-right corner activation zone, expressed
/// relative to the touchpad's absolute X / Y range. Proportional so the
/// same values work across touchpads with different reported resolutions.
const TOP_RIGHT_ZONE_X_MIN_FRAC: f64 = 0.85;
const TOP_RIGHT_ZONE_Y_MAX_FRAC: f64 = 0.15;

/// Top-right corner activation zone. Tuned to match what a user can hit
/// without precision: rightmost 15 % horizontally and topmost 15 % vertically.
fn in_top_right_zone(x: i32, y: i32, x_max: i32, y_max: i32) -> bool {
    (x as f64) > (x_max as f64) * TOP_RIGHT_ZONE_X_MIN_FRAC
        && (y as f64) < (y_max as f64) * TOP_RIGHT_ZONE_Y_MAX_FRAC
}

/// State of the corner-tap hold detector. `Tracking` means the finger is
/// currently inside the top-right zone and the 1-second timer is armed.
#[derive(Copy, Clone)]
enum HoldTimer {
    Idle,
    Tracking { deadline: tokio::time::Instant },
}

/// How long the user must hold a finger in the top-right zone to trigger
/// the activation flip.
const HOLD_DURATION: std::time::Duration = std::time::Duration::from_millis(1000);

/// How far a finger may drift, as a fraction of each axis' range, before its
/// touch counts as pointing instead of as a key tap. ~4 % is a few
/// millimetres: above the jitter of a deliberate tap, well below a cursor
/// move.
const DRAG_SLOP_FRAC: f64 = 0.04;

/// What the finger on the pad is doing while we own the touchpad. A touch
/// landing on a key starts as a candidate tap and is demoted to `Pointer`
/// once it drags, gains a second finger or clicks - which is what lets taps
/// type digits while drags still move the cursor.
#[derive(Copy, Clone)]
enum Touch {
    /// No finger on the pad.
    None,
    /// Single finger resting on `cell`, so far without moving.
    Key { cell: usize, start_x: i32, start_y: i32 },
    /// Committed to pointing: frames go through to the virtual touchpad.
    Pointer,
}

/// Whether a finger has drifted far enough from where it landed to count as
/// a drag rather than a tap.
fn dragged(dx: i32, dy: i32, x_max: i32, y_max: i32) -> bool {
    (dx.abs() as f64) > (x_max as f64) * DRAG_SLOP_FRAC
        || (dy.abs() as f64) > (y_max as f64) * DRAG_SLOP_FRAC
}

/// Computes a cell index from a touch point. Returns `None` if either axis
/// has zero range (defensive: would otherwise divide by zero) or the
/// resulting cell has no key.
fn cell_for(x: i32, y: i32, x_max: i32, y_max: i32, layout: &Layout) -> Option<usize> {
    if x_max <= 0 || y_max <= 0 {
        return None;
    }
    let cols = layout.cols as i32;
    let rows = layout.rows as i32;
    let col = ((x as i64 * cols as i64) / x_max as i64).clamp(0, (cols - 1) as i64) as usize;
    let row = ((y as i64 * rows as i64) / y_max as i64).clamp(0, (rows - 1) as i64) as usize;
    let idx = row * cols as usize + col;
    layout.cells.get(idx).copied().flatten().map(|_| idx)
}

/// Checks whether *any* connected evdev device currently reports the NumLock
/// LED as on. Iterating across all devices (not just the first keyboard) is
/// deliberate: on some laptops only one of several input devices exposes the
/// LED state, and a wrong `false` here would flip the user's NumLock to OFF.
fn is_numlock_on() -> bool {
    for (_, dev) in evdev::enumerate() {
        if let Ok(leds) = dev.get_led_state()
            && leds.contains(evdev::LedCode::LED_NUML)
        {
            return true;
        }
    }
    false
}

/// Ensures NumLock is on so that emitted `KEY_KP*` codes are interpreted as
/// digits, not navigation keys. Without NumLock, KEY_KP1..9 become End/Down/
/// PgDn/Left/Begin/Right/Home/Up/PgUp (operators are NumLock-independent and
/// keep working either way).
///
/// Called only on Idle→Active transitions; on deactivation we deliberately
/// leave NumLock on, because we cannot know whether the user originally had
/// it on themselves and an idempotent "always on after Ayuz" is less
/// surprising than a state we silently restore.
fn ensure_numlock_on(virt: &mut VirtualDevice) {
    if !is_numlock_on() {
        emit_tap(virt, &[KeyCode::KEY_NUMLOCK]);
    }
}

/// Main NumberPad event loop. Spawn via `tokio::spawn`; exit by sending a
/// new value on `shutdown`. `active_rx` flips Idle/Active without exiting.
/// `feedback_tx` notifies the component when the active state was toggled
/// from inside this loop (i.e. by the on-touchpad corner-tap gesture).
pub async fn run_loop(
    mut shutdown: watch::Receiver<bool>,
    mut active_rx: watch::Receiver<bool>,
    feedback_tx: mpsc::UnboundedSender<bool>,
) {
    let Some((i2c_path, i2c_addr)) = detect_i2c_target() else {
        tracing::warn!("NumberPad: no i2c target detected at run_loop start");
        return;
    };

    let device = match find_touchpad() {
        Some(d) => d,
        None => {
            tracing::warn!("NumberPad: no touchpad evdev device found");
            return;
        }
    };

    let Some((x_max, y_max)) = touchpad_abs_bounds(&device) else {
        tracing::warn!("NumberPad: touchpad reported invalid absolute range");
        return;
    };

    let layout = numberpad_layouts::for_product(&read_product_name());

    let mut virt = match build_virtual_device() {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("NumberPad: uinput device creation failed: {}", e);
            return;
        }
    };

    // Losing this only costs pointing while the grid is lit, so a failure
    // here must not take the whole feature down with it.
    let mut relay = match PointerRelay::new(&device) {
        Ok(r) => Some(r),
        Err(e) => {
            tracing::warn!("NumberPad: pointer relay unavailable: {}", e);
            None
        }
    };

    let Some(mut stream) = open_event_stream(device) else {
        return;
    };

    // Local state. The LEDs follow `active` at once; `grabbed` lags behind
    // while a finger is on the pad - see `reconcile_grab`.
    let mut active = *active_rx.borrow();
    let mut grabbed = false;
    let mut touch_down = false;
    let mut frame_open = false;
    let mut touch_ended = false;
    let mut cur_x: i32 = 0;
    let mut cur_y: i32 = 0;
    let mut touch = Touch::None;
    let mut hold = HoldTimer::Idle;

    // Apply the initial state (LEDs + grab) if we entered Active immediately.
    if active {
        set_leds(&i2c_path, i2c_addr, true);
        reconcile_grab(stream.device_mut(), &mut virt, true, &mut grabbed);
    }

    loop {
        tokio::select! {
            _ = shutdown.changed() => break,
            changed = active_rx.changed() => {
                if changed.is_err() {
                    break;
                }
                let new_active = *active_rx.borrow();
                if new_active != active {
                    active = new_active;
                    set_leds(&i2c_path, i2c_addr, active);
                }
            }
            ev = stream.next_event() => {
                let event = match ev {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::warn!("NumberPad: event read failed: {}", e);
                        break;
                    }
                };
                if event.event_type() != EventType::SYNCHRONIZATION {
                    frame_open = true;
                }
                // Feed the relay first; it withholds the frame until the
                // verdict below says where it belongs.
                if grabbed && let Some(relay) = relay.as_mut() {
                    relay.push(event);
                }
                match event.destructure() {
                    EventSummary::Key(_, KeyCode::BTN_TOUCH, 1) => {
                        // Finger went down. Arm the corner-tap hold timer if
                        // the press lands in the top-right zone.
                        touch_down = true;
                        if in_top_right_zone(cur_x, cur_y, x_max, y_max) {
                            hold = HoldTimer::Tracking {
                                deadline: tokio::time::Instant::now() + HOLD_DURATION,
                            };
                        }
                        if grabbed {
                            // A press with no key under it has nothing to
                            // type, so it is pointing from the first frame.
                            touch = match cell_for(cur_x, cur_y, x_max, y_max, layout) {
                                Some(cell) => Touch::Key { cell, start_x: cur_x, start_y: cur_y },
                                None => Touch::Pointer,
                            };
                            if matches!(touch, Touch::Pointer)
                                && let Some(relay) = relay.as_mut()
                            {
                                relay.commit();
                            }
                        }
                    }
                    EventSummary::Key(_, KeyCode::BTN_TOUCH, 0) => {
                        // Finger lifted. Cancel any pending hold (fast taps in
                        // the corner still emit normally via the cell logic
                        // below because hold cancellation happens *before* the
                        // gesture-fire branch could win the select).
                        touch_down = false;
                        touch_ended = true;
                        hold = HoldTimer::Idle;
                        if grabbed
                            && let Touch::Key { cell, .. } = touch
                            && cell_for(cur_x, cur_y, x_max, y_max, layout) == Some(cell)
                            && let Some(c) = layout.cells[cell]
                        {
                            emit_tap(&mut virt, c.keys);
                        }
                        touch = Touch::None;
                    }
                    // A second finger or a physical click: the user is
                    // pointing, not typing.
                    EventSummary::Key(
                        _,
                        KeyCode::BTN_TOOL_DOUBLETAP
                        | KeyCode::BTN_TOOL_TRIPLETAP
                        | KeyCode::BTN_TOOL_QUADTAP
                        | KeyCode::BTN_TOOL_QUINTTAP
                        | KeyCode::BTN_LEFT,
                        1,
                    ) => {
                        hold = HoldTimer::Idle;
                        if grabbed {
                            touch = Touch::Pointer;
                            if let Some(relay) = relay.as_mut() {
                                relay.commit();
                            }
                        }
                    }
                    EventSummary::AbsoluteAxis(_, axis, value) => {
                        match axis {
                            AbsoluteAxisCode::ABS_X | AbsoluteAxisCode::ABS_MT_POSITION_X => {
                                cur_x = value;
                            }
                            AbsoluteAxisCode::ABS_Y | AbsoluteAxisCode::ABS_MT_POSITION_Y => {
                                cur_y = value;
                            }
                            _ => {}
                        }
                        if matches!(hold, HoldTimer::Tracking { .. })
                            && !in_top_right_zone(cur_x, cur_y, x_max, y_max)
                        {
                            hold = HoldTimer::Idle;
                        }
                        if let Touch::Key { start_x, start_y, .. } = touch
                            && dragged(cur_x - start_x, cur_y - start_y, x_max, y_max)
                        {
                            touch = Touch::Pointer;
                            if let Some(relay) = relay.as_mut() {
                                relay.commit();
                            }
                        }
                    }
                    EventSummary::Synchronization(_, SynchronizationCode::SYN_REPORT, _) => {
                        frame_open = false;
                        if let Some(relay) = relay.as_mut() {
                            relay.end_frame();
                            if touch_ended {
                                relay.reset();
                            }
                        }
                        touch_ended = false;
                    }
                    _ => {}
                }
            }
            // Fourth branch: fires when the hold timer's deadline elapses.
            // `std::future::pending` is the canonical "never resolves" future,
            // used here so the branch is inert when no hold is in progress.
            () = async {
                match hold {
                    HoldTimer::Tracking { deadline } => tokio::time::sleep_until(deadline).await,
                    HoldTimer::Idle => std::future::pending::<()>().await,
                }
            } => {
                active = !active;
                // The gesture has no other feedback, so light the grid on the
                // hold itself rather than on release.
                set_leds(&i2c_path, i2c_addr, active);
                hold = HoldTimer::Idle;
                // Suppress the cell emit that would otherwise fire on the
                // upcoming BTN_TOUCH=0: the corner hold has consumed this touch.
                touch = Touch::None;
                let _ = feedback_tx.send(active);
            }
        }

        // The grab half of the flip lands here, not inline: it may only
        // happen between touches - see `reconcile_grab`.
        if !touch_down && !frame_open {
            reconcile_grab(stream.device_mut(), &mut virt, active, &mut grabbed);
        }
    }

    // Clean up: LEDs off, ungrab. Mirror the Idle transition regardless of
    // whichever state we were in when the shutdown fired.
    if grabbed {
        if let Err(e) = stream.device_mut().ungrab() {
            tracing::warn!("NumberPad: evdev ungrab failed: {}", e);
        }
    }
    if active {
        set_leds(&i2c_path, i2c_addr, false);
    }
}

/// Grabs or releases the touchpad to match `desired`, and does nothing when
/// it already does.
///
/// Callers must only reach here with no finger on the pad and no event frame
/// half-read. `EVIOCGRAB` diverts events the instant it takes effect, so
/// grabbing mid-touch leaves the compositor with a touch that never ends:
/// libinput keeps the slot alive and, since the corner press that turns the
/// NumberPad on sits in its right-edge palm zone, keeps it as a palm forever.
/// Later single-finger touches reuse that slot and are written off as the
/// same palm, killing pointing until the device is recreated at reboot.
fn reconcile_grab(
    device: &mut Device,
    virt: &mut VirtualDevice,
    desired: bool,
    grabbed: &mut bool,
) {
    if desired == *grabbed {
        return;
    }
    let result = if desired { device.grab() } else { device.ungrab() };
    if let Err(e) = result {
        tracing::warn!("NumberPad: evdev grab({}) failed: {}", desired, e);
    }
    if desired {
        ensure_numlock_on(virt);
    }
    *grabbed = desired;
}

/// Drives the NumberPad LEDs. The controller wants an unlock packet before it
/// accepts the enable byte. Logs and continues on failure - a dark grid
/// should not stop the keys from working.
///
/// Separate from the grab above because the two are applied at different
/// moments: the LEDs follow the user's toggle immediately.
fn set_leds(i2c_path: &Path, i2c_addr: u16, on: bool) {
    if on {
        if let Err(e) = i2c_send(i2c_path, i2c_addr, STATE_UNLOCK) {
            tracing::warn!("NumberPad: i2c unlock failed: {}", e);
        }
        if let Err(e) = i2c_send(i2c_path, i2c_addr, STATE_ENABLE) {
            tracing::warn!("NumberPad: i2c enable failed: {}", e);
        }
    } else if let Err(e) = i2c_send(i2c_path, i2c_addr, STATE_DISABLE) {
        tracing::warn!("NumberPad: i2c disable failed: {}", e);
    }
}

/// Emits a press-then-release sequence for `keys` through the virtual device.
/// Presses fire in order, releases fire in reverse — so for `[Shift, 5]` the
/// timeline is Shift-down, 5-down, 5-up, Shift-up, which is the textbook
/// modifier ordering. `emit` auto-appends a `SYN_REPORT` per call.
fn emit_tap(virt: &mut VirtualDevice, keys: &[KeyCode]) {
    for key in keys {
        let press = InputEvent::new(evdev::EventType::KEY.0, key.0, 1);
        if let Err(e) = virt.emit(&[press]) {
            tracing::warn!("NumberPad: emit press failed: {}", e);
            return;
        }
    }
    for key in keys.iter().rev() {
        let release = InputEvent::new(evdev::EventType::KEY.0, key.0, 0);
        if let Err(e) = virt.emit(&[release]) {
            tracing::warn!("NumberPad: emit release failed: {}", e);
        }
    }
}
