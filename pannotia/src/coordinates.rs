//! Coordinates (i.e. `(x, y)` pairs) needed for accessing bitstreams

use std::fmt::Display;

use crate::chips::Family;

/// The location of a tile within the array
///
/// The origin for this coordinate system is at the bottom-left,
/// with x increasing when moving right and y increasing when moving up
/// (a "mathematics" convention).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TilePos {
    pub y: u32,
    pub x: u32,
}
impl Display for TilePos {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}, {}", self.x, self.y)
    }
}

/// The location of a bit within the main array's configuration data
///
/// The origin for this coordinate system is at the top-left,
/// with x increasing when moving right and y increasing when moving down
/// (a "computer graphics" convention).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct GlobalBitPos {
    pub y: u32,
    pub x: u32,
}
impl Display for GlobalBitPos {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}, {}", self.x, self.y)
    }
}

/// The location of a bit within one tile's configuration data
///
/// The origin for this coordinate system is at the top-left,
/// with x increasing when moving right and y increasing when moving down
/// (a "computer graphics" convention).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TileRelativeBitPos {
    pub y: u32,
    pub x: u32,
}
impl Display for TileRelativeBitPos {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}, {}", self.x, self.y)
    }
}

/// Convert a tile-relative coordinate to a global coordinate
impl From<(Family, TilePos, TileRelativeBitPos)> for GlobalBitPos {
    fn from((family, tile_pos, tile_bit_pos): (Family, TilePos, TileRelativeBitPos)) -> Self {
        let (tile_w, tile_h) = family.tile_dims();
        assert!(tile_pos.x < tile_w);
        assert!(tile_pos.y < tile_h);
        match family {
            Family::AGRV2K => {
                let x_out = match tile_pos.x {
                    0 => tile_bit_pos.x,                                     // left-side IO
                    1..=12 => tile_bit_pos.x + 20 + ((tile_pos.x - 1) * 36), // tiles left of BRAM
                    13 => tile_bit_pos.x + 20 + 12 * 36,                     // BRAM column
                    14..=20 => tile_bit_pos.x + 20 + 12 * 36 + 180 + ((tile_pos.x - 14) * 36), // tiles right of BRAM
                    21 => tile_bit_pos.x + 20 + 12 * 36 + 180 + 7 * 36, // routing-only column
                    22 => tile_bit_pos.x + 20 + 12 * 36 + 180 + 7 * 36 + 20, // right-side IO
                    _ => unreachable!(),
                };
                // NOTE: the tile position's y-axis and the bit coordinate y-axis go in opposite directions
                let y_out = match tile_pos.y {
                    13 => tile_bit_pos.y,                                     // top-side IO
                    1..=12 => tile_bit_pos.y + 22 + ((12 - tile_pos.y) * 68), // central portion
                    0 => tile_bit_pos.y + 22 + 12 * 68,                       // bottom-side IO
                    _ => unreachable!(),
                };

                GlobalBitPos { y: y_out, x: x_out }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agrv2k_tile_coord_convert() {
        // logic tile in the bottom-left
        assert_eq!(
            GlobalBitPos::from((
                Family::AGRV2K,
                TilePos { x: 1, y: 1 },
                TileRelativeBitPos { x: 11, y: 22 }
            )),
            GlobalBitPos {
                x: (20) + 11,
                y: (22 + 11 * 68) + 22
            }
        );

        // logic tile at the top, right of the BRAM column
        assert_eq!(
            GlobalBitPos::from((
                Family::AGRV2K,
                TilePos { x: 14, y: 12 },
                TileRelativeBitPos { x: 11, y: 22 }
            )),
            GlobalBitPos {
                x: (20 + 12 * 36 + 180) + 11,
                y: (22) + 22
            }
        );

        // logic tile in the top-right
        assert_eq!(
            GlobalBitPos::from((
                Family::AGRV2K,
                TilePos { x: 20, y: 12 },
                TileRelativeBitPos { x: 11, y: 22 }
            )),
            GlobalBitPos {
                x: (20 + 12 * 36 + 180 + 6 * 36) + 11,
                y: (22) + 22
            }
        );

        // IO tile in the top-right
        assert_eq!(
            GlobalBitPos::from((
                Family::AGRV2K,
                TilePos { x: 22, y: 12 },
                TileRelativeBitPos { x: 11, y: 22 }
            )),
            GlobalBitPos {
                x: (20 + 12 * 36 + 180 + 7 * 36 + 20) + 11,
                y: (22) + 22
            }
        );
    }
}
