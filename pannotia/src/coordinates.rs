//! Coordinates (i.e. `(x, y)` pairs) needed for accessing bitstreams

use std::borrow::{Borrow, BorrowMut};
use std::fmt::Display;
use std::marker::PhantomData;

use crate::chips::Family;
use crate::container::DebugTracer;

/// The location of a tile within the array
///
/// The origin for this coordinate system is at the bottom-left,
/// with x increasing when moving right and y increasing when moving up
/// (a "mathematics" convention).
///
/// This has a specific repr to be convenient for the downstream PnR crate
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct TilePos {
    pub y: u8,
    pub x: u8,
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
    pub y: u8,
    pub x: u8,
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
                    0 => tile_bit_pos.x as u32, // left-side IO
                    1..=12 => tile_bit_pos.x as u32 + 20 + ((tile_pos.x as u32 - 1) * 36), // tiles left of BRAM
                    13 => {
                        // BRAM column
                        if (5..=12).contains(&tile_pos.y) {
                            // a hard-IP tile, which is *right* aligned
                            tile_bit_pos.x as u32 + 20 + 12 * 36 + 180 - 20
                        } else {
                            // normal BRAM
                            tile_bit_pos.x as u32 + 20 + 12 * 36
                        }
                    }
                    14..=20 => {
                        tile_bit_pos.x as u32 + 20 + 12 * 36 + 180 + ((tile_pos.x as u32 - 14) * 36)
                    } // tiles right of BRAM
                    21 => tile_bit_pos.x as u32 + 20 + 12 * 36 + 180 + 7 * 36, // routing-only column
                    22 => tile_bit_pos.x as u32 + 20 + 12 * 36 + 180 + 7 * 36 + 16, // right-side IO
                    _ => unreachable!(),
                };
                // NOTE: the tile position's y-axis and the bit coordinate y-axis go in opposite directions
                let y_out = match tile_pos.y {
                    13 => tile_bit_pos.y as u32, // top-side IO
                    1..=12 => tile_bit_pos.y as u32 + 22 + ((12 - tile_pos.y as u32) * 68), // central portion
                    0 => tile_bit_pos.y as u32 + 22 + 12 * 68, // bottom-side IO
                    _ => unreachable!(),
                };

                GlobalBitPos { y: y_out, x: x_out }
            }
        }
    }
}

/// Internal trait which returns tile-relative bit positions for a given field
pub(crate) trait FieldPositionCalculator {
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos;
}
/// Internal helper to convert tile-relative bit positions into something [bitmux] can use.
pub(crate) struct GenericFieldRef<
    D: DebugTracer,
    Ref: Borrow<crate::container::Bitstream<D>>,
    F: FieldPositionCalculator,
> {
    pub(crate) bitstream: Ref,
    pub(crate) tile_pos: TilePos,
    pub(crate) field_pos: F,
    pub(crate) _d: PhantomData<D>,
}
impl<
    D: DebugTracer,
    Ref: Borrow<crate::container::Bitstream<D>>,
    F: FieldPositionCalculator + std::fmt::Debug,
> bitmux::BitGetter for GenericFieldRef<D, Ref, F>
{
    #[inline]
    fn get_bit(&self, biti: usize) -> bool {
        let tile_relative_pos = self.field_pos.get_bit_pos(biti);
        let bitstream = self.bitstream.borrow();
        let family = bitstream.family();
        let global_bit_pos: GlobalBitPos = (family, self.tile_pos, tile_relative_pos).into();
        bitstream.debug_log_access(
            global_bit_pos,
            self.tile_pos,
            tile_relative_pos,
            &self.field_pos,
        );
        bitstream.get_logic_array_bit(global_bit_pos)
    }
}
impl<D: DebugTracer, Ref: BorrowMut<crate::container::Bitstream<D>>, F: FieldPositionCalculator>
    bitmux::BitSetter for GenericFieldRef<D, Ref, F>
{
    #[inline]
    fn set_bit(&mut self, biti: usize, val: bool) {
        let tile_relative_pos = self.field_pos.get_bit_pos(biti);
        let bitstream = self.bitstream.borrow_mut();
        let family = bitstream.family();
        let global_bit_pos: GlobalBitPos = (family, self.tile_pos, tile_relative_pos).into();
        bitstream.set_logic_array_bit(global_bit_pos, val);
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
                x: (20 + 12 * 36 + 180 + 7 * 36 + 16) + 11,
                y: (22) + 22
            }
        );
    }
}
