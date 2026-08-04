//! Bitstream container file format
//!
//! This module handles the "raw binary" bitstream container file format,
//! which is the highest abstraction level handled directly by hardware.
//!
//! This format organizes the following information:
//! - IO pad configuration
//! - PLL/clock configuration
//! - "miscellaneous" configuration
//! - the "actual logic"
//!
//! However, even though this format is structured, there is typically only one
//! (or rarely, a small number) instance of each of the above pieces of information,
//! and the "actual logic" is one gigantic block, so most details of this format
//! can be thought of as "useless" or conceptually "irrelevant" overhead.
//!
//! ## Overall structure
//!
//! A bitstream binary consists of the following:
//! 1. Device ID
//! 2. User ID
//! 3. Commands
//!     - Array writes
//!     - Register writes
//! 4. CRC32
//!
//! The device ID is a 32-bit word indicating the target FPGA device.
//!
//! The user ID is a 32-bit word. If not specified, the vendor tools default to `0x0000ffff`.
//! TODO: It can presumably be read out via JTAG on devices other than the AGRV2K, but it is useless on the AGRV2K.
//!
//! The CRC32 polynomial is `0x04c11db7`, specifically the [`CRC-32/BZIP2`](https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-32-bzip2) configuration.
//!
//! ## Registers
//!
//! There is only one known register, with address `2`. It is written with the value `0xf8f` at the end of the configuration process.
//! TODO: `DEV_OE` and `DEV_CLRn`
//!
//! ## Arrays
//!
//! Every other chunk of information is written to an array, which is identified by a "group" and "chain".
//!
//! For example, on the ARGV2K, group 1 chain 0 configures the IO pads and group 1 chain 1 configures the PLL.
//!
//! The "actual logic" is stored as one extremely-large array in group 0 chain 0.

use std::error;
use std::fmt::Display;
use std::io;

use crate::coordinates::{GlobalBitPos, TilePos, TileRelativeBitPos};
use crate::padring::PadRingExt;
use crate::tiles::io::IOTileCommonMut;
use crate::tiles::{TileRef, TileRefTrait, TileType};

use bitvec::prelude::*;

/// A bitstream control word, describing how to process the data that follows
///
/// The format of the control word is as follows:
/// - `bits[31:29]` - type (`101` for array data, `001` for a register access)
/// - `bits[27]` - indicates that this is the last frame
/// - `bits[25]` - indicates that this is a write (it is unknown where a read would output data to)
///
/// ## For array data
/// - `bits[9:5]` - indicates the config "group"
/// - `bits[4:0]` - indicates the config "chain"
///
/// This is then followed by:
/// - `word2[31:8]` - length in bits, minus 1
/// - `word2[7:4]` - "idle clocks" (it is unknown what precisely this does)
///
/// ## For a register access
/// - `bits[24:10]` - this is always 0x3f
/// - `bits[9:0]` - this is the register address
///
/// This is then followed by a 32-bit value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeaderWord(pub u32);
impl HeaderWord {
    pub const fn hdr_type(self) -> u32 {
        self.0 >> 29
    }
    pub const fn last_frame(self) -> bool {
        (self.0 & (1 << 27)) != 0
    }
    pub const fn config_group(self) -> u32 {
        (self.0 >> 5) & 0b11111
    }
    pub const fn config_chain(self) -> u32 {
        self.0 & 0b11111
    }
    pub const fn register(self) -> u32 {
        self.0 & 0b1111111111
    }

    pub const fn make_config_hdr(last_frame: bool, config_group: u32, config_chain: u32) -> Self {
        let x = (0b101 << 29)
            | (if last_frame { 1 << 27 } else { 0 })
            | (1 << 25)
            | (config_group << 5)
            | config_chain;
        Self(x)
    }
    pub const fn make_reg_write_hdr(last_frame: bool, reg_addr: u32) -> Self {
        let x = (0b001 << 29)
            | (if last_frame { 1 << 27 } else { 0 })
            | (1 << 25)
            | (0x3f << 10)
            | reg_addr;
        Self(x)
    }
}

