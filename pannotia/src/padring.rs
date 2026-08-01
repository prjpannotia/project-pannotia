//! Settings which control the IO pad ring

use std::borrow::{Borrow, BorrowMut};
use std::fmt::Display;
use std::marker::PhantomData;

use bitmux::BitstreamField;

use crate::container::{Bitstream, DebugTracer};

fn pad_i_to_bit_i(pad_i: u8) -> usize {
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
            DriveStrength::_0MA => write!(f, "0 mA"),
            DriveStrength::_2MA => write!(f, "2 mA"),
            DriveStrength::_4MA => write!(f, "4 mA"),
            DriveStrength::_8MA => write!(f, "8 mA"),
            DriveStrength::_16MA => write!(f, "16 mA"),
            DriveStrength::_30MA => write!(f, "30 mA"),
        }
    }
}

pub trait PadRingExt {
    fn pad_input_en(&self, pad_i: u8) -> bool;
    fn pad_drive_strength(&self, pad_i: u8) -> DriveStrength;
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
}
