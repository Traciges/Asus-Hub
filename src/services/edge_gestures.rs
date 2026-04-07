use evdev::{AbsoluteAxisCode, Device, EventSummary, KeyCode};
use rust_i18n::t;
use tokio::sync::watch;

/// Fraction of touchpad width/height that counts as an edge zone (4%)
const EDGE_PERCENT: f64 = 0.04;
/// Minimum movement in touchpad units required to trigger an action
const STEP_THRESHOLD: i32 = 300;

/// Tracks the current touch gesture as events arrive from the input device.
enum GestureState {
    /// No finger is currently touching the pad.
    Idle,
    /// Finger down — waiting for the first X and Y position to classify the gesture.
    Classifying { x: Option<i32>, y: Option<i32> },
    /// Touch started in the left edge zone; tracks volume via vertical movement.
    LeftEdge { last_y: i32 },
    /// Touch started in the right edge zone; tracks brightness via vertical movement.
    RightEdge { last_y: i32 },
    /// Touch started in the top edge zone; triggers media prev/next on horizontal movement.
    TopEdge { start_x: i32, done: bool },
    /// Touch started outside any edge zone — no action will be taken.
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

/// Scans `/dev/input/` for the first device whose name contains "touchpad"
/// and that reports both `ABS_X` and `ABS_Y` absolute axes.
fn find_touchpad() -> Option<Device> {
    for (_, device) in evdev::enumerate() {
        let name = device.name().unwrap_or_default().to_lowercase();
        if !name.contains("touchpad") {
            continue;
        }
        let supported = device.supported_absolute_axes();
        if let Some(axes) = supported {
            if axes.contains(AbsoluteAxisCode::ABS_X) && axes.contains(AbsoluteAxisCode::ABS_Y) {
                return Some(device);
            }
        }
    }
    None
}

/// Spawns an external program asynchronously to perform a gesture action.
///
/// Failures are logged as warnings but do not propagate — this is a fire-and-forget call.
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

    let abs_state = match device.get_abs_state() {
        Ok(states) => states,
        Err(e) => {
            tracing::warn!("{}", t!("error_abs_info", error = e.to_string()));
            return;
        }
    };
    let x_info = abs_state[AbsoluteAxisCode::ABS_X.0 as usize];
    let y_info = abs_state[AbsoluteAxisCode::ABS_Y.0 as usize];

    let x_max = x_info.maximum;
    let y_max = y_info.maximum;
    let left_bound = (x_max as f64 * EDGE_PERCENT) as i32;
    let right_bound = (x_max as f64 * (1.0 - EDGE_PERCENT)) as i32;
    let top_bound = (y_max as f64 * EDGE_PERCENT) as i32;

    let mut stream = match device.into_event_stream() {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Failed to open event stream: {e}");
            return;
        }
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
                    state = GestureState::Classifying { x: None, y: None };
                } else {
                    state = GestureState::Idle;
                }
            }
            EventSummary::AbsoluteAxis(_, AbsoluteAxisCode::ABS_X, value) => {
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
            EventSummary::AbsoluteAxis(_, AbsoluteAxisCode::ABS_Y, value) => {
                if let GestureState::Classifying { y, .. } = &mut state {
                    *y = Some(value);
                    try_classify(&mut state, left_bound, right_bound, top_bound);
                } else {
                    match &mut state {
                        GestureState::LeftEdge { last_y } => {
                            let dy = value - *last_y;
                            if dy.abs() >= STEP_THRESHOLD {
                                *last_y = value;
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
                                if dy < 0 {
                                    run_action("brightnessctl", &["set", "5%+"]).await;
                                } else {
                                    run_action("brightnessctl", &["set", "5%-"]).await;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}
