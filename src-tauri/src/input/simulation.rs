/// Fix #2: Uses Coordinate::Rel for mouse movement (relative, resolution-independent)
/// Fix #10: Extended key mappings for punctuation and symbol keys
use anyhow::Result;
use enigo::{Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};

use crate::core::protocol::{
    KeyCode, KeyPayload, MouseButtonPayload, MouseMovePayload, PacketHeader,
    FLAG_PRESS, FLAG_RELATIVE, PKT_KEY, PKT_MOUSE_BUTTON, PKT_MOUSE_MOVE,
};

pub struct InputSimulator {
    enigo: Enigo,
}

impl InputSimulator {
    pub fn new() -> Result<Self> {
        let enigo = Enigo::new(&Settings::default())?;
        Ok(Self { enigo })
    }

    /// Simulate a mouse move.
    /// Fix #2: Always relative — no screen resolution mismatch.
    pub fn simulate_mouse_move(&mut self, payload: &MouseMovePayload, _relative: bool) -> Result<()> {
        // Ignore the relative flag from the packet — always use Rel on the client.
        // The server capture layer now always sends relative deltas.
        self.enigo.move_mouse(payload.x, payload.y, Coordinate::Rel)?;
        Ok(())
    }

    pub fn simulate_mouse_button(&mut self, payload: &MouseButtonPayload, press: bool) -> Result<()> {
        let button = match payload.button {
            0 => Button::Left,
            1 => Button::Right,
            2 => Button::Middle,
            3 => { self.enigo.scroll(3,  enigo::Axis::Vertical)?; return Ok(()); }
            4 => { self.enigo.scroll(-3, enigo::Axis::Vertical)?; return Ok(()); }
            _ => return Ok(()),
        };
        let dir = if press { Direction::Press } else { Direction::Release };
        self.enigo.button(button, dir)?;
        Ok(())
    }

    pub fn simulate_key(&mut self, payload: &KeyPayload, press: bool) -> Result<()> {
        if let Some(k) = keycode_to_enigo(KeyCode::from_u16(payload.keycode)) {
            let dir = if press { Direction::Press } else { Direction::Release };
            self.enigo.key(k, dir)?;
        }
        Ok(())
    }

    /// Dispatch a decoded plaintext packet to the appropriate simulator method.
    pub fn dispatch(&mut self, header: &PacketHeader, payload_bytes: &[u8]) -> Result<()> {
        let press    = (header.flags & FLAG_PRESS)    != 0;
        let relative = (header.flags & FLAG_RELATIVE) != 0;

        match header.packet_type {
            PKT_MOUSE_MOVE => {
                let p = MouseMovePayload::from_bytes(payload_bytes)?;
                self.simulate_mouse_move(&p, relative)?;
            }
            PKT_MOUSE_BUTTON => {
                let p = MouseButtonPayload::from_bytes(payload_bytes)?;
                self.simulate_mouse_button(&p, press)?;
            }
            PKT_KEY => {
                let p = KeyPayload::from_bytes(payload_bytes)?;
                self.simulate_key(&p, press)?;
            }
            _ => {}
        }
        Ok(())
    }
}

/// Fix #10 — Extended key mapping including punctuation and symbol keys
fn keycode_to_enigo(kc: KeyCode) -> Option<Key> {
    use KeyCode::*;
    match kc {
        // Letters
        A => Some(Key::Unicode('a')), B => Some(Key::Unicode('b')),
        C => Some(Key::Unicode('c')), D => Some(Key::Unicode('d')),
        E => Some(Key::Unicode('e')), F => Some(Key::Unicode('f')),
        G => Some(Key::Unicode('g')), H => Some(Key::Unicode('h')),
        I => Some(Key::Unicode('i')), J => Some(Key::Unicode('j')),
        K => Some(Key::Unicode('k')), L => Some(Key::Unicode('l')),
        M => Some(Key::Unicode('m')), N => Some(Key::Unicode('n')),
        O => Some(Key::Unicode('o')), P => Some(Key::Unicode('p')),
        Q => Some(Key::Unicode('q')), R => Some(Key::Unicode('r')),
        S => Some(Key::Unicode('s')), T => Some(Key::Unicode('t')),
        U => Some(Key::Unicode('u')), V => Some(Key::Unicode('v')),
        W => Some(Key::Unicode('w')), X => Some(Key::Unicode('x')),
        Y => Some(Key::Unicode('y')), Z => Some(Key::Unicode('z')),
        // Numbers
        Num0 => Some(Key::Unicode('0')), Num1 => Some(Key::Unicode('1')),
        Num2 => Some(Key::Unicode('2')), Num3 => Some(Key::Unicode('3')),
        Num4 => Some(Key::Unicode('4')), Num5 => Some(Key::Unicode('5')),
        Num6 => Some(Key::Unicode('6')), Num7 => Some(Key::Unicode('7')),
        Num8 => Some(Key::Unicode('8')), Num9 => Some(Key::Unicode('9')),
        // Punctuation / symbols (Fix #10)
        Minus        => Some(Key::Unicode('-')),
        Equal        => Some(Key::Unicode('=')),
        LeftBracket  => Some(Key::Unicode('[')),
        RightBracket => Some(Key::Unicode(']')),
        // Navigation & editing
        Return    => Some(Key::Return),
        Escape    => Some(Key::Escape),
        Backspace => Some(Key::Backspace),
        Tab       => Some(Key::Tab),
        Space     => Some(Key::Space),
        Delete    => Some(Key::Delete),
        Home      => Some(Key::Home),
        End       => Some(Key::End),
        PageUp    => Some(Key::PageUp),
        PageDown  => Some(Key::PageDown),
        ArrowUp   => Some(Key::UpArrow),
        ArrowDown => Some(Key::DownArrow),
        ArrowLeft => Some(Key::LeftArrow),
        ArrowRight=> Some(Key::RightArrow),
        // Function keys
        F1  => Some(Key::F1),  F2  => Some(Key::F2),  F3  => Some(Key::F3),
        F4  => Some(Key::F4),  F5  => Some(Key::F5),  F6  => Some(Key::F6),
        F7  => Some(Key::F7),  F8  => Some(Key::F8),  F9  => Some(Key::F9),
        F10 => Some(Key::F10), F11 => Some(Key::F11), F12 => Some(Key::F12),
        // Modifiers
        // Note: enigo 0.2 does not expose Key::AltGr/RControl/RShift separately,
        // so left and right variants map to the same logical key. AltGr distinction
        // requires upgrading to a newer enigo release.
        LeftCtrl  | RightCtrl  => Some(Key::Control),
        LeftShift | RightShift => Some(Key::Shift),
        LeftAlt   | RightAlt   => Some(Key::Alt),
        LeftMeta  | RightMeta  => Some(Key::Meta),
        Unknown => None,
    }
}
