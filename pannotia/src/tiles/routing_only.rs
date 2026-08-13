//! Routing-only tiles
//!
//! This tile behaves like a logic tile without any logic cells inside.
//! This means that there are no `IMUX` nor control signals.
//!
//! Each right-going neighbor wires (`T1_E`) instead makes a selection
//! from 1-of-2 of the `RMUX` local lines that _would've_ gone into `IMUX` in a logic tile.
//! This reduces the 2× 16 local lines into a 1× 16 count of neighbor wires.
//!
//! The inputs to `RMUX` which in a logic tile _would've_ come from LE outputs
//! instead comes directly from neighbor wires (`T1_W`) from the cell on the right.
//!
//! Visually, this looks like this:
//!
//! ```text
//!                                     +------------+
//! output wires other than T1_E <------|            |--+
//! general-purpose routing wires ----->| 6× 16 RMUX |  | RMUX-to-RMUX self-wires
//!                                     |            |<-+
//!                                     | 4× CtrlMUX |----------------------+
//!                                     |            |<-------------+       | 2× 16 local lines
//!                                     +------------+              |       v
//!                                         ^                       |   +----------+
//!                 +--------------------+  |                       |   | 16× OMUX |----> T1_E wires
//! global wires -> | 4× global-to-local | -+                       |   +----------+
//!                 +--------------------+                          |
//!                                                                 +-- T1_W wires
//! ```

use super::generic_routing::{GenericRoutingRefMutTrait, GenericRoutingRefTrait, RMUX, RMUXRef};

use super::*;

make_tile_ref! {
    /// Access to a routing-only tile
    RoutingOnlyTileRef = TileType::RoutingOnly
}

#[derive(Debug)]
struct OMUXRef(u8);
impl FieldPositionCalculator for OMUXRef {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 16, "output index out of range");
        TileRelativeBitPos {
            x: 15,
            y: self.0 * 4 + if self.0 >= 8 { 4 } else { 0 },
        }
    }
}

magic_tile_impl_gen! {
    impl RoutingOnlyTileRef {
        pub fn global_to_local(&self, inp_idx: u8) -> GlobalToLocalMux {
            GlobalToLocalMuxRef {
                is_bram: false,
                i: inp_idx,
            }
        }

        pub fn right_neighbor_output(&self, lc_idx: u8) -> Mux2 {
            OMUXRef(lc_idx)
        }
    }
}

magic_tile_impl_gen! {
    impl on RoutingOnlyTileRef trait GenericRoutingRefTrait, GenericRoutingRefMutTrait {
        fn rmux(&self, rmux_idx: u8) -> RMUX {
            RMUXRef {
                is_bram: false,
                i: rmux_idx,
            }
        }
    }
}
