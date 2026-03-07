use anyhow::Result;
use enigo::{
    Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings,
};

use crate::core::protocol::{
    KeyCode, MouseButtonPayload, MouseMovePayload, PacketHeader, KeyPayload,
    FLAG_PRESS, PKT_KEY, PKT_MOUSE_BUTTON, PKT_MOUSE_MOVE,
};

pub struct InputSimulator {
    enigo: Enigo,
}

impl InputSimulator {
    pub fn new() -> Result<Self> {
        let enigo = Enigo::new(&Settings::default())?;
        Ok(Self { enigo })
    }

    /// Simulate a mouse move event
    pub fn simulate_mouse_move(&mut self, payload: &MouseMovePayload, relative: bool) -> Result<()> {
        if relative {
            self.enigo
                .move_mouse(payload.x, payload.y, Coordinate::Rel)?;
        } else {
            self.enigo
                .move_mouse(payload.x, payload.y, Coordinate::Abs)?;
        }
        Ok(())
    }

    /// Simulate a mouse button event
    pub fn simulate_mouse_button(
        &mut self,
        payload: &MouseButtonPayload,
        press: bool,
    ) -> Result<()> {
        let button = match payload.button {
            0 => Button::Left,
            1 => Button::Right,
            2 => Button::Middle,
            3 => {
                self.enigo.scroll(3, enigo::Axis::Vertical)?;
                return Ok(());
            }
            4 => {
                self.enigo.scroll(-3, enigo::Axis::Vertical)?;
                return Ok(());
            }
            _ => return Ok(()),
        };
        let direction = if press { Direction::Press } else { Direction::Release };
        self.enigo.button(button, direction)?;
        Ok(())
    }

    /// Simulate a keyboard event
    pub fn simulate_key(&mut self, payload: &KeyPayload, press: bool) -> Result<()> {
        let key = keycode_to_enigo(KeyCode::from_u16(payload.keycode));
        if let Some(k) = key {
            let direction = if press { Direction::Press } else { Direction::Release };
            self.enigo.key(k, direction)?;
        }
        Ok(())
    }

    /// Dispatch a raw packet to the appropriate simulation method
    pub fn dispatch(&mut self, header: &PacketHeader, payload_bytes: &[u8]) -> Result<()> {
        let press = (header.flags & FLAG_PRESS) != 0;
        let relative = (header.flags & crate::core::protocol::FLAG_RELATIVE) != 0;

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
            _ => {} // Ignore unknown packet types
        }
        Ok(())
    }
}

/// Map InputSync KeyCode to enigo Key
fn keycode_to_enigo(kc: KeyCode) -> Option<Key> {
    use KeyCode::*;
    match kc {
        A => Some(Key::Unicode('a')),
        B => Some(Key::Unicode('b')),
        C => Some(Key::Unicode('c')),
        D => Some(Key::Unicode('d')),
        E => Some(Key::Unicode('e')),
        F => Some(Key::Unicode('f')),
        G => Some(Key::Unicode('g')),
        H => Some(Key::Unicode('h')),
        I => Some(Key::Unicode('i')),
        J => Some(Key::Unicode('j')),
        K => Some(Key::Unicode('k')),
        L => Some(Key::Unicode('l')),
        M => Some(Key::Unicode('m')),
        N => Some(Key::Unicode('n')),
        O => Some(Key::Unicode('o')),
        P => Some(Key::Unicode('p')),
        Q => Some(Key::Unicode('q')),
        R => Some(Key::Unicode('r')),
        S => Some(Key::Unicode('s')),
        T => Some(Key::Unicode('t')),
        U => Some(Key::Unicode('u')),
        V => Some(Key::Unicode('v')),
        W => Some(Key::Unicode('w')),
        X => Some(Key::Unicode('x')),
        Y => Some(Key::Unicode('y')),
        Z => Some(Key::Unicode('z')),
        Num0 => Some(Key::Unicode('0')),
        Num1 => Some(Key::Unicode('1')),
        Num2 => Some(Key::Unicode('2')),
        Num3 => Some(Key::Unicode('3')),
        Num4 => Some(Key::Unicode('4')),
        Num5 => Some(Key::Unicode('5')),
        Num6 => Some(Key::Unicode('6')),
        Num7 => Some(Key::Unicode('7')),
        Num8 => Some(Key::Unicode('8')),
        Num9 => Some(Key::Unicode('9')),
        Return => Some(Key::Return),
        Escape => Some(Key::Escape),
        Backspace => Some(Key::Backspace),
        Tab => Some(Key::Tab),
        Space => Some(Key::Space),
        F1 => Some(Key::F1),
        F2 => Some(Key::F2),
        F3 => Some(Key::F3),
        F4 => Some(Key::F4),
        F5 => Some(Key::F5),
        F6 => Some(Key::F6),
        F7 => Some(Key::F7),
        F8 => Some(Key::F8),
        F9 => Some(Key::F9),
        F10 => Some(Key::F10),
        F11 => Some(Key::F11),
        F12 => Some(Key::F12),
        Home => Some(Key::Home),
        End => Some(Key::End),
        PageUp => Some(Key::PageUp),
        PageDown => Some(Key::PageDown),
        Delete => Some(Key::Delete),
        ArrowUp => Some(Key::UpArrow),
        ArrowDown => Some(Key::DownArrow),
        ArrowLeft => Some(Key::LeftArrow),
        ArrowRight => Some(Key::RightArrow),
        LeftCtrl | RightCtrl => Some(Key::Control),
        LeftShift | RightShift => Some(Key::Shift),
        LeftAlt | RightAlt => Some(Key::Alt),
        LeftMeta | RightMeta => Some(Key::Meta),
        Unknown | Minus | Equal | LeftBracket | RightBracket => None,
    }
}
