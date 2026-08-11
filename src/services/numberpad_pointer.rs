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

//! Virtual touchpad that keeps pointing alive while the NumberPad is lit.
//!
//! `EVIOCGRAB` is all-or-nothing, so while [`crate::services::numberpad`]
//! holds the touchpad the compositor sees nothing and the cursor freezes. The
//! ASUS firmware keeps the pad a pointing device and only lets *taps* type a
//! digit; to match that, the touchpad is cloned into a uinput device and the
//! grabbed frames are replayed onto it once the touch turns out to be
//! pointing rather than typing.
//!
//! That verdict takes a few frames, so frames are withheld from the moment
//! the finger lands and flushed on [`PointerRelay::commit`] - the compositor
//! still gets the gesture from its true beginning.

use evdev::uinput::VirtualDevice;
use evdev::{AbsInfo, Device, EventType, InputEvent, SynchronizationCode, UinputAbsSetup};

/// Name the clone advertises, as shown in the desktop's input settings.
const RELAY_NAME: &str = "Ayuz NumberPad Touchpad";

/// Ceiling on withheld events, ~4 seconds of a jittering contact. Reaching it
/// means a finger is simply resting on the pad; replaying that much history
/// later would fling the cursor, so the backlog is dropped instead.
const MAX_HELD_EVENTS: usize = 2048;

/// Replays grabbed touchpad frames onto a virtual clone of the touchpad.
pub struct PointerRelay {
    device: VirtualDevice,
    /// Events of the frame currently being read, without its `SYN_REPORT`.
    frame: Vec<InputEvent>,
    /// Completed frames withheld while the verdict on this touch is open,
    /// keeping their `SYN_REPORT` separators so a flush replays real frames.
    held: Vec<InputEvent>,
    /// Set when the backlog outgrew [`MAX_HELD_EVENTS`] and was thrown away.
    stalled: bool,
    /// Set once this touch is committed to pointing; frames then go out live.
    forwarding: bool,
}

impl PointerRelay {
    /// Clones `source`'s properties, keys, absolute axes and device id, so
    /// udev classifies the result as a touchpad and libinput applies the same
    /// model quirks as for the real one.
    pub fn new(source: &Device) -> std::io::Result<Self> {
        let abs_state = source.get_abs_state()?;
        let mut builder = VirtualDevice::builder()?
            .name(RELAY_NAME)
            .input_id(source.input_id())
            .with_properties(source.properties())?;
        if let Some(keys) = source.supported_keys() {
            builder = builder.with_keys(keys)?;
        }
        if let Some(axes) = source.supported_absolute_axes() {
            for axis in axes.iter() {
                let info = AbsInfo::from(abs_state[axis.0 as usize]);
                builder = builder.with_absolute_axis(&UinputAbsSetup::new(axis, info))?;
            }
        }
        Ok(Self {
            device: builder.build()?,
            frame: Vec::new(),
            held: Vec::new(),
            stalled: false,
            forwarding: false,
        })
    }

    /// Records one event of the frame being read. Only key and absolute-axis
    /// events are relayed; the clone advertises nothing else.
    pub fn push(&mut self, event: InputEvent) {
        if event.event_type() == EventType::KEY || event.event_type() == EventType::ABSOLUTE {
            self.frame.push(event);
        }
    }

    /// Closes the frame at its `SYN_REPORT`: sent straight out once the touch
    /// is committed to pointing, withheld until then.
    pub fn end_frame(&mut self) {
        if self.frame.is_empty() {
            return;
        }
        if self.forwarding {
            if let Err(e) = self.device.emit(&self.frame) {
                tracing::warn!("NumberPad: pointer relay emit failed: {}", e);
            }
            self.frame.clear();
        } else if self.stalled {
            self.frame.clear();
        } else {
            self.held.append(&mut self.frame);
            self.held.push(syn_report());
            if self.held.len() > MAX_HELD_EVENTS {
                self.held.clear();
                self.stalled = true;
            }
        }
    }

    /// Declares the current touch to be pointing, not typing: the backlog is
    /// replayed at once and later frames go out live.
    pub fn commit(&mut self) {
        if self.forwarding {
            return;
        }
        self.forwarding = true;
        if self.stalled || self.held.is_empty() {
            self.held.clear();
            return;
        }
        // `emit` ends every write with a `SYN_REPORT`, so the trailing
        // separator would only add an empty frame.
        self.held.pop();
        if let Err(e) = self.device.emit(&self.held) {
            tracing::warn!("NumberPad: pointer relay flush failed: {}", e);
        }
        self.held.clear();
    }

    /// Ends the touch. Anything still withheld belongs to a tap that typed a
    /// key, so it is dropped.
    pub fn reset(&mut self) {
        self.frame.clear();
        self.held.clear();
        self.stalled = false;
        self.forwarding = false;
    }
}

/// The frame separator kept inside [`PointerRelay::held`].
fn syn_report() -> InputEvent {
    InputEvent::new(
        EventType::SYNCHRONIZATION.0,
        SynchronizationCode::SYN_REPORT.0,
        0,
    )
}
