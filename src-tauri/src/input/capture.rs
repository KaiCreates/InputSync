use anyhow::Result;
use rdev::{Button as RdevButton, Event, EventType, Key as RdevKey};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

use crate::core::protocol::{InputPacket, KeyCode};
use crate::state::ServerConfig;

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

/// Detect primary screen size; returns (width, height).
/// Falls back to a large default if detection fails.
fn get_screen_size() -> (u32, u32) {
    // rdev::display_size() returns (width, height) on Linux/macOS/Windows
    match rdev::display_size() {
        Ok((w, h)) => (w as u32, h as u32),
        Err(_) => (1920, 1080),
    }
}

fn is_dead_corner(
    x: f64,
    y: f64,
    sw: u32,
    sh: u32,
    corners: &crate::state::DeadCorners,
) -> bool {
    let cs = corners.size_px as f64;
    if corners.top_left     && x < cs              && y < cs              { return true; }
    if corners.top_right    && x > (sw as f64 - cs) && y < cs              { return true; }
    if corners.bottom_left  && x < cs              && y > (sh as f64 - cs) { return true; }
    if corners.bottom_right && x > (sw as f64 - cs) && y > (sh as f64 - cs) { return true; }
    false
}

fn is_in_dead_zone(x: f64, y: f64, sw: u32, sh: u32, zones: &[crate::state::DeadZone]) -> bool {
    let (fw, fh) = (sw as f64, sh as f64);
    for dz in zones {
        let zx = dz.x_frac as f64 * fw;
        let zy = dz.y_frac as f64 * fh;
        let zw = dz.w_frac as f64 * fw;
        let zh = dz.h_frac as f64 * fh;
        if x >= zx && x <= zx + zw && y >= zy && y <= zy + zh {
            return true;
        }
    }
    false
}

fn check_edge_trigger(
    x: f64,
    y: f64,
    config: &ServerConfig,
    screen: (u32, u32),
    forwarding: &AtomicBool,
) {
    let (sw, sh) = screen;
    let px = config.edge_triggers.trigger_px as f64;

    if is_dead_corner(x, y, sw, sh, &config.dead_corners) {
        return;
    }
    if is_in_dead_zone(x, y, sw, sh, &config.dead_zones) {
        return;
    }

    let triggers = &config.edge_triggers;
    if triggers.left   && x <= px                    { forwarding.store(true, Ordering::Relaxed); }
    if triggers.right  && x >= (sw as f64 - px)      { forwarding.store(true, Ordering::Relaxed); }
    if triggers.top    && y <= px                    { forwarding.store(true, Ordering::Relaxed); }
    if triggers.bottom && y >= (sh as f64 - px)      { forwarding.store(true, Ordering::Relaxed); }
}

pub fn start_capture(
    event_tx: tokio::sync::mpsc::Sender<InputPacket>,
    forwarding: Arc<AtomicBool>,
    config: ServerConfig,
) -> Result<CaptureHandle> {
    let (stop_tx, stop_rx) = mpsc::sync_channel::<()>(1);

    std::thread::Builder::new()
        .name("inputsync-capture".into())
        .spawn(move || {
            let screen_size = get_screen_size();
            tracing::info!("Capture thread: screen size {:?}", screen_size);

            let mut last_x: f64 = 0.0;
            let mut last_y: f64 = 0.0;
            let mut first_move = true;
            let mut local_seq: u32 = 0;
            let mut was_forwarding = false;

            let callback = move |event: Event| {
                if stop_rx.try_recv().is_ok() {
                    return;
                }

                // ScrollLock toggles forwarding off (return to server)
                if matches!(&event.event_type, EventType::KeyPress(RdevKey::ScrollLock)) {
                    let was = forwarding.fetch_xor(true, Ordering::Relaxed);
                    tracing::info!("Input forwarding toggled via ScrollLock → {}", !was);
                    return;
                }

                // Check edge triggers on every mouse move (even when not forwarding)
                if let EventType::MouseMove { x, y } = &event.event_type {
                    check_edge_trigger(*x, *y, &config, screen_size, &forwarding);
                }

                let is_forwarding = forwarding.load(Ordering::Relaxed);

                // Reset mouse baseline and signal screen entry/exit
                if is_forwarding && !was_forwarding {
                    first_move = true;
                    let _ = event_tx.try_send(InputPacket::enter_screen());
                } else if !is_forwarding && was_forwarding {
                    let _ = event_tx.try_send(InputPacket::exit_screen());
                }
                was_forwarding = is_forwarding;

                if !is_forwarding {
                    return;
                }

                let packet: Option<InputPacket> = match &event.event_type {
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

                // Only advance seq when a packet is actually emitted.
                // Advancing it for dropped events (first_move, zero delta, zero wheel)
                // would create spurious counter gaps on the receiver side.
                if let Some(pkt) = packet {
                    local_seq = local_seq.wrapping_add(1);
                    if event_tx.try_send(pkt).is_err() {
                        tracing::trace!("Input queue full — packet dropped");
                    }
                }
            };

            if let Err(e) = rdev::listen(callback) {
                tracing::error!("rdev capture error: {:?}", e);
            }
        })
        .expect("Failed to spawn capture thread");

    Ok(CaptureHandle { stop_tx })
}
