//! General-purpose interconnect
//!
//! These muxes are found in the "main" section of the array
//! (i.e. in logic, BRAM, and routing-only tiles, and not at the edges).
//!
//! The exact set of inputs at each mux varies slightly depending on the tile type,
//! but the routing is generic enough that it makes sense to abstract it via a common interface.
//!
//! General-purpose routing wires travel along the x/y-axis (i.e. no diagonals nor turns).
//! They are unidirectional and driven from exactly one place,
//! and the specific input that is chosen is controlled by the associated [RMUX].
//! These wires also have a fixed length (or "span") (in units of tiles)
//! and pass "over/through" the intermediate tiles before finally terminating.
//!
//! In the vendor toolchain, these wires are named `Tn{X|Y}_{N|S|E|W}_…`.
//! This indicates the length of the wire (`n`), the axis it travels along,
//! and the direction (using compass directions) the signal flows _towards_.
//! For example, a `T4X_E_…` wire is a horizontal span-4 wire traveling right.
//!
//! Wires are grouped into bundles according to the tile from which it originates,
//! and so a full name of such a _bundle_ is `Tn{X|Y}_{N|S|E|W}_I{0..=3}`.
//! For example, the `T4X_E_I1` bundle comes from two tiles to the _left_ of this one
//! (the signal is _traveling_ east, so it comes _from_ the west/left).
//! Within each bundle, a numeric index selects one specific wire.
//!
//! Visually, the general-purpose interconnect looks something like this:
//! ```text
//!                +----------------+
//!                |                |
//! >-- T4X_E_I0 -->-------+---\  /->-- T4X_E_O ----->
//! >-- T4X_E_I1 -->-----+-|--\ \-|->-- T4X_E_I1_O -->
//! >-- T4X_E_I2 -->---+-|-|-\ \--|->-- T4X_E_I2_O -->
//! >-- T4X_E_I3 -->-\ | | |  \---|->-- T4X_E_I3_O -->
//!                | | | | |      | |
//!                | ∨ ∨ ∨ ∨      ∧ |
//!                | tile internals |
//!                +----------------+
//! ```
//! A similar pattern repeats for each of the four directions.
//! On parts with long wires, a similar pattern also repeats for those.
//!
//! In addition to these wires that have been depicted and explained,
//! tiles also have span-1/`T1`/"neighbor" wires and span-0/`T0`/"self" wires.
//! Neighbor wires are very similar to other wires except that they never
//! pass "through" tiles and only ever terminate into the adjacent tile.
//! They also only exist along the x-axis. There are no neighbor wires going up/down.
//! Self wires can only ever route further into "tile internals" and cannot leave the tile.
//!
//! There are:
//! * 16 `T1` wires in each direction left/right
//! * 12 `T4` wires in each direction left/right/up/down
//!   (except for the AG16K where there are 16 wires going left/right)
//! * in the AG16K, one long wire in each direction
//! * 96 `RMUX` per tile
//!
//! As a quirk, left-going neighbor wires are controlled by `RMUX`,
//! but right-going neighbor wires are controlled by tile-specific output muxes.
//! "Whatever is left" of the `RMUX`es control `T0` wires.

use std::borrow::{Borrow, BorrowMut};
use std::fmt::Display;

use super::*;

use bitmux::BitstreamField;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct GenericRoutingRef<D: DebugTracer, Ref: Borrow<Bitstream<D>>> {
    pub(super) r: Ref,
    pub(super) p: TilePos,
    pub(super) _d: PhantomData<D>,
}
impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> TileRefTrait<D, Ref> for GenericRoutingRef<D, Ref> {
    fn tile_type(&self) -> TileType {
        let family = self.r.borrow().family();
        family.get_tile_type(self.p)
    }
    fn pos(&self) -> TilePos {
        self.p
    }
    fn as_base_tile(self) -> TileRef<D, Ref> {
        TileRef {
            r: self.r,
            p: self.p,
            _d: PhantomData,
        }
    }
}

