//! Settings which control the IO pad ring

use std::borrow::{Borrow, BorrowMut};
use std::fmt::Display;
use std::marker::PhantomData;
use std::str::FromStr;

use bitmux::BitstreamField;

use crate::container::{Bitstream, DebugTracer};
use crate::coordinates::TilePos;

const fn pad_i_to_bit_i(pad_i: u8) -> usize {
    let mut base_offset = pad_i as usize * 10;

    if pad_i >= 22 {
        base_offset += 2; // wakeup
    }
    if pad_i >= 38 {
        base_offset += 1; // boot0
    }
    if pad_i >= 40 {
        base_offset += 10; // osc
    }
    if pad_i >= 43 {
        base_offset += 13; // mipi_rx
    }
    if pad_i >= 52 {
        base_offset += 15; // mipi_tx
    }
    if pad_i >= 59 {
        base_offset += 3; // usb
    }

    833 - base_offset
}

/// Map from pad ring control bit internal index to IO tile coordinate and site
pub const PADRING_TO_TILE: [(TilePos, u8); 79] = [
    // top edge
    (TilePos { x: 14, y: 13 }, 0),
    (TilePos { x: 14, y: 13 }, 2),
    (TilePos { x: 15, y: 13 }, 0),
    (TilePos { x: 15, y: 13 }, 2),
    (TilePos { x: 15, y: 13 }, 3),
    (TilePos { x: 16, y: 13 }, 0),
    (TilePos { x: 16, y: 13 }, 1),
    (TilePos { x: 16, y: 13 }, 2),
    (TilePos { x: 16, y: 13 }, 3),
    (TilePos { x: 17, y: 13 }, 0),
    (TilePos { x: 17, y: 13 }, 1),
    (TilePos { x: 17, y: 13 }, 2),
    (TilePos { x: 17, y: 13 }, 3),
    (TilePos { x: 18, y: 13 }, 0),
    (TilePos { x: 18, y: 13 }, 1),
    (TilePos { x: 18, y: 13 }, 2),
    (TilePos { x: 18, y: 13 }, 3),
    (TilePos { x: 19, y: 13 }, 0),
    (TilePos { x: 19, y: 13 }, 1),
    (TilePos { x: 19, y: 13 }, 2),
    (TilePos { x: 19, y: 13 }, 3),
    (TilePos { x: 20, y: 13 }, 1),
    (TilePos { x: 20, y: 13 }, 2),
    (TilePos { x: 20, y: 13 }, 3),
    // right edge
    (TilePos { x: 22, y: 3 }, 0),
    (TilePos { x: 22, y: 3 }, 1),
    (TilePos { x: 22, y: 3 }, 2),
    (TilePos { x: 22, y: 3 }, 3),
    (TilePos { x: 22, y: 2 }, 3),
    (TilePos { x: 22, y: 2 }, 5),
    (TilePos { x: 22, y: 1 }, 0),
    (TilePos { x: 22, y: 1 }, 2),
    (TilePos { x: 22, y: 1 }, 3),
    (TilePos { x: 22, y: 1 }, 4),
    // bottom edge
    (TilePos { x: 20, y: 0 }, 2),
    (TilePos { x: 20, y: 0 }, 0),
    (TilePos { x: 19, y: 0 }, 3),
    (TilePos { x: 19, y: 0 }, 1),
    (TilePos { x: 18, y: 0 }, 1),
    (TilePos { x: 18, y: 0 }, 0),
    (TilePos { x: 17, y: 0 }, 2),
    (TilePos { x: 17, y: 0 }, 1),
    (TilePos { x: 17, y: 0 }, 0),
    (TilePos { x: 8, y: 0 }, 3),
    (TilePos { x: 8, y: 0 }, 2),
    (TilePos { x: 8, y: 0 }, 1),
    (TilePos { x: 8, y: 0 }, 0),
    (TilePos { x: 7, y: 0 }, 3),
    (TilePos { x: 7, y: 0 }, 1),
    (TilePos { x: 7, y: 0 }, 0),
    (TilePos { x: 6, y: 0 }, 3),
    (TilePos { x: 6, y: 0 }, 0),
    (TilePos { x: 1, y: 0 }, 3),
    (TilePos { x: 1, y: 0 }, 2),
    (TilePos { x: 1, y: 0 }, 1),
    (TilePos { x: 1, y: 0 }, 0),
    // left edge
    (TilePos { x: 0, y: 1 }, 5),
    (TilePos { x: 0, y: 1 }, 4),
    (TilePos { x: 0, y: 1 }, 3),
    (TilePos { x: 0, y: 1 }, 2),
    (TilePos { x: 0, y: 1 }, 0),
    (TilePos { x: 0, y: 2 }, 5),
    (TilePos { x: 0, y: 2 }, 4),
    (TilePos { x: 0, y: 2 }, 3),
    (TilePos { x: 0, y: 2 }, 2),
    (TilePos { x: 0, y: 2 }, 1),
    (TilePos { x: 0, y: 2 }, 0),
    (TilePos { x: 0, y: 3 }, 5),
    (TilePos { x: 0, y: 3 }, 4),
    (TilePos { x: 0, y: 3 }, 3),
    (TilePos { x: 0, y: 3 }, 2),
    (TilePos { x: 0, y: 3 }, 1),
    (TilePos { x: 0, y: 3 }, 0),
    (TilePos { x: 0, y: 4 }, 5),
    (TilePos { x: 0, y: 4 }, 4),
    (TilePos { x: 0, y: 4 }, 3),
    (TilePos { x: 0, y: 4 }, 2),
    (TilePos { x: 0, y: 4 }, 1),
    (TilePos { x: 0, y: 4 }, 0),
];

