//! Pretty-print muxes
//!
//! This crate *relies on* autoderef specialization!

use std::fmt::Display;

use pannotia::prelude::*;
use pannotia::routedb::*;

/// This is needed to get around issues with blanket impls
pub struct PrettyPrintWrap<T>(pub T);

pub trait PrettyPrintGeneric {
    fn pretty_print<W: std::fmt::Write>(
        &self,
        w: W,
        family: Family,
        tile_pos: TilePos,
        tile_type: TileType,
        i: u8,
    ) -> std::fmt::Result;
}
impl<T: Display> PrettyPrintGeneric for PrettyPrintWrap<T> {
    fn pretty_print<W: std::fmt::Write>(
        &self,
        mut w: W,
        _family: Family,
        _tile_pos: TilePos,
        _tile_type: TileType,
        _i: u8,
    ) -> std::fmt::Result {
        write!(w, "{}", self.0)
    }
}

pub trait PrettyPrintRMUX {
    fn pretty_print<W: std::fmt::Write>(
        &self,
        w: W,
        family: Family,
        tile_pos: TilePos,
        tile_type: TileType,
        i: u8,
    ) -> std::fmt::Result;
}
impl PrettyPrintRMUX for &PrettyPrintWrap<mux::RMUX> {
    fn pretty_print<W: std::fmt::Write>(
        &self,
        mut w: W,
        family: Family,
        tile_pos: TilePos,
        tile_type: TileType,
        i: u8,
    ) -> std::fmt::Result {
        if let mux::RMUX::I(rmux_inp_i) = self.0 {
            // normal format
            write!(w, "{}\t// ", self.0)?;

            // mux purpose
            let this_rmux = RMUX_PURPOSE[i as usize];
            match this_rmux {
                RMUXPurpose::SelfWire => write!(w, "rmux_self[{}]", i / 6 * 2 + i % 6 - 4)?,
                RMUXPurpose::LeftNeighbor => write!(w, "T1_W[{}]", i / 6)?,
                RMUXPurpose::Span4 {
                    going_dir,
                    wire_idx,
                } => write!(w, "T4_{}[{}]", going_dir, wire_idx)?,
            }

            write!(w, " = ")?;

            // mux source (decoded)
            let rmux_src = rmux_input(i, rmux_inp_i, tile_type == TileType::BRAM);
            match rmux_src {
                RMUXSource::GlobalToLocal(i) => write!(w, "glb2loc[{i}]")?,
                RMUXSource::RMUX(i) => write!(w, "rmux[{i}]")?,
                RMUXSource::CellOutput(i) => write!(w, "this_output[{i}]")?,
                RMUXSource::RoutingWire(src_wire) => {
                    let abs_wire = src_wire.to_absolute(family, tile_pos);
                    write!(
                        w,
                        "tile[{}] {}_{}[{}]",
                        abs_wire.tile, abs_wire.ty, abs_wire.going_dir, abs_wire.wire_idx
                    )?
                }
                _ => unreachable!(),
            }
        }

        Ok(())
    }
}
