use anyhow::{bail, Result};
use std::time::{SystemTime, UNIX_EPOCH};

// Protocol version for handshake compatibility
pub const PROTOCOL_VERSION_MAJOR: u16 = 1;
pub const PROTOCOL_VERSION_MINOR: u16 = 2;

// Packet type constants
pub const PKT_MOUSE_MOVE: u8 = 0x01;
pub const PKT_MOUSE_BUTTON: u8 = 0x02;
pub const PKT_KEY: u8 = 0x03;
pub const PKT_CLIPBOARD: u8 = 0x04;
pub const PKT_PING: u8 = 0x05;
pub const PKT_PONG: u8 = 0x06;
pub const PKT_ENTER_SCREEN: u8 = 0x07;
pub const PKT_EXIT_SCREEN: u8 = 0x08;

// Handshake message types (TCP control channel)
pub const HS_CLIENT_HELLO: u8 = 0x10;
pub const HS_SERVER_HELLO: u8 = 0x11;
pub const HS_SESSION_CODE: u8 = 0x12;
pub const HS_NONCE_EXCHANGE: u8 = 0x13;
pub const HS_ACK: u8 = 0x20;
pub const HS_REJECT: u8 = 0x21;

// Flag bits
pub const FLAG_PRESS: u8 = 0x01;
pub const FLAG_RELEASE: u8 = 0x02;
pub const FLAG_RELATIVE: u8 = 0x04;

/// Fixed 12-byte packet header
/// Layout: [type(1)][flags(1)][payload_len(2)][timestamp_us(8)]
#[derive(Debug, Clone)]
pub struct PacketHeader {
    pub packet_type: u8,
    pub flags: u8,
    pub payload_len: u16,
    pub timestamp_us: i64,
}

impl PacketHeader {
    pub fn new(packet_type: u8, flags: u8, payload_len: u16) -> Self {
        let timestamp_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_micros() as i64)
            .unwrap_or(0);
        Self {
            packet_type,
            flags,
            payload_len,
            timestamp_us,
        }
    }

    pub fn to_bytes(&self) -> [u8; 12] {
        let mut buf = [0u8; 12];
        buf[0] = self.packet_type;
        buf[1] = self.flags;
        buf[2..4].copy_from_slice(&self.payload_len.to_be_bytes());
        buf[4..12].copy_from_slice(&self.timestamp_us.to_be_bytes());
        buf
    }

    pub fn from_bytes(buf: &[u8]) -> Result<Self> {
        if buf.len() < 12 {
            bail!("Header too short: {} bytes", buf.len());
        }
        Ok(Self {
            packet_type: buf[0],
            flags: buf[1],
            payload_len: u16::from_be_bytes([buf[2], buf[3]]),
            timestamp_us: i64::from_be_bytes(buf[4..12].try_into()?),
        })
    }
}

/// Mouse move event payload (16 bytes)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MouseMovePayload {
    /// Absolute X coordinate (or delta if FLAG_RELATIVE)
    pub x: i32,
    /// Absolute Y coordinate (or delta if FLAG_RELATIVE)
    pub y: i32,
    /// Sequence number for ordering
    pub seq: u32,
    /// Reserved for future use (screen index, etc.)
    pub reserved: u32,
}

impl MouseMovePayload {
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut buf = [0u8; 16];
        buf[0..4].copy_from_slice(&self.x.to_be_bytes());
        buf[4..8].copy_from_slice(&self.y.to_be_bytes());
        buf[8..12].copy_from_slice(&self.seq.to_be_bytes());
        buf[12..16].copy_from_slice(&self.reserved.to_be_bytes());
        buf
    }

    pub fn from_bytes(buf: &[u8]) -> Result<Self> {
        if buf.len() < 16 {
            bail!("MouseMove payload too short");
        }
        Ok(Self {
            x: i32::from_be_bytes(buf[0..4].try_into()?),
            y: i32::from_be_bytes(buf[4..8].try_into()?),
            seq: u32::from_be_bytes(buf[8..12].try_into()?),
            reserved: u32::from_be_bytes(buf[12..16].try_into()?),
        })
    }
}

/// Mouse button event payload (8 bytes)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MouseButtonPayload {
    /// Button: 0=left, 1=right, 2=middle, 3=scroll_up, 4=scroll_down
    pub button: u8,
    pub reserved: [u8; 3],
    pub seq: u32,
}

