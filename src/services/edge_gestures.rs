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

use evdev::{AbsoluteAxisCode, EventSummary, KeyCode};
use rust_i18n::t;
use tokio::sync::watch;

use crate::services::commands::is_kde_desktop;
use crate::services::evdev_runner::{find_touchpad, open_event_stream, touchpad_abs_bounds};
use crate::services::kde_brightness;

/// Fraction of touchpad width/height that counts as an edge zone (4%)
const EDGE_PERCENT: f64 = 0.04;
/// Minimum movement in touchpad units required to trigger an action
const STEP_THRESHOLD: i32 = 300;

/// Tracks the current touch gesture as events arrive from the input device.
enum GestureState {
    /// No finger is currently touching the pad.
    Idle,
    /// Finger down - waiting for the first X and Y position to classify the gesture.
    Classifying { x: Option<i32>, y: Option<i32> },
    /// Touch started in the left edge zone; tracks volume via vertical movement.
    LeftEdge { last_y: i32 },
    /// Touch started in the right edge zone; tracks brightness via vertical movement.
    RightEdge { last_y: i32 },
    /// Touch started in the top edge zone; triggers media prev/next on horizontal movement.
    TopEdge { start_x: i32, done: bool },
    /// Touch started outside any edge zone - no action will be taken.
    Other,
}

/// Transitions a [`GestureState::Classifying`] state to the appropriate edge state once both
/// X and Y coordinates have been received for the initial touch position.
fn try_classify(state: &mut GestureState, left: i32, right: i32, top: i32) {
    if let GestureState::Classifying {
        x: Some(x),
        y: Some(y),
    } = *state
    {
        *state = if x < left {
            GestureState::LeftEdge { last_y: y }
        } else if x > right {
            GestureState::RightEdge { last_y: y }
        } else if y < top {
            GestureState::TopEdge {
                start_x: x,
                done: false,
            }
        } else {
            GestureState::Other
        };
    }
}

/// Spawns an external program asynchronously to perform a gesture action.
///
/// Failures are logged as warnings but do not propagate - this is a fire-and-forget call.
async fn run_action(program: &str, args: &[&str]) {
    let result = tokio::process::Command::new(program)
        .args(args)
        .status()
        .await;
    if let Err(e) = result {
        tracing::warn!(
            "{}",
            t!(
                "error_gesture_action",
                program = program,
                error = e.to_string()
            )
        );
    }
}

/// Main event loop for touchpad edge gesture detection.
///
/// Finds the touchpad device, reads its absolute-axis bounds, then processes `evdev` events
/// to recognise three gesture zones:
/// - **Left edge** (vertical swipe) → volume up/down via `pactl`
/// - **Right edge** (vertical swipe) → brightness up/down via `brightnessctl`
/// - **Top edge** (horizontal swipe) → media previous/next via `playerctl`
///
/// The loop exits cleanly when `shutdown` fires (the sender's value changes).
pub async fn run_gesture_loop(mut shutdown: watch::Receiver<bool>) {
    let device = match find_touchpad() {
        Some(d) => d,
        None => {
            tracing::warn!("{}", t!("error_no_touchpad"));
            return;
        }
    };

    let (x_max, y_max) = match touchpad_abs_bounds(&device) {
        Some(b) => b,
        None => return,
    };
    let left_bound = (x_max as f64 * EDGE_PERCENT) as i32;
    let right_bound = (x_max as f64 * (1.0 - EDGE_PERCENT)) as i32;
    let top_bound = (y_max as f64 * EDGE_PERCENT) as i32;

    let Some(mut stream) = open_event_stream(device) else {
        return;
    };

    let mut state = GestureState::Idle;

    loop {
        let event = tokio::select! {
            _ = shutdown.changed() => break,
            result = stream.next_event() => {
                match result {
                    Ok(ev) => ev,
                    Err(e) => {
                        tracing::warn!("{}", t!("error_event_read", error = e.to_string()));
                        break;
                    }
                }
            }
        };

        match event.destructure() {
            EventSummary::Key(_, KeyCode::BTN_TOUCH, value) => {
                if value == 1 {
                    // Palm rejection: a touch that starts while the user is
                    // typing never becomes a gesture.
                    state = if crate::services::typing_watch::gestures_suppressed() {
                        GestureState::Other
                    } else {
                        GestureState::Classifying { x: None, y: None }
                    };
                } else {
                    state = GestureState::Idle;
                }
            }
            EventSummary::AbsoluteAxis(
                _,
                AbsoluteAxisCode::ABS_X | AbsoluteAxisCode::ABS_MT_POSITION_X,
                value,
            ) => {
                if let GestureState::Classifying { x, .. } = &mut state {
                    *x = Some(value);
                    try_classify(&mut state, left_bound, right_bound, top_bound);
                } else if let GestureState::TopEdge { start_x, done } = &mut state {
                    if !*done {
                        let dx = value - *start_x;
                        if dx.abs() >= STEP_THRESHOLD {
                            *done = true;
                            if dx < 0 {
                                run_action("playerctl", &["previous"]).await;
                            } else {
                                run_action("playerctl", &["next"]).await;
                            }
                        }
                    }
                }
            }
            EventSummary::AbsoluteAxis(
                _,
                AbsoluteAxisCode::ABS_Y | AbsoluteAxisCode::ABS_MT_POSITION_Y,
                value,
            ) => {
                if let GestureState::Classifying { y, .. } = &mut state {
                    *y = Some(value);
                    try_classify(&mut state, left_bound, right_bound, top_bound);
                } else {
                    match &mut state {
                        GestureState::LeftEdge { last_y } => {
                            let dy = value - *last_y;
                            if dy.abs() >= STEP_THRESHOLD {
                                *last_y = value;
                                run_action(
                                    "pactl",
                                    &["set-sink-mute", "@DEFAULT_SINK@", "0"],
                                )
                                .await;
                                if dy < 0 {
                                    run_action(
                                        "pactl",
                                        &["set-sink-volume", "@DEFAULT_SINK@", "+5%"],
                                    )
                                    .await;
                                } else {
                                    run_action(
                                        "pactl",
                                        &["set-sink-volume", "@DEFAULT_SINK@", "-5%"],
                                    )
                                    .await;
                                }
                            }
                        }
                        GestureState::RightEdge { last_y } => {
                            let dy = value - *last_y;
                            if dy.abs() >= STEP_THRESHOLD {
                                *last_y = value;
                                let delta = if dy < 0 { 5 } else { -5 };
                                let arg = if delta > 0 { "5%+" } else { "5%-" };
                                if is_kde_desktop() {
                                    if let Err(e) =
                                        kde_brightness::adjust_brightness_relative(delta).await
                                    {
                                        tracing::warn!(
                                            "PowerDevil setBrightness failed, falling back to brightnessctl: {e}"
                                        );
                                        run_action("brightnessctl", &["set", arg]).await;
                                    }
                                } else {
                                    run_action("brightnessctl", &["set", arg]).await;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            EventSummary::Key(
                _,
                KeyCode::BTN_TOOL_DOUBLETAP | KeyCode::BTN_TOOL_TRIPLETAP,
                1,
            ) => {
                state = GestureState::Other;
            }
            EventSummary::AbsoluteAxis(_, AbsoluteAxisCode::ABS_MT_SLOT, value) if value > 0 => {
                state = GestureState::Other;
            }
            _ => {}
        }
    }
}
