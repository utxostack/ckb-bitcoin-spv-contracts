use core::result;

use ckb_std::error::SysError;

pub type Result<T> = result::Result<T, Error>;

#[repr(i8)]
pub enum Error {
    // 0x01 ~ 0x0f: Errors from SDK, or other system errors.
    IndexOutOfBound = 0x01,
    ItemMissing,
    LengthNotEnough,
    Encoding,
    Unknown,

    // 0x10 ~ 0x2f: Errors in current crate.
    ShouldNotBeType = 0x10,
    WitnessIsIncorrect,
    InputsCapacityOverflow,
    OutputsCapacityOverflow,
    LostCapacityWithoutOwnership,
}

impl From<SysError> for Error {
    fn from(err: SysError) -> Self {
        use SysError::*;
        match err {
            IndexOutOfBound => Self::IndexOutOfBound,
            ItemMissing => Self::ItemMissing,
            LengthNotEnough(_) => Self::LengthNotEnough,
            Encoding => Self::Encoding,
            Unknown(_) => Self::Unknown,
        }
    }
}
