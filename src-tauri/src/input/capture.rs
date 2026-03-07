/// Fix #2: Relative mouse movement — tracks last position, sends deltas
/// Fix #3: ScrollLock toggles forwarding (switch mechanism)
/// Fix #7: Bounded channel with try_send — drops packets rather than OOM
use anyhow::Result;
use rdev::{Button as RdevButton, Event, EventType, Key as RdevKey};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

use crate::core::protocol::{InputPacket, KeyCode};

fn rdev_key_to_keycode(key: RdevKey) -> u16 {
    use RdevKey::*;
    let kc = match key {
        KeyA => KeyCode::A,   KeyB => KeyCode::B,   KeyC => KeyCode::C,
        KeyD => KeyCode::D,   KeyE => KeyCode::E,   KeyF => KeyCode::F,
        KeyG => KeyCode::G,   KeyH => KeyCode::H,   KeyI => KeyCode::I,
        KeyJ => KeyCode::J,   KeyK => KeyCode::K,   KeyL => KeyCode::L,
        KeyM => KeyCode::M,   KeyN => KeyCode::N,   KeyO => KeyCode::O,
        KeyP => KeyCode::P,   KeyQ => KeyCode::Q,   KeyR => KeyCode::R,
        KeyS => KeyCode::S,   KeyT => KeyCode::T,   KeyU => KeyCode::U,
        KeyV => KeyCode::V,   KeyW => KeyCode::W,   KeyX => KeyCode::X,
        KeyY => KeyCode::Y,   KeyZ => KeyCode::Z,
        Num0 => KeyCode::Num0,  Num1 => KeyCode::Num1,  Num2 => KeyCode::Num2,
        Num3 => KeyCode::Num3,  Num4 => KeyCode::Num4,  Num5 => KeyCode::Num5,
        Num6 => KeyCode::Num6,  Num7 => KeyCode::Num7,  Num8 => KeyCode::Num8,
        Num9 => KeyCode::Num9,
        Return    => KeyCode::Return,
        Escape    => KeyCode::Escape,
        Backspace => KeyCode::Backspace,
        Tab       => KeyCode::Tab,
        Space     => KeyCode::Space,
        F1  => KeyCode::F1,  F2  => KeyCode::F2,  F3  => KeyCode::F3,
        F4  => KeyCode::F4,  F5  => KeyCode::F5,  F6  => KeyCode::F6,
        F7  => KeyCode::F7,  F8  => KeyCode::F8,  F9  => KeyCode::F9,
        F10 => KeyCode::F10, F11 => KeyCode::F11, F12 => KeyCode::F12,
        Home      => KeyCode::Home,     End      => KeyCode::End,
        PageUp    => KeyCode::PageUp,   PageDown => KeyCode::PageDown,
        Delete    => KeyCode::Delete,
        UpArrow   => KeyCode::ArrowUp,  DownArrow  => KeyCode::ArrowDown,
        LeftArrow => KeyCode::ArrowLeft, RightArrow => KeyCode::ArrowRight,
        ControlLeft  => KeyCode::LeftCtrl,   ControlRight => KeyCode::RightCtrl,
        ShiftLeft    => KeyCode::LeftShift,  ShiftRight   => KeyCode::RightShift,
        Alt          => KeyCode::LeftAlt,    AltGr        => KeyCode::RightAlt,
        MetaLeft     => KeyCode::LeftMeta,   MetaRight    => KeyCode::RightMeta,
        _ => KeyCode::Unknown,
    };
    kc as u16
}

fn rdev_button_to_code(button: RdevButton) -> u8 {
    match button {
        RdevButton::Left    => 0,
        RdevButton::Right   => 1,
        RdevButton::Middle  => 2,
        RdevButton::Unknown(_) => 255,
    }
}

/// Dropping this handle signals the capture thread to stop forwarding.
/// The rdev::listen thread itself cannot be cleanly killed, but it will
/// stop forwarding events and block until the process exits.
pub struct CaptureHandle {
    stop_tx: mpsc::SyncSender<()>,
}