/// A second word for config arrays, specifying the length
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConfigWord(pub u32);
impl ConfigWord {
    pub const fn bits(self) -> u32 {
        (self.0 >> 8) + 1
    }

    pub const fn make_config_word(bits: u32) -> Self {
        let x = ((bits - 1) << 8) | (2 << 4);
        Self(x)
    }
}

pub(crate) const BITSTREAM_CRC: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_BZIP2);

/// Errors that can arise while parsing a bitstream container
#[derive(Debug)]
#[non_exhaustive]
pub enum BitstreamContainerError {
    IoError(io::Error),
    InvalidDeviceID(u32),
    InvalidHeaderWord(u32),
    UnexpectedConfigData {
        group: u32,
        chain: u32,
    },
    UnexpectedConfigSize {
        group: u32,
        chain: u32,
        expected_bits: u32,
        have_bits: u32,
    },
    InvalidCRC {
        expected: u32,
        calculated: u32,
    },
}
impl Display for BitstreamContainerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::IoError(e) => e.fmt(f),
            Self::InvalidDeviceID(x) => {
                write!(f, "invalid or unsupported device ID 0x{:08x}", x)
            }
            Self::InvalidHeaderWord(x) => {
                write!(f, "header word 0x{:08x} is of invalid type", x)
            }
            Self::UnexpectedConfigData { group, chain } => {
                write!(f, "not expecting config group {} chain {}", group, chain)
            }
            Self::UnexpectedConfigSize {
                group,
                chain,
                expected_bits,
                have_bits,
            } => {
                write!(
                    f,
                    "wrong size for config group {} chain {}, expecting {} bits but got {} bits",
                    group, chain, expected_bits, have_bits
                )
            }
            BitstreamContainerError::InvalidCRC {
                expected,
                calculated,
            } => {
                write!(
                    f,
                    "invalid CRC, expecting 0x{:08x} but calculated 0x{:08x}",
                    expected, calculated
                )
            }
        }
    }
}
impl error::Error for BitstreamContainerError {
    fn cause(&self) -> Option<&dyn error::Error> {
        match self {
            Self::IoError(e) => Some(e),
            _ => None,
        }
    }
}
impl From<io::Error> for BitstreamContainerError {
    fn from(value: io::Error) -> Self {
        Self::IoError(value)
    }
}

/// Represents an in-memory FPGA bitstream
#[derive(Debug)]
pub struct Bitstream<D: DebugTracer = DummyDebugTracer> {
    family: crate::chips::Family,
    /// User ID value to identify the bitstream design
    pub user_id: u32,
    config_arrays: Vec<Vec<BitVec<u8, Msb0>>>,
    /// The debug tracer, if one has been provided
    pub debug_tracer: D,
}
impl Bitstream {
    pub fn new(family: crate::chips::Family) -> Self {
        Self::new_with_debug(family, DummyDebugTracer {})
    }

    pub fn read<R: io::Read>(r: R) -> Result<Self, BitstreamContainerError> {
        Self::read_with_debug(r, DummyDebugTracer {})
    }
}

fn wipe_io_tile(tile: &mut dyn IOTileCommonMut, io_i: u8) {
    tile.set_out_clock_choice(io_i, crate::tiles::io::IOClockMux::GND);
    tile.set_in_clock_choice(io_i, crate::tiles::io::IOClockMux::GND);
    tile.set_local_to_io_out(io_i, crate::tiles::io::LocalToIOMux::GND);
    tile.set_local_to_io_oe(io_i, crate::tiles::io::LocalToIOMux::GND);
    tile.set_local_to_out_clk_en(io_i, crate::tiles::io::LocalToIOMux::GND);
    tile.set_local_to_in_clk_en(io_i, crate::tiles::io::LocalToIOMux::GND);
    tile.set_local_to_async_ctrl(io_i, crate::tiles::io::LocalToIOMux::GND);
    tile.set_local_to_sync_ctrl(io_i, crate::tiles::io::LocalToIOMux::GND);
}