struct DriveStrengthBits<D: DebugTracer, Ref: Borrow<Bitstream<D>>> {
    r: Ref,
    pad_i: u8,
    _d: PhantomData<D>,
}
impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> bitmux::BitGetter for DriveStrengthBits<D, Ref> {
    fn get_bit(&self, biti: usize) -> bool {
        let bitstream = self.r.borrow();
        let biti_base = pad_i_to_bit_i(self.pad_i) - 4;
        bitstream.get_aux_array_bit(1, 0, biti_base - biti)
    }
}
impl<D: DebugTracer, Ref: BorrowMut<Bitstream<D>>> bitmux::BitSetter for DriveStrengthBits<D, Ref> {
    fn set_bit(&mut self, biti: usize, val: bool) {
        let bitstream = self.r.borrow_mut();
        let biti_base = pad_i_to_bit_i(self.pad_i) - 4;
        bitstream.set_aux_array_bit(1, 0, biti_base - biti, val);
    }
}

// FIXME: It is not known if other values are valid?
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[bitmux::bitenum]
pub enum DriveStrength {
    _0MA = "0000",
    _2MA = "0001",
    _4MA = "0010",
    _8MA = "0100",
    _16MA = "1000",
    _30MA = "1111",
}
impl Default for DriveStrength {
    fn default() -> Self {
        Self::_4MA
    }
}
impl Display for DriveStrength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::_0MA => write!(f, "0 mA"),
            Self::_2MA => write!(f, "2 mA"),
            Self::_4MA => write!(f, "4 mA"),
            Self::_8MA => write!(f, "8 mA"),
            Self::_16MA => write!(f, "16 mA"),
            Self::_30MA => write!(f, "30 mA"),
        }
    }
}
impl FromStr for DriveStrength {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "0 ma" => Ok(Self::_0MA),
            "2 ma" => Ok(Self::_2MA),
            "4 ma" => Ok(Self::_4MA),
            "8 ma" => Ok(Self::_8MA),
            "16 ma" => Ok(Self::_16MA),
            "30 ma" => Ok(Self::_30MA),
            _ => Err(()),
        }
    }
}

struct TerminationBits<D: DebugTracer, Ref: Borrow<Bitstream<D>>> {
    r: Ref,
    pad_i: u8,
    _d: PhantomData<D>,
}
impl<D: DebugTracer, Ref: Borrow<Bitstream<D>>> bitmux::BitGetter for TerminationBits<D, Ref> {
    fn get_bit(&self, biti: usize) -> bool {
        let bitstream = self.r.borrow();
        let biti_base = pad_i_to_bit_i(self.pad_i) - 8;
        bitstream.get_aux_array_bit(1, 0, biti_base - biti)
    }
}
impl<D: DebugTracer, Ref: BorrowMut<Bitstream<D>>> bitmux::BitSetter for TerminationBits<D, Ref> {
    fn set_bit(&mut self, biti: usize, val: bool) {
        let bitstream = self.r.borrow_mut();
        let biti_base = pad_i_to_bit_i(self.pad_i) - 8;
        bitstream.set_aux_array_bit(1, 0, biti_base - biti, val);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[bitmux::bitenum]
pub enum PullUpDown {
    None = "00",
    Down = "01",
    Up = "10",
    Keeper = "11",
}
impl Default for PullUpDown {
    fn default() -> Self {
        Self::None
    }
}
impl Display for PullUpDown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Down => write!(f, "pulldown"),
            Self::Up => write!(f, "pullup"),
            Self::Keeper => write!(f, "keeper"),
        }
    }
}
impl FromStr for PullUpDown {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "none" => Ok(Self::None),
            "pulldown" => Ok(Self::Down),
            "pullup" => Ok(Self::Up),
            "keeper" => Ok(Self::Keeper),
            _ => Err(()),
        }
    }
}