impl CaptureHandle {
    pub fn stop(self) {
        let _ = self.stop_tx.send(());
    }
}

impl Drop for CaptureHandle {
    fn drop(&mut self) {
        let _ = self.stop_tx.try_send(());
    }
}

/// Start capturing input events.
///
/// - `event_tx`: bounded channel; packets are dropped if the consumer is slow (Fix #7)
/// - `forwarding`: AtomicBool controlled externally; when false events are captured
///   but not forwarded. ScrollLock toggles this flag (Fix #3).
pub fn start_capture(
    event_tx: tokio::sync::mpsc::Sender<InputPacket>,
    forwarding: Arc<AtomicBool>,
) -> Result<CaptureHandle> {
    let (stop_tx, stop_rx) = mpsc::sync_channel::<()>(1);

    std::thread::Builder::new()
        .name("inputsync-capture".into())
        .spawn(move || {
            // Per-iteration state — lives inside the closure, no Arc needed
            let mut last_x: f64 = 0.0;
            let mut last_y: f64 = 0.0;
            let mut first_move = true;
            let mut local_seq: u32 = 0;

            let callback = move |event: Event| {
                // Check stop signal (non-blocking)
                if stop_rx.try_recv().is_ok() {
                    return;
                }

                // Fix #3 — ScrollLock press toggles forwarding on the server
                if matches!(&event.event_type, EventType::KeyPress(RdevKey::ScrollLock)) {
                    let was = forwarding.fetch_xor(true, Ordering::Relaxed);
                    log::info!("Input forwarding toggled via ScrollLock → {}", !was);
                    return;
                }

                // Don't forward if capture is paused
                if !forwarding.load(Ordering::Relaxed) {
                    return;
                }

                local_seq = local_seq.wrapping_add(1);

                let packet: Option<InputPacket> = match &event.event_type {
                    // Fix #2 — Relative mouse movement
                    EventType::MouseMove { x, y } => {
                        if first_move {
                            last_x = *x;
                            last_y = *y;
                            first_move = false;
                            None
                        } else {
                            let dx = (*x - last_x) as i32;
                            let dy = (*y - last_y) as i32;
                            last_x = *x;
                            last_y = *y;
                            if dx == 0 && dy == 0 {
                                None
                            } else {
                                Some(InputPacket::mouse_move(dx, dy, local_seq, true))
                            }
                        }
                    }

                    EventType::ButtonPress(btn) => {
                        let code = rdev_button_to_code(*btn);
                        if code == 255 { None } else {
                            Some(InputPacket::mouse_button(code, local_seq, true))
                        }
                    }

                    EventType::ButtonRelease(btn) => {
                        let code = rdev_button_to_code(*btn);
                        if code == 255 { None } else {
                            Some(InputPacket::mouse_button(code, local_seq, false))
                        }
                    }

                    EventType::KeyPress(key) => {
                        let kc = rdev_key_to_keycode(*key);
                        if kc == KeyCode::Unknown as u16 { None } else {
                            Some(InputPacket::key_event(kc, 0, local_seq, true))
                        }
                    }

                    EventType::KeyRelease(key) => {
                        let kc = rdev_key_to_keycode(*key);
                        if kc == KeyCode::Unknown as u16 { None } else {
                            Some(InputPacket::key_event(kc, 0, local_seq, false))
                        }
                    }

                    EventType::Wheel { delta_x: _, delta_y } => {
                        if *delta_y > 0 {
                            Some(InputPacket::mouse_button(3, local_seq, true))
                        } else if *delta_y < 0 {
                            Some(InputPacket::mouse_button(4, local_seq, true))
                        } else {
                            None
                        }
                    }
                };

                if let Some(pkt) = packet {
                    // Fix #7 — Non-blocking try_send; drop on full buffer (backpressure)
                    if event_tx.try_send(pkt).is_err() {
                        log::trace!("Input queue full — packet dropped");
                    }
                }
            };

            if let Err(e) = rdev::listen(callback) {
                log::error!("rdev capture error: {:?}", e);
            }
        })
        .expect("Failed to spawn capture thread");

    Ok(CaptureHandle { stop_tx })
}