impl<D: DebugTracer> Bitstream<D> {
    pub fn new_with_debug(family: crate::chips::Family, debug_tracer: D) -> Self {
        let array_sizes = family.config_bits();
        let config_arrays = array_sizes
            .iter()
            .map(|chains| {
                chains
                    .iter()
                    .map(|&nbits| {
                        let len_in_bytes = crate::divroundup(nbits as u32, 32) as usize * 4;
                        let mut vec = Vec::new();
                        vec.resize(len_in_bytes, 0);
                        BitVec::from_vec(vec)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let mut ret = Self {
            family,
            user_id: 0xffff,
            config_arrays,
            debug_tracer,
        };

        match family {
            crate::chips::Family::AGRV2K => {
                // Fill a bunch of known all-1 bits

                // Big top-left MCU hole
                for y in 0..(22 + 7 * 68) {
                    for x in 0..(20 + 12 * 36) {
                        ret.set_logic_array_bit(GlobalBitPos { y, x }, true);
                    }
                }
                // Empty IOs above wide BRAM column
                for y in 0..22 {
                    for x in (20 + 12 * 36)..(20 + 12 * 36 + 180) {
                        ret.set_logic_array_bit(GlobalBitPos { y, x }, true);
                    }
                }
                // Top-right
                for y in 0..22 {
                    for x in (20 + 12 * 36 + 180 + 7 * 36)..(20 + 12 * 36 + 180 + 7 * 36 + 36) {
                        ret.set_logic_array_bit(GlobalBitPos { y, x }, true);
                    }
                }
                // Bottom-right
                for y in (22 + 12 * 68)..(22 + 12 * 68 + 22) {
                    for x in (20 + 12 * 36 + 180 + 7 * 36)..(20 + 12 * 36 + 180 + 7 * 36 + 36) {
                        ret.set_logic_array_bit(GlobalBitPos { y, x }, true);
                    }
                }

                // By default, apparently all IOs which _exist_ are driven with 0s on their signals
                for (pad_i, (io_tile_pos, io_i)) in
                    crate::padring::PADRING_TO_TILE.iter().enumerate()
                {
                    let tile = ret.tile_mut(*io_tile_pos).unwrap();
                    let mut tb_io;
                    let mut lr_io;
                    let io_tile: &mut dyn IOTileCommonMut = match tile.tile_type() {
                        TileType::TopBottomIO => {
                            tb_io = tile.as_topbottom_io_tile();
                            &mut tb_io
                        }
                        TileType::LeftRightIO => {
                            lr_io = tile.as_leftright_io_tile();
                            &mut lr_io
                        }
                        _ => unreachable!(),
                    };

                    wipe_io_tile(io_tile, *io_i);

                    // They apparently also default to a particular drive strength
                    ret.set_pad_drive_strength(
                        pad_i as u8,
                        crate::padring::DriveStrength::default(),
                    );
                }

                // These maybe-MIPI IOs are also driven low
                let tile = ret.tile_mut(TilePos { y: 0, x: 2 }).unwrap();
                let mut tile = tile.as_topbottom_io_tile();
                wipe_io_tile(&mut tile, 0);

                let tile = ret.tile_mut(TilePos { y: 0, x: 9 }).unwrap();
                let mut tile = tile.as_topbottom_io_tile();
                wipe_io_tile(&mut tile, 0);

                // The vendor tool clears these bits corresponding to BOOT0
                let tile = ret.tile_mut(TilePos { y: 0, x: 18 }).unwrap();
                let mut tile = tile.as_topbottom_io_tile();
                wipe_io_tile(&mut tile, 3);

                // Clear out bits to the MCU interface
                for x in 1..13 {
                    let tile = ret.tile_mut(TilePos { y: 5, x }).unwrap();
                    let mut tile = tile.as_top_ip_tile();

                    let num_pins_actually_used = if x == 4 { 9 } else { 8 };
                    for i in 0..num_pins_actually_used {
                        tile.set_to_ip(i, crate::tiles::hard_ip::Mux13Inv::GND);
                    }
                }
                for y in 5..13 {
                    let tile = ret.tile_mut(TilePos { y, x: 13 }).unwrap();
                    let mut tile = tile.as_leftright_ip_tile();
                    for i in 0..12 {
                        tile.set_to_ip_13(i, crate::tiles::hard_ip::Mux13Inv::GND);
                    }

                    let num_pins_actually_used = match y {
                        12 => 3,
                        11 => 7,
                        10 => 3,
                        9 => 3,
                        8 => 3,
                        7 => 2,
                        6 => 4,
                        5 => 4,
                        _ => unreachable!(),
                    };
                    for i in 0..num_pins_actually_used {
                        tile.set_to_ip_17(i, crate::tiles::hard_ip::Mux17Inv::GND);
                    }
                }

                // Clear out bits to PLL
                let tile = ret.tile_mut(TilePos { y: 5, x: 22 }).unwrap();
                let mut tile = tile.as_pll_tile();
                for i in 0..11 {
                    tile.set_to_pll(i, crate::tiles::hard_ip::Mux13Inv::GND);
                }
                // PLL analog settings
                tile.set_analog_icp(4);
                tile.set_analog_rlpf(1);
                tile.set_analog_rref(1);
                tile.set_analog_rvi(1);
                tile.set_analog_ivco(2);
            }
        }

        ret
    }

    pub fn read_with_debug<R: io::Read>(
        r: R,
        debug_tracer: D,
    ) -> Result<Self, BitstreamContainerError> {
        // Wrap the reader into something that _also_ updates CRC along the way
        struct CRCReader<'a, R: io::Read> {
            r: R,
            crc: crc::Digest<'a, u32>,
        }
        impl<'a, R: io::Read> CRCReader<'a, R> {
            /// Read a block of data, updating the CRC
            fn get_data(&mut self, x: &mut [u8]) -> io::Result<()> {
                self.r.read_exact(x)?;
                self.crc.update(x);
                Ok(())
            }

            /// Read a block of data into a Vec
            fn get_vec(&mut self, l: usize) -> io::Result<Vec<u8>> {
                let mut ret = vec![0; l];
                self.get_data(&mut ret)?;
                Ok(ret)
            }

            /// Read a *big* endian u32, update the CRC
            fn get_u32(&mut self) -> io::Result<u32> {
                let mut ret = [0; 4];
                self.get_data(&mut ret)?;
                Ok(u32::from_be_bytes(ret))
            }
        }
        let mut r = CRCReader {
            r,
            crc: BITSTREAM_CRC.digest(),
        };

        let device_id = r.get_u32()?;
        let user_id = r.get_u32()?;
        log::debug!("device id 0x{device_id:08x}, user id 0x{user_id:08x}");

        let family = crate::chips::Family::try_from(device_id)
            .map_err(|_| BitstreamContainerError::InvalidDeviceID(device_id))?;
        let array_sizes = family.config_bits();

        // Pre-fill config array vecs with empty vecs
        let mut ret = Self {
            family,
            user_id,
            config_arrays: Vec::with_capacity(array_sizes.len()),
            debug_tracer,
        };
        for szs in array_sizes {
            ret.config_arrays.push(Vec::with_capacity(szs.len()));
        }
        for (i, szs) in array_sizes.iter().enumerate() {
            for _ in szs.iter() {
                ret.config_arrays[i].push(BitVec::new());
            }
        }

        loop {
            let hdr = HeaderWord(r.get_u32()?);
            log::debug!("got header word 0x{:08x}", hdr.0);

            match hdr.hdr_type() {
                0b101 => {
                    let config_group = hdr.config_group();
                    let config_chain = hdr.config_chain();
                    let config_word = ConfigWord(r.get_u32()?);
                    let len_in_bits = config_word.bits();
                    log::debug!(
                        "{} bits for config group {} chain {}",
                        len_in_bits,
                        config_group,
                        config_chain
                    );

                    // Validate that this config group/chain are as expected
                    if config_group as usize >= array_sizes.len() {
                        return Err(BitstreamContainerError::UnexpectedConfigData {
                            group: config_group,
                            chain: config_chain,
                        });
                    }
                    if config_chain as usize >= array_sizes[config_group as usize].len() {
                        return Err(BitstreamContainerError::UnexpectedConfigData {
                            group: config_group,
                            chain: config_chain,
                        });
                    }
                    let expected_bits = array_sizes[config_group as usize][config_chain as usize];
                    if len_in_bits as usize != expected_bits {
                        return Err(BitstreamContainerError::UnexpectedConfigSize {
                            group: config_group,
                            chain: config_chain,
                            expected_bits: expected_bits as u32,
                            have_bits: len_in_bits,
                        });
                    }

                    let len_in_bytes = crate::divroundup(len_in_bits, 32) as usize * 4;
                    let array_data = BitVec::try_from_vec(r.get_vec(len_in_bytes)?).unwrap();
                    let _ = std::mem::replace(
                        &mut ret.config_arrays[config_group as usize][config_chain as usize],
                        array_data,
                    );
                }
                0b001 => {
                    let register = hdr.register();
                    let reg_value = r.get_u32()?;
                    log::debug!("reg 0x{:x} = 0x{:08x}", register, reg_value);

                    // TODO: DEV_CLRn and DEV_OE functionality exists here, tbd
                    if register != 2 || (register == 2 && reg_value != 0xf8f) {
                        log::warn!(
                            "Unknown register write 0x{:x} = 0x{:08x}",
                            register,
                            reg_value
                        );
                    }
                }
                _ => return Err(BitstreamContainerError::InvalidHeaderWord(hdr.0)),
            }

            if hdr.last_frame() {
                break;
            }
        }

        let crc_computed = r.crc.finalize();
        let mut crc_expected = [0; 4];
        r.r.read_exact(&mut crc_expected)?;
        let crc_expected = u32::from_be_bytes(crc_expected);
        if crc_computed != crc_expected {
            return Err(BitstreamContainerError::InvalidCRC {
                expected: crc_expected,
                calculated: crc_computed,
            });
        }

        Ok(ret)
    }

    pub fn save<W: io::Write>(&self, w: W) -> io::Result<()> {
        // Wrap the write into something that _also_ updates CRC along the way
        struct CRCWriter<'a, W: io::Write> {
            w: W,
            crc: crc::Digest<'a, u32>,
        }
        impl<'a, W: io::Write> CRCWriter<'a, W> {
            /// Write a block of data, updating the CRC
            fn put_data(&mut self, x: &[u8]) -> io::Result<()> {
                self.crc.update(x);
                self.w.write_all(x)?;
                Ok(())
            }

            /// Put a *big* endian u32, update the CRC
            fn put_u32(&mut self, x: u32) -> io::Result<()> {
                self.put_data(&x.to_be_bytes())
            }
        }
        let mut w = CRCWriter {
            w,
            crc: BITSTREAM_CRC.digest(),
        };

        // IDs
        w.put_u32(self.family.device_id())?;
        w.put_u32(self.user_id)?;

        // XXX maybe generalize this later?
        let array_sizes = self.family.config_bits();
        let mut put_cfg_array = |group, chain| -> io::Result<()> {
            log::debug!("writing config group {group} chain {chain}");
            let hdr = HeaderWord::make_config_hdr(false, group, chain);
            w.put_u32(hdr.0)?;

            let len_in_bits = array_sizes[group as usize][chain as usize] as u32;
            let cfgw = ConfigWord::make_config_word(len_in_bits);
            w.put_u32(cfgw.0)?;

            let len_in_bytes = crate::divroundup(len_in_bits, 32) as usize * 4;
            w.put_data(
                &self.config_arrays[group as usize][chain as usize].as_raw_slice()[..len_in_bytes],
            )?;
            Ok(())
        };
        match self.family {
            crate::chips::Family::AGRV2K => {
                // IO
                put_cfg_array(1, 0)?;

                // PLLs
                put_cfg_array(1, 1)?;
            }
        }

        // main logic array
        put_cfg_array(0, 0)?;

        // TODO: DEV_OE etc
        let hdr = HeaderWord::make_reg_write_hdr(true, 2);
        w.put_u32(hdr.0)?;
        w.put_u32(0xf8f)?;

        // CRC
        let crc = w.crc.finalize().to_be_bytes();
        w.w.write_all(&crc)?;

        Ok(())
    }

    pub const fn family(&self) -> crate::chips::Family {
        self.family
    }

    #[inline]
    pub fn get_aux_array_bit(&self, group: u32, chain: u32, biti: usize) -> bool {
        self.config_arrays[group as usize][chain as usize][biti]
    }
    #[inline]
    pub fn set_aux_array_bit(&mut self, group: u32, chain: u32, biti: usize, val: bool) {
        self.config_arrays[group as usize][chain as usize].set(biti, val);
    }
    #[inline]
    pub fn get_aux_array_bits(&self, group: u32, chain: u32, bits: std::ops::Range<usize>) -> u32 {
        assert!(bits.len() <= 32, "bitfield cannot use >32 bits");
        let mut ret: BitArr!(for 32, in u32, Lsb0) = BitArray::ZERO;
        ret[..bits.len()]
            .clone_from_bitslice(&self.config_arrays[group as usize][chain as usize][bits]);
        ret.into_inner()[0]
    }
    #[inline]
    pub fn set_aux_array_bits(
        &mut self,
        group: u32,
        chain: u32,
        bits: std::ops::Range<usize>,
        val: u32,
    ) {
        assert!(bits.len() <= 32, "bitfield cannot use >32 bits");
        let val = &BitSlice::<_, Lsb0>::from_element(&val)[..bits.len()];
        self.config_arrays[group as usize][chain as usize][bits].clone_from_bitslice(&val);
    }

    #[inline]
    pub fn get_logic_array_bit(&self, bit: GlobalBitPos) -> bool {
        let (w, h) = self.family.main_logic_bits();
        assert!(bit.x < w && bit.y < h);
        let real_w = crate::divroundup(w, 32) * 32;
        self.config_arrays[0][0]
            [bit.y as usize * real_w as usize + (w as usize - 1 - bit.x as usize)]
    }
    #[inline]
    pub(crate) fn debug_log_access(
        &self,
        global_bit_pos: GlobalBitPos,
        tile_pos: TilePos,
        tile_relative_pos: TileRelativeBitPos,
        field: &dyn std::fmt::Debug,
    ) {
        self.debug_tracer
            .log_coordinate_access(global_bit_pos, tile_pos, tile_relative_pos, field);
    }
    #[inline]
    pub fn set_logic_array_bit(&mut self, bit: GlobalBitPos, val: bool) {
        let (w, h) = self.family.main_logic_bits();
        assert!(bit.x < w && bit.y < h);
        let real_w = crate::divroundup(w, 32) * 32;
        self.config_arrays[0][0].set(
            bit.y as usize * real_w as usize + (w as usize - 1 - bit.x as usize),
            val,
        );
    }

    #[inline]
    pub fn tile(&self, pos: TilePos) -> Option<TileRef<D, &Self>> {
        if self.family.get_tile_type(pos) == TileType::None {
            return None;
        }
        Some(TileRef::new(self, pos))
    }
    #[inline]
    pub fn tile_mut(&mut self, pos: TilePos) -> Option<TileRef<D, &mut Self>> {
        if self.family.get_tile_type(pos) == TileType::None {
            return None;
        }
        Some(TileRef::new(self, pos))
    }
}

/// Trait which logs accesses to bitstream bits
///
/// This can be used to help verify that all bits are actually being read,
/// no duplicate bits are being hit, or to generate pretty tables.
pub trait DebugTracer {
    fn log_coordinate_access(
        &self,
        global_bit_pos: GlobalBitPos,
        tile_pos: TilePos,
        tile_relative_pos: TileRelativeBitPos,
        field: &dyn std::fmt::Debug,
    );
}

/// Default debug tracer which doesn't do anything
pub struct DummyDebugTracer {}
impl DebugTracer for DummyDebugTracer {
    #[inline(always)]
    fn log_coordinate_access(
        &self,
        _: GlobalBitPos,
        _: TilePos,
        _: TileRelativeBitPos,
        _: &dyn std::fmt::Debug,
    ) {
    }
}