pub(crate) struct RMUXRef {
    pub(crate) is_bram: bool,
    pub(crate) i: u8,
}
impl FieldPositionCalculator for RMUXRef {
    #[inline]
    fn get_bit_pos(&self, biti: usize) -> TileRelativeBitPos {
        assert!(self.i < 96, "RMUX index out of range");
        // There are 96 RMUXes per tile, which we _logically_ group as 16x 6,
        // but which are _physically_ grouped more like 4x 24.
        let group_of_24 = (self.i / 24) as u32;
        let group_within_24 = self.i % 24;

        // Each sub-group of 24 is 3 columns of 8 in this strange order.
        // The middle column also has its bits flipped.
        // Each mux is a 5 wide by 2 high block, so we _can_ start computing the
        // y coordinate now
        let (mut y_offs, col_of_three) = bitmux::bittable!(
            ((#y + group_of_24 * 8) * 2, #x),
            0       2       4,
            6       8       10,
            17      13      15,
            23      19      21,
            1       3       5,
            7       9       11,
            16      12      14,
            22      18      20,
        )[group_within_24 as usize];

        // These tiles all have a 4-row gap in the middle for global routing bits
        if self.i >= 48 {
            y_offs += 4;
        }

        // We can perform a basic lookup of the 5x2 bit block now
        let (y, xbase) = bitmux::bittable!(
            (y_offs + #y, #x),
            7   6   4   2   0,
            8   9   5   3   1,
        )[biti];

        // Finally deal with inverting the middle column
        let mut x = match col_of_three {
            0 => xbase,
            1 => 9 - xbase,
            2 => 10 + xbase,
            _ => unreachable!(),
        };

        // For a BRAM tile, this is scooted right by 1
        if self.is_bram {
            x += 1;
        }

        TileRelativeBitPos { y, x }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum RMUX {
    None,
    I(u8),
}
impl Default for RMUX {
    fn default() -> Self {
        Self::None
    }
}
impl Display for RMUX {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "<unset>"),
            Self::I(i) => write!(f, "#{i}"),
        }
    }
}
impl bitmux::BitstreamField for RMUX {
    fn get(b: impl bitmux::BitGetter) -> Self {
        let bits = b.get_bits::<10>();
        bitmux::twohot!(3, 7, match bits {
            #bits => RMUX::I(#val),
            0 => RMUX::None,
            _ => panic!("invalid RMUX {bits:010b}"),
        })
    }
    fn set(&self, mut b: impl bitmux::BitSetter) {
        let bits = bitmux::twohot!(3, 7, match self {
            RMUX::I(#val) => #bits,
            RMUX::None => 0,
            _ => panic!("invalid RMUX {}", self),
        });
        b.set_bits::<10>(bits);
    }
}

pub trait GenericRoutingRefTrait {
    fn rmux(&self, rmux_idx: u8) -> RMUX;
}
pub trait GenericRoutingRefMutTrait: GenericRoutingRefTrait {
    fn set_rmux(&mut self, rmux_idx: u8, val: RMUX);
}

impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> GenericRoutingRefTrait
    for GenericRoutingRef<D, Ref>
{
    fn rmux(&self, rmux_idx: u8) -> RMUX {
        let is_bram = self.tile_type() == TileType::BRAM;
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow(),
            tile_pos: self.p,
            field_pos: RMUXRef {
                is_bram,
                i: rmux_idx,
            },
            _d: PhantomData,
        };
        RMUX::get(ref_)
    }
}
impl<D: DebugTracer, Ref: BorrowMut<Bitstream<D>>> GenericRoutingRefMutTrait
    for GenericRoutingRef<D, Ref>
{
    fn set_rmux(&mut self, rmux_idx: u8, val: RMUX) {
        let is_bram = self.tile_type() == TileType::BRAM;
        let ref_ = GenericFieldRef {
            bitstream: self.r.borrow_mut(),
            tile_pos: self.p,
            field_pos: RMUXRef {
                is_bram,
                i: rmux_idx,
            },
            _d: PhantomData,
        };
        val.set(ref_);
    }
}
