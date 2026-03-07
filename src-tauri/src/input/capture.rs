use anyhow::Result;
use rdev::{Event, EventType, Key as RdevKey, Button as RdevButton};
use std::sync::mpsc;

use crate::core::protocol::{InputPacket, KeyCode};

#[allow(unused_variables)]

/// Converts an rdev key to InputSync KeyCode
fn rdev_key_to_keycode(key: RdevKey) -> u16 {
    use RdevKey::*;
    let kc = match key {
        KeyA => KeyCode::A, KeyB => KeyCode::B, KeyC => KeyCode::C,
        KeyD => KeyCode::D, KeyE => KeyCode::E, KeyF => KeyCode::F,
        KeyG => KeyCode::G, KeyH => KeyCode::H, KeyI => KeyCode::I,
        KeyJ => KeyCode::J, KeyK => KeyCode::K, KeyL => KeyCode::L,
        KeyM => KeyCode::M, KeyN => KeyCode::N, KeyO => KeyCode::O,
        KeyP => KeyCode::P, KeyQ => KeyCode::Q, KeyR => KeyCode::R,
        KeyS => KeyCode::S, KeyT => KeyCode::T, KeyU => KeyCode::U,
        KeyV => KeyCode::V, KeyW => KeyCode::W, KeyX => KeyCode::X,
        KeyY => KeyCode::Y, KeyZ => KeyCode::Z,
        Num0 => KeyCode::Num0, Num1 => KeyCode::Num1, Num2 => KeyCode::Num2,
        Num3 => KeyCode::Num3, Num4 => KeyCode::Num4, Num5 => KeyCode::Num5,
        Num6 => KeyCode::Num6, Num7 => KeyCode::Num7, Num8 => KeyCode::Num8,
        Num9 => KeyCode::Num9,
        Return => KeyCode::Return, Escape => KeyCode::Escape,
        Backspace => KeyCode::Backspace, Tab => KeyCode::Tab,
        Space => KeyCode::Space,
        F1 => KeyCode::F1, F2 => KeyCode::F2, F3 => KeyCode::F3,
        F4 => KeyCode::F4, F5 => KeyCode::F5, F6 => KeyCode::F6,
        F7 => KeyCode::F7, F8 => KeyCode::F8, F9 => KeyCode::F9,
        F10 => KeyCode::F10, F11 => KeyCode::F11, F12 => KeyCode::F12,
        Home => KeyCode::Home, End => KeyCode::End,
        PageUp => KeyCode::PageUp, PageDown => KeyCode::PageDown,
        Delete => KeyCode::Delete,
        UpArrow => KeyCode::ArrowUp, DownArrow => KeyCode::ArrowDown,
        LeftArrow => KeyCode::ArrowLeft, RightArrow => KeyCode::ArrowRight,
        ControlLeft => KeyCode::LeftCtrl, ControlRight => KeyCode::RightCtrl,
        ShiftLeft => KeyCode::LeftShift, ShiftRight => KeyCode::RightShift,
        Alt => KeyCode::LeftAlt, AltGr => KeyCode::RightAlt,
        MetaLeft => KeyCode::LeftMeta, MetaRight => KeyCode::RightMeta,
        _ => KeyCode::Unknown,
    };
    kc as u16
}

/// Converts an rdev mouse button to InputSync button code
fn rdev_button_to_code(button: RdevButton) -> u8 {
    match button {
        RdevButton::Left => 0,
        RdevButton::Right => 1,
        RdevButton::Middle => 2,
        RdevButton::Unknown(_) => 255,
    }
}

/// Input capture handle — drop to stop capture
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
/// Events are sent to `event_tx` as `InputPacket` values.
/// Returns a `CaptureHandle`; dropping it stops capture.
pub fn start_capture(
    event_tx: tokio::sync::mpsc::UnboundedSender<InputPacket>,
) -> Result<CaptureHandle> {
    let (stop_tx, stop_rx) = mpsc::sync_channel::<()>(1);

    // rdev::listen must run on a dedicated OS thread (it blocks)
    std::thread::spawn(move || {
        let tx = event_tx.clone();
        let mut local_seq: u32 = 0;

        let callback = move |event: Event| {
            if stop_rx.try_recv().is_ok() {
                return;
            }
            local_seq = local_seq.wrapping_add(1);
            let packet = event_to_packet(&event, local_seq);
            if let Some(pkt) = packet {
                let _ = tx.send(pkt);
            }
        };

        if let Err(e) = rdev::listen(callback) {
            log::error!("rdev capture error: {:?}", e);
        }
    });

    Ok(CaptureHandle { stop_tx })
}

fn event_to_packet(event: &Event, seq: u32) -> Option<InputPacket> {
    match &event.event_type {
        EventType::MouseMove { x, y } => {
            Some(InputPacket::mouse_move(*x as i32, *y as i32, seq, false))
        }
        EventType::ButtonPress(btn) => {
            let code = rdev_button_to_code(*btn);
            Some(InputPacket::mouse_button(code, seq, true))
        }
        EventType::ButtonRelease(btn) => {
            let code = rdev_button_to_code(*btn);
            Some(InputPacket::mouse_button(code, seq, false))
        }
        EventType::KeyPress(key) => {
            let keycode = rdev_key_to_keycode(*key);
            Some(InputPacket::key_event(keycode, 0, seq, true))
        }
        EventType::KeyRelease(key) => {
            let keycode = rdev_key_to_keycode(*key);
            Some(InputPacket::key_event(keycode, 0, seq, false))
        }
        EventType::Wheel { delta_x: _, delta_y } => {
            if *delta_y > 0 {
                Some(InputPacket::mouse_button(3, seq, true)) // scroll up
            } else if *delta_y < 0 {
                Some(InputPacket::mouse_button(4, seq, true)) // scroll down
            } else {
                None
            }
        }
    }
}
