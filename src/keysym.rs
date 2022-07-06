// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Copyright 2022 Oxide Computer Company

use Keysym::*;

#[derive(Debug)]
pub enum Keysym {
    Unknown(u32),
    Utf32(char),
    Backspace,
    Tab,
    ReturnOrEnter,
    Escape,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Left,
    Up,
    Right,
    Down,
    FunctionKey(u8),
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    MetaLeft,
    MetaRight,
    AltLeft,
    AltRight,
}

impl TryFrom<u32> for Keysym {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        const XK_F1: u32 = 0xffbe;
        const XK_F12: u32 = 0xffc9;

        match value {
            0xff08 => Ok(Backspace),
            0xff09 => Ok(Tab),
            0xff0d => Ok(ReturnOrEnter),
            0xff1b => Ok(Escape),
            0xff63 => Ok(Insert),
            0xffff => Ok(Delete),
            0xff50 => Ok(Home),
            0xff57 => Ok(End),
            0xff55 => Ok(PageUp),
            0xff56 => Ok(PageDown),
            0xff51 => Ok(Left),
            0xff52 => Ok(Up),
            0xff53 => Ok(Right),
            0xff54 => Ok(Down),
            f if (f >= XK_F1 && f <= XK_F12) => {
                let n = f - XK_F1 + 1;
                // TODO: handle cast
                Ok(FunctionKey(n as u8))
            }
            0xffe1 => Ok(ShiftLeft),
            0xffe2 => Ok(ShiftRight),
            0xffe3 => Ok(ControlLeft),
            0xffe4 => Ok(ControlRight),
            0xffe7 => Ok(MetaLeft),
            0xffe8 => Ok(MetaRight),
            0xffe9 => Ok(AltLeft),
            0xffea => Ok(AltRight),

            // TODO: figure out if there's a better way to map codes
            other => {
                let c = char::from_u32(other);
                match c {
                    // TODO: figure out what to do with these
                    None => Ok(Unknown(other)),
                    Some(v) => Ok(Utf32(v)),
                }
            }
        }
    }
}