impl MouseButtonPayload {
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut buf = [0u8; 8];
        buf[0] = self.button;
        buf[1..4].copy_from_slice(&self.reserved);
        buf[4..8].copy_from_slice(&self.seq.to_be_bytes());
        buf
    }

    pub fn from_bytes(buf: &[u8]) -> Result<Self> {
        if buf.len() < 8 {
            bail!("MouseButton payload too short");
        }
        Ok(Self {
            button: buf[0],
            reserved: [buf[1], buf[2], buf[3]],
            seq: u32::from_be_bytes(buf[4..8].try_into()?),
        })
    }
}

/// Keyboard event payload (8 bytes)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyPayload {
    /// Platform-independent key code (see KeyCode enum)
    pub keycode: u16,
    /// Modifier state: bit0=shift, bit1=ctrl, bit2=alt, bit3=meta
    pub modifiers: u8,
    pub reserved: u8,
    pub seq: u32,
}

impl KeyPayload {
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut buf = [0u8; 8];
        buf[0..2].copy_from_slice(&self.keycode.to_be_bytes());
        buf[2] = self.modifiers;
        buf[3] = self.reserved;
        buf[4..8].copy_from_slice(&self.seq.to_be_bytes());
        buf
    }

    pub fn from_bytes(buf: &[u8]) -> Result<Self> {
        if buf.len() < 8 {
            bail!("Key payload too short");
        }
        Ok(Self {
            keycode: u16::from_be_bytes([buf[0], buf[1]]),
            modifiers: buf[2],
            reserved: buf[3],
            seq: u32::from_be_bytes(buf[4..8].try_into()?),
        })
    }
}

/// A full input event (header + payload) ready for UDP transmission
#[derive(Debug, Clone)]
pub struct InputPacket {
    pub header: PacketHeader,
    pub payload: Vec<u8>,
}

impl InputPacket {
    pub fn mouse_move(x: i32, y: i32, seq: u32, relative: bool) -> Self {
        let flags = if relative { FLAG_RELATIVE } else { 0 };
        let payload = MouseMovePayload {
            x,
            y,
            seq,
            reserved: 0,
        }
        .to_bytes()
        .to_vec();
        Self {
            header: PacketHeader::new(PKT_MOUSE_MOVE, flags, payload.len() as u16),
            payload,
        }
    }

    pub fn mouse_button(button: u8, seq: u32, press: bool) -> Self {
        let flags = if press { FLAG_PRESS } else { FLAG_RELEASE };
        let payload = MouseButtonPayload {
            button,
            reserved: [0; 3],
            seq,
        }
        .to_bytes()
        .to_vec();
        Self {
            header: PacketHeader::new(PKT_MOUSE_BUTTON, flags, payload.len() as u16),
            payload,
        }
    }

    pub fn key_event(keycode: u16, modifiers: u8, seq: u32, press: bool) -> Self {
        let flags = if press { FLAG_PRESS } else { FLAG_RELEASE };
        let payload = KeyPayload {
            keycode,
            modifiers,
            reserved: 0,
            seq,
        }
        .to_bytes()
        .to_vec();
        Self {
            header: PacketHeader::new(PKT_KEY, flags, payload.len() as u16),
            payload,
        }
    }

    pub fn ping(seq: u32) -> Self {
        let payload = seq.to_be_bytes().to_vec();
        Self {
            header: PacketHeader::new(PKT_PING, 0, 4),
            payload,
        }
    }

    pub fn enter_screen() -> Self {
        Self {
            header: PacketHeader::new(PKT_ENTER_SCREEN, 0, 0),
            payload: Vec::new(),
        }
    }

    pub fn exit_screen() -> Self {
        Self {
            header: PacketHeader::new(PKT_EXIT_SCREEN, 0, 0),
            payload: Vec::new(),
        }
    }

