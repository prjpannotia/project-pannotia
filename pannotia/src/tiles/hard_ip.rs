//! Interface to external hard IP blocks

use std::fmt::Display;

/// A mux with 13 choices and an optional invert
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Mux13Inv {
    VCC,
    GND,
    I { invert: bool, i: u8 },
}
impl Default for Mux13Inv {
    fn default() -> Self {
        Self::VCC
    }
}
impl Display for Mux13Inv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VCC => write!(f, "VCC"),
            Self::GND => write!(f, "GND"),
            Self::I { invert, i } => {
                if *invert {
                    write!(f, "!")?;
                }
                write!(f, "#{i}")
            }
        }
    }
}
impl bitmux::BitstreamField for Mux13Inv {
    fn get(b: impl bitmux::BitGetter) -> Self {
        Self::from_bits(b.get_bits::<9>())
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        b.set_bits::<9>(self.to_bits());
    }
}
impl Mux13Inv {
    pub(crate) fn from_bits(bits: u32) -> Self {
        let invert = bits & 0b1_0000_0000 != 0;
        bitmux::twohot!(3, 4, match bits & 0b1111_1111 {
            #bits => Self::I { invert, i: #val },
            0b1000_0000 => Self::I { invert, i: 12 },
            0 if invert => Self::GND,
            0 if !invert => Self::VCC,
            _ => panic!("invalid Mux13Inv {bits:03b}"),
        })
    }

    pub(crate) fn to_bits(self) -> u32 {
        let mut bits = bitmux::twohot!(3, 4, match self {
            Self::I { i: #val, .. } => #bits,
            Self::I { i: 12, .. } => 0b1000_0000,
            Self::GND => 0b1_0000_0000,
            Self::VCC => 0,
            _ => panic!("invalid Mux13Inv {}", self),
        });
        if let Self::I { invert: true, .. } = self {
            bits |= 0b1_0000_0000;
        }
        bits
    }
}

/// A mux with 17 choices and an optional invert
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Mux17Inv {
    VCC,
    GND,
    I { invert: bool, i: u8 },
}
impl Default for Mux17Inv {
    fn default() -> Self {
        Self::VCC
    }
}
impl Display for Mux17Inv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VCC => write!(f, "VCC"),
            Self::GND => write!(f, "GND"),
            Self::I { invert, i } => {
                if *invert {
                    write!(f, "!")?;
                }
                write!(f, "#{i}")
            }
        }
    }
}
impl bitmux::BitstreamField for Mux17Inv {
    fn get(b: impl bitmux::BitGetter) -> Self {
        Self::from_bits(b.get_bits::<10>())
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        b.set_bits::<10>(self.to_bits());
    }
}
impl Mux17Inv {
    pub(crate) fn from_bits(bits: u32) -> Self {
        let invert = bits & 0b10_0000_0000 != 0;
        bitmux::twohot!(4, 4, match bits & 0b1_1111_1111 {
            #bits => Self::I { invert, i: #val },
            0b1_0000_0000 => Self::I { invert, i: 16 },
            0 if invert => Self::GND,
            0 if !invert => Self::VCC,
            _ => panic!("invalid Mux17Inv {bits:03b}"),
        })
    }

    pub(crate) fn to_bits(self) -> u32 {
        let mut bits = bitmux::twohot!(4, 4, match self {
            Self::I { i: #val, .. } => #bits,
            Self::I { i: 16, .. } => 0b1_0000_0000,
            Self::GND => 0b10_0000_0000,
            Self::VCC => 0,
            _ => panic!("invalid Mux17Inv {}", self),
        });
        if let Self::I { invert: true, .. } = self {
            bits |= 0b10_0000_0000;
        }
        bits
    }
}
