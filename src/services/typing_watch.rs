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

//! Palm rejection: watches every keyboard for typing activity and suppresses
//! touchpad input while the user types.
//!
//! Two independent actions are supported (see [`TypingConfig`]):
//! - suppress the edge gestures of [`crate::services::edge_gestures`], and/or
//! - disable the touchpad entirely via [`crate::services::touchpad_ctl`].
//!
//! Both are undone once no key has been pressed for
//! [`TypingConfig::reactivation_delay_ms`].

use evdev::{EventSummary, KeyCode};
use rust_i18n::t;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::{mpsc, watch};

use crate::services::config::AppConfig;
use crate::services::evdev_runner::{find_keyboards, open_event_stream};
use crate::services::touchpad_ctl;

/// Runtime configuration of the watcher, mirrored from the active profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypingConfig {
    /// Ignore touchpad edge gestures while typing.
    pub disable_gestures: bool,
    /// Turn the whole touchpad off while typing.
    pub disable_touchpad: bool,
    /// Idle time after the last keystroke before everything is restored.
    pub reactivation_delay_ms: u32,
}

impl TypingConfig {
    /// `true` when at least one of the two actions is switched on.
    pub fn any_enabled(&self) -> bool {
        self.disable_gestures || self.disable_touchpad
    }
}

/// Set while typing is in progress *and* gesture suppression is enabled.
static GESTURES_SUPPRESSED: AtomicBool = AtomicBool::new(false);

/// Whether [`crate::services::edge_gestures`] should currently ignore touches.
pub fn gestures_suppressed() -> bool {
    GESTURES_SUPPRESSED.load(Ordering::Relaxed)
}

/// Keys that count as typing. Limited to the main keyboard block
/// (`KEY_ESC`..`KEY_COMPOSE`) so that media keys, mouse buttons and the ASUS
/// hotkeys never trigger palm rejection. Modifiers are excluded as well:
/// holding Ctrl or Shift while clicking is a pointer gesture, not typing.
fn is_typing_key(key: KeyCode) -> bool {
    const MODIFIERS: &[KeyCode] = &[
        KeyCode::KEY_LEFTSHIFT,
        KeyCode::KEY_RIGHTSHIFT,
        KeyCode::KEY_LEFTCTRL,
        KeyCode::KEY_RIGHTCTRL,
        KeyCode::KEY_LEFTALT,
        KeyCode::KEY_RIGHTALT,
        KeyCode::KEY_LEFTMETA,
        KeyCode::KEY_RIGHTMETA,
    ];
    (KeyCode::KEY_ESC.code()..=KeyCode::KEY_COMPOSE.code()).contains(&key.code())
        && !MODIFIERS.contains(&key)
}

/// Applies or reverts the touchpad disable, keeping the user's manual switch
/// authoritative: the touchpad is only turned off when the profile says it
/// should be on, and only turned back on when we were the one that turned it off.
async fn set_touchpad(active: bool) {
    if !AppConfig::load().active_profile().touchpad_active {
        return;
    }
    if let Err(e) = touchpad_ctl::set_enabled(active).await {
        tracing::warn!("{}", e);
    }
}

/// Main loop of the watcher. Runs until `cfg_rx`'s sender is dropped, which is
/// how the UI component stops it once both options are switched off.
pub async fn run(mut cfg_rx: watch::Receiver<TypingConfig>) {
    let keyboards = find_keyboards();
    if keyboards.is_empty() {
        tracing::warn!("{}", t!("error_typing_no_keyboard"));
        return;
    }

    // One reader task per keyboard, all funnelled into a single channel. The
    // payload is irrelevant - only the timing of a keystroke matters.
    let (key_tx, mut key_rx) = mpsc::channel::<()>(64);
    for device in keyboards {
        let Some(mut stream) = open_event_stream(device) else {
            continue;
        };
        let key_tx = key_tx.clone();
        tokio::spawn(async move {
            loop {
                let event = match stream.next_event().await {
                    Ok(ev) => ev,
                    Err(e) => {
                        tracing::warn!("{}", t!("error_event_read", error = e.to_string()));
                        break;
                    }
                };
                if let EventSummary::Key(_, key, value) = event.destructure()
                    && value != 0
                    && is_typing_key(key)
                    && key_tx.send(()).await.is_err()
                {
                    break;
                }
            }
        });
    }
    drop(key_tx);

    let mut typing = false;
    let mut touchpad_disabled_by_us = false;

    loop {
        let cfg = *cfg_rx.borrow();

        if !typing {
            tokio::select! {
                changed = cfg_rx.changed() => {
                    if changed.is_err() {
                        break;
                    }
                }
                key = key_rx.recv() => {
                    if key.is_none() {
                        break;
                    }
                    if !cfg.any_enabled() {
                        continue;
                    }
                    typing = true;
                    GESTURES_SUPPRESSED.store(cfg.disable_gestures, Ordering::Relaxed);
                    if cfg.disable_touchpad {
                        touchpad_disabled_by_us = true;
                        set_touchpad(false).await;
                    }
                }
            }
            continue;
        }

        let mut stop = false;
        let restore = tokio::select! {
            changed = cfg_rx.changed() => {
                // Options turned off mid-typing: revert immediately.
                stop = changed.is_err();
                stop || !cfg_rx.borrow().any_enabled()
            }
            key = key_rx.recv() => {
                // Still typing - the idle timer restarts on the next iteration.
                stop = key.is_none();
                stop
            }
            _ = tokio::time::sleep(Duration::from_millis(cfg.reactivation_delay_ms as u64)) => true,
        };

        if restore {
            typing = false;
            GESTURES_SUPPRESSED.store(false, Ordering::Relaxed);
            if touchpad_disabled_by_us {
                touchpad_disabled_by_us = false;
                set_touchpad(true).await;
            }
        }
        if stop {
            break;
        }
    }

    // Shutting down (both options disabled or the app is closing): never leave
    // the touchpad off behind us.
    GESTURES_SUPPRESSED.store(false, Ordering::Relaxed);
    if touchpad_disabled_by_us {
        set_touchpad(true).await;
    }
}
