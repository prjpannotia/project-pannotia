//! Database of factoids about chip designs

use std::fmt;

/// Represents the "family" of the FPGA being worked on
///
/// This represents a unique bitstream format and die layout,
/// but it abstracts over details such as pinouts and mechanical packages.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Family {
    /// AGRV2K CPLDs and AG32V microcontrollers
    AGRV2K,
}
impl fmt::Display for Family {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Family {
    /// Get the device ID code for this family
    pub const fn device_id(self) -> u32 {
        match self {
            Family::AGRV2K => 0x40200001,
        }
    }

    /// Return the sizes of all the configuration arrays
    pub const fn config_bits(self) -> &'static [&'static [usize]] {
        match self {
            Family::AGRV2K => &[
                &[860 * 928], // group 0 chain 0 (main array)
                &[
                    834, // group 1 chain 0 (IO)
                    239, // group 1 chain 1 (PLL)
                ],
            ],
        }
    }
}
/// Try converting a device ID to a chip family
impl TryFrom<u32> for Family {
    type Error = ();
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0x40200001 => Ok(Self::AGRV2K),
            _ => Err(()),
        }
    }
}
