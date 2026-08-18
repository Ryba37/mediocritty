use serde::{Deserialize, Deserializer};

const CUBE: [u8; 6] = [0, 95, 135, 175, 215, 255];

#[derive(Clone, Copy)]
pub struct HexColor(pub [u8; 3]);

impl<'de> Deserialize<'de> for HexColor {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s: &str = Deserialize::deserialize(d)?;
        let s = s.trim_start_matches('#');
        if s.len() != 6 {
            return Err(serde::de::Error::custom("color must be in #rrggbb format"));
        }
        let bytes = (0..3)
            .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16))
            .collect::<Result<Vec<_>, _>>()
            .map_err(serde::de::Error::custom)?;
        Ok(HexColor([bytes[0], bytes[1], bytes[2]]))
    }
}

impl Default for HexColor {
    fn default() -> Self {
        HexColor([0, 0, 0])
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub struct Palette {
    pub black: HexColor,
    pub red: HexColor,
    pub green: HexColor,
    pub yellow: HexColor,
    pub blue: HexColor,
    pub magenta: HexColor,
    pub cyan: HexColor,
    pub white: HexColor,
    pub bright_black: HexColor,
    pub bright_red: HexColor,
    pub bright_green: HexColor,
    pub bright_yellow: HexColor,
    pub bright_blue: HexColor,
    pub bright_magenta: HexColor,
    pub bright_cyan: HexColor,
    pub bright_white: HexColor,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            black: HexColor([0x28, 0x28, 0x28]),
            red: HexColor([0xcc, 0x24, 0x1d]),
            green: HexColor([0x98, 0x97, 0x1a]),
            yellow: HexColor([0xd7, 0x99, 0x21]),
            blue: HexColor([0x45, 0x85, 0x88]),
            magenta: HexColor([0xb1, 0x62, 0x86]),
            cyan: HexColor([0x68, 0x9d, 0x6a]),
            white: HexColor([0xa8, 0x99, 0x84]),
            bright_black: HexColor([0x92, 0x83, 0x74]),
            bright_red: HexColor([0xfb, 0x49, 0x34]),
            bright_green: HexColor([0xb8, 0xbb, 0x26]),
            bright_yellow: HexColor([0xfa, 0xbd, 0x2f]),
            bright_blue: HexColor([0x83, 0xa5, 0x98]),
            bright_magenta: HexColor([0xd3, 0x86, 0x9b]),
            bright_cyan: HexColor([0x8e, 0xc0, 0x7c]),
            bright_white: HexColor([0xeb, 0xdb, 0xb2]),
        }
    }
}

impl Palette {
    pub fn to_array(&self) -> [[u8; 3]; 16] {
        [
            self.black.0,
            self.red.0,
            self.green.0,
            self.yellow.0,
            self.blue.0,
            self.magenta.0,
            self.cyan.0,
            self.white.0,
            self.bright_black.0,
            self.bright_red.0,
            self.bright_green.0,
            self.bright_yellow.0,
            self.bright_blue.0,
            self.bright_magenta.0,
            self.bright_cyan.0,
            self.bright_white.0,
        ]
    }
}

pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub fn indexed(i: u8, palette: &[[u8; 3]; 16]) -> [u8; 3] {
    match i {
        0..=15 => palette[i as usize],
        16..=231 => {
            let n = i - 16;
            [
                CUBE[(n / 36) as usize],
                CUBE[(n / 6 % 6) as usize],
                CUBE[(n % 6) as usize],
            ]
        }
        _ => {
            let v = 8 + (i - 232) * 10;
            [v, v, v]
        }
    }
}
