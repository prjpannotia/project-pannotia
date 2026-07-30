//! Database of factoids about chip designs

use std::fmt;

use crate::coordinates::TilePos;
use crate::tiles::TileType;

/// Represents the "family" of the FPGA being worked on
///
/// This represents a unique bitstream format and die layout,
/// but it abstracts over details such as pinouts and mechanical packages.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Family {
    /// AGRV2K CPLDs and AG32V microcontrollers
    AGRV2K,
}
impl fmt::Display for Family {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Family {
    /// Get the device ID code for this family
    pub const fn device_id(self) -> u32 {
        match self {
            Family::AGRV2K => 0x40200001,
        }
    }

    /// Return the sizes of all the configuration arrays
    pub const fn config_bits(self) -> &'static [&'static [usize]] {
        match self {
            Family::AGRV2K => &[
                &[860 * 928], // group 0 chain 0 (main array)
                &[
                    834, // group 1 chain 0 (IO)
                    239, // group 1 chain 1 (PLL)
                ],
            ],
        }
    }

    /// Returns the size of the main logic array in bits, `(W, H)`
    ///
    /// This does _not_ include per-row unused padding bits,
    /// so W*H might not equal the number of bits returned by [config_bits](Self::config_bits)
    pub const fn main_logic_bits(self) -> (u32, u32) {
        match self {
            // NOTE: we lose 8 unused bits from the end of each row
            Family::AGRV2K => (920, 860),
        }
    }

    /// Returns the size of the logic array in tiles, `(W, H)`
    pub const fn tile_dims(self) -> (u32, u32) {
        match self {
            Family::AGRV2K => (23, 14),
        }
    }

    /// Returns the type of tile that exists at the specified position
    pub const fn get_tile_type(self, pos: TilePos) -> TileType {
        let (w, h) = self.tile_dims();
        if pos.x >= w || pos.y >= h {
            return TileType::None;
        }

        match self {
            Family::AGRV2K => {
                if pos.x >= 1 && pos.x <= 12 && pos.y >= 1 && pos.y <= 4 {
                    TileType::Logic // bottom-left block of logic
                } else if pos.x >= 14 && pos.x <= 20 && pos.y >= 1 && pos.y <= 12 {
                    TileType::Logic // right block of logic
                } else if pos.x == 21 && pos.y >= 1 && pos.y <= 12 {
                    TileType::RoutingOnly
                } else if pos.x == 22 && pos.y == 4 {
                    TileType::GCLKSW
                } else if pos.x == 22 && pos.y == 5 {
                    TileType::PLL
                } else if pos.x == 13 && pos.y >= 1 && pos.y <= 4 {
                    TileType::BRAM
                } else if pos.x == 13 && pos.y >= 5 && pos.y <= 12 {
                    TileType::LeftRightIP // MCU right-side interface
                } else if pos.x >= 1 && pos.x <= 12 && pos.y == 5 {
                    TileType::TopIP // MCU bottom-side interface
                } else if pos.x == 0 && pos.y >= 1 && pos.y <= 4 {
                    TileType::LeftRightIO // left/west IO
                } else if pos.x == 22 && pos.y >= 1 && pos.y <= 3 {
                    TileType::LeftRightIO // right/east IO
                } else if pos.x >= 14 && pos.x <= 20 && pos.y == 13 {
                    TileType::TopBottomIO // top/north IO
                } else if pos.x >= 1 && pos.x <= 20 && pos.x != 4 && pos.x != 13 && pos.y == 0 {
                    TileType::TopBottomIO // bottom/south IO
                } else if pos.x == 22 && pos.y >= 6 && pos.y <= 12 {
                    TileType::LeftRightIP // analog IP
                } else {
                    TileType::None
                }
            }
        }
    }
}
/// Try converting a device ID to a chip family
impl TryFrom<u32> for Family {
    type Error = ();
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0x40200001 => Ok(Self::AGRV2K),
            _ => Err(()),
        }
    }
}
