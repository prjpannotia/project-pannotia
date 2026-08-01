//! Settings which control the IO pad ring

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

pub trait PadRingExt {
    fn pad_input_en(&self, pad_i: u8) -> bool;
    fn pad_drive_strength(&self, pad_i: u8) -> u8;
}
impl<D: crate::container::DebugTracer> PadRingExt for crate::container::Bitstream<D> {
    fn pad_input_en(&self, pad_i: u8) -> bool {
        let biti = pad_i_to_bit_i(pad_i) - 0;
        self.get_aux_array_bit(1, 0, biti)
    }
    fn pad_drive_strength(&self, pad_i: u8) -> u8 {
        let biti = pad_i_to_bit_i(pad_i) - 4;

        let mut bits = 0;
        for i in 0..4 {
            if self.get_aux_array_bit(1, 0, biti - i) {
                bits |= 1 << i;
            }
        }

        bits
    }
}
