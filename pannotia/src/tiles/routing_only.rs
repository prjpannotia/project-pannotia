//! Routing-only tiles
//!
//! This tile behaves like a logic tile without any logic cells inside.
//! This means that there are no `IMUX` nor control signals.
//! Right-going neighbor wires instead select from the `RMUX` self-wires.

use super::generic_routing::{GenericRoutingRefMutTrait, GenericRoutingRefTrait, RMUX, RMUXRef};

use super::*;

make_tile_ref! {
    /// Access to a routing-only tile
    RoutingOnlyTileRef = TileType::RoutingOnly
}

struct OMUXRef(u8);
impl FieldPositionCalculator for OMUXRef {
    #[inline]
    fn get_bit_pos(&self, _biti: usize) -> TileRelativeBitPos {
        assert!(self.0 < 16, "output index out of range");
        TileRelativeBitPos {
            x: 15,
            y: self.0 as u32 * 4 + if self.0 >= 8 { 4 } else { 0 },
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