    /// Serialize to wire format: header(12) + payload(N)
    pub fn to_wire(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(12 + self.payload.len());
        buf.extend_from_slice(&self.header.to_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub fn from_wire(buf: &[u8]) -> Result<Self> {
        if buf.len() < 12 {
            bail!("Packet too short");
        }
        let header = PacketHeader::from_bytes(buf)?;
        let payload_end = 12 + header.payload_len as usize;
        if buf.len() < payload_end {
            bail!("Truncated payload");
        }
        Ok(Self {
            payload: buf[12..payload_end].to_vec(),
            header,
        })
    }
}

/// Normalized key codes for cross-platform use
/// Maps to a subset of USB HID usage codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum KeyCode {
    // Letters
    A = 0x04, B = 0x05, C = 0x06, D = 0x07, E = 0x08, F = 0x09,
    G = 0x0A, H = 0x0B, I = 0x0C, J = 0x0D, K = 0x0E, L = 0x0F,
    M = 0x10, N = 0x11, O = 0x12, P = 0x13, Q = 0x14, R = 0x15,
    S = 0x16, T = 0x17, U = 0x18, V = 0x19, W = 0x1A, X = 0x1B,
    Y = 0x1C, Z = 0x1D,
    // Numbers
    Num1 = 0x1E, Num2 = 0x1F, Num3 = 0x20, Num4 = 0x21, Num5 = 0x22,
    Num6 = 0x23, Num7 = 0x24, Num8 = 0x25, Num9 = 0x26, Num0 = 0x27,
    // Special
    Return = 0x28, Escape = 0x29, Backspace = 0x2A, Tab = 0x2B, Space = 0x2C,
    Minus = 0x2D, Equal = 0x2E, LeftBracket = 0x2F, RightBracket = 0x30,
    // Function keys
    F1 = 0x3A, F2 = 0x3B, F3 = 0x3C, F4 = 0x3D, F5 = 0x3E, F6 = 0x3F,
    F7 = 0x40, F8 = 0x41, F9 = 0x42, F10 = 0x43, F11 = 0x44, F12 = 0x45,
    // Navigation
    Home = 0x4A, PageUp = 0x4B, Delete = 0x4C, End = 0x4D, PageDown = 0x4E,
    ArrowRight = 0x4F, ArrowLeft = 0x50, ArrowDown = 0x51, ArrowUp = 0x52,
    // Modifiers
    LeftCtrl = 0xE0, LeftShift = 0xE1, LeftAlt = 0xE2, LeftMeta = 0xE3,
    RightCtrl = 0xE4, RightShift = 0xE5, RightAlt = 0xE6, RightMeta = 0xE7,
    // Other
    Unknown = 0xFFFF,
}

impl KeyCode {
    pub fn from_u16(val: u16) -> Self {
        match val {
            0x04 => Self::A, 0x05 => Self::B, 0x06 => Self::C, 0x07 => Self::D,
            0x08 => Self::E, 0x09 => Self::F, 0x0A => Self::G, 0x0B => Self::H,
            0x0C => Self::I, 0x0D => Self::J, 0x0E => Self::K, 0x0F => Self::L,
            0x10 => Self::M, 0x11 => Self::N, 0x12 => Self::O, 0x13 => Self::P,
            0x14 => Self::Q, 0x15 => Self::R, 0x16 => Self::S, 0x17 => Self::T,
            0x18 => Self::U, 0x19 => Self::V, 0x1A => Self::W, 0x1B => Self::X,
            0x1C => Self::Y, 0x1D => Self::Z,
            0x1E => Self::Num1, 0x1F => Self::Num2, 0x20 => Self::Num3,
            0x21 => Self::Num4, 0x22 => Self::Num5, 0x23 => Self::Num6,
            0x24 => Self::Num7, 0x25 => Self::Num8, 0x26 => Self::Num9,
            0x27 => Self::Num0,
            0x28 => Self::Return, 0x29 => Self::Escape, 0x2A => Self::Backspace,
            0x2B => Self::Tab, 0x2C => Self::Space,
            0x3A => Self::F1, 0x3B => Self::F2, 0x3C => Self::F3, 0x3D => Self::F4,
            0x3E => Self::F5, 0x3F => Self::F6, 0x40 => Self::F7, 0x41 => Self::F8,
            0x42 => Self::F9, 0x43 => Self::F10, 0x44 => Self::F11, 0x45 => Self::F12,
            0x4A => Self::Home, 0x4B => Self::PageUp, 0x4C => Self::Delete,
            0x4D => Self::End, 0x4E => Self::PageDown,
            0x4F => Self::ArrowRight, 0x50 => Self::ArrowLeft,
            0x51 => Self::ArrowDown, 0x52 => Self::ArrowUp,
            0xE0 => Self::LeftCtrl, 0xE1 => Self::LeftShift, 0xE2 => Self::LeftAlt,
            0xE3 => Self::LeftMeta, 0xE4 => Self::RightCtrl, 0xE5 => Self::RightShift,
            0xE6 => Self::RightAlt, 0xE7 => Self::RightMeta,
            _ => Self::Unknown,
        }
    }
}