pub trait PadRingExt {
    fn pad_input_en(&self, pad_i: u8) -> bool;
    fn pad_drive_strength(&self, pad_i: u8) -> DriveStrength;
    fn pad_termination(&self, pad_i: u8) -> PullUpDown;
    fn pad_open_drain(&self, pad_i: u8) -> bool;
    fn pad_reduced_slew(&self, pad_i: u8) -> bool;
    fn pad_pullup_to_fabric(&self, pad_i: u8) -> bool;

    fn set_pad_input_en(&mut self, pad_i: u8, val: bool);
    fn set_pad_drive_strength(&mut self, pad_i: u8, val: DriveStrength);
    fn set_pad_termination(&mut self, pad_i: u8, val: PullUpDown);
    fn set_pad_open_drain(&mut self, pad_i: u8, val: bool);
    fn set_pad_reduced_slew(&mut self, pad_i: u8, val: bool);
    fn set_pad_pullup_to_fabric(&mut self, pad_i: u8, val: bool);
}
impl<D: DebugTracer> PadRingExt for Bitstream<D> {
    fn pad_input_en(&self, pad_i: u8) -> bool {
        let biti = pad_i_to_bit_i(pad_i) - 0;
        self.get_aux_array_bit(1, 0, biti)
    }
    fn pad_drive_strength(&self, pad_i: u8) -> DriveStrength {
        DriveStrength::get(DriveStrengthBits {
            r: self,
            pad_i,
            _d: PhantomData,
        })
    }
    fn pad_termination(&self, pad_i: u8) -> PullUpDown {
        PullUpDown::get(TerminationBits {
            r: self,
            pad_i,
            _d: PhantomData,
        })
    }
    fn pad_open_drain(&self, pad_i: u8) -> bool {
        let biti = pad_i_to_bit_i(pad_i) - 3;
        self.get_aux_array_bit(1, 0, biti)
    }
    fn pad_reduced_slew(&self, pad_i: u8) -> bool {
        let biti = pad_i_to_bit_i(pad_i) - 2;
        self.get_aux_array_bit(1, 0, biti)
    }
    fn pad_pullup_to_fabric(&self, pad_i: u8) -> bool {
        let biti = pad_i_to_bit_i(pad_i) - 1;
        self.get_aux_array_bit(1, 0, biti)
    }

    fn set_pad_input_en(&mut self, pad_i: u8, val: bool) {
        let biti = pad_i_to_bit_i(pad_i) - 0;
        self.set_aux_array_bit(1, 0, biti, val);
    }
    fn set_pad_drive_strength(&mut self, pad_i: u8, val: DriveStrength) {
        val.set(DriveStrengthBits {
            r: self,
            pad_i,
            _d: PhantomData,
        });
    }
    fn set_pad_termination(&mut self, pad_i: u8, val: PullUpDown) {
        val.set(TerminationBits {
            r: self,
            pad_i,
            _d: PhantomData,
        });
    }
    fn set_pad_open_drain(&mut self, pad_i: u8, val: bool) {
        let biti = pad_i_to_bit_i(pad_i) - 3;
        self.set_aux_array_bit(1, 0, biti, val);
    }
    fn set_pad_reduced_slew(&mut self, pad_i: u8, val: bool) {
        let biti = pad_i_to_bit_i(pad_i) - 2;
        self.set_aux_array_bit(1, 0, biti, val);
    }
    fn set_pad_pullup_to_fabric(&mut self, pad_i: u8, val: bool) {
        let biti = pad_i_to_bit_i(pad_i) - 1;
        self.set_aux_array_bit(1, 0, biti, val);
    }
}
