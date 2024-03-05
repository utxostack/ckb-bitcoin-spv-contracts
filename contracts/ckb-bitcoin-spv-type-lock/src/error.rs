use core::result;

use ckb_bitcoin_spv_verifier::error::{BootstrapError, UpdateError};
use ckb_std::error::SysError;

pub type Result<T> = result::Result<T, Error>;

#[repr(i8)]
pub enum InternalError {
    // 0x01 ~ 0x0f: Errors from SDK, or other system errors.
    IndexOutOfBound = 0x01,
    ItemMissing,
    LengthNotEnough,
    Encoding,
    Unknown,

    // 0x10 ~ 0x1f: Errors before doing operations.
    UnknownOperation = 0x10,

    // 0x20 ~ 0x37: Errors when create.
    CreateNotEnoughCells = 0x20,
    CreateShouldBeOrdered,
    CreateCellsCountNotMatched,
    CreateIncorrectUniqueId,
    CreateBadInfoCellData,
    CreateInfoIndexShouldBeZero,
    CreateWitnessIsNotExisted,
    CreateBadClientCellData,
    CreateNewClientIsIncorrect,

    // 0x38 ~ 0x3f: Errors when destroy.
    DestroyNotEnoughCells = 0x3f,

    // 0x40 ~ 0x4f: Errors when update.
    UpdateInputInfoNotFound = 0x40,
    UpdateInputClientNotFound,
    UpdateInputClientIdIsMismatch,
    UpdateOutputInfoNotFound,
    UpdateOutputClientNotFound,
    UpdateOutputInfoChanged,
    UpdateCellDepMoreThanOne,
    UpdateCellDepNotFound,
    UpdateCellDepClientNotFound,
    UpdateCellDepClientIdIsMismatch,
    UpdateWitnessIsNotExisted,

    // 0x50 ~ 0x5f: Errors when reorg.
    ReorgFailed = 0x50,
}

pub enum Error {
    // 0x01 ~ 0x5f: Errors that not from external crates.
    Internal(InternalError),
    // 0x60 ~ 0x7f: Errors when bootstrap or apply the update.
    //
    // Different steps may have same error codes.
    Bootstrap(BootstrapError),
    Update(UpdateError),
}

impl From<SysError> for InternalError {
    fn from(err: SysError) -> Self {
        match err {
            SysError::IndexOutOfBound => Self::IndexOutOfBound,
            SysError::ItemMissing => Self::ItemMissing,
            SysError::LengthNotEnough(_) => Self::LengthNotEnough,
            SysError::Encoding => Self::Encoding,
            SysError::Unknown(_) => Self::Unknown,
        }
    }
}

impl From<SysError> for Error {
    fn from(err: SysError) -> Self {
        Into::<InternalError>::into(err).into()
    }
}

impl From<InternalError> for Error {
    fn from(err: InternalError) -> Self {
        Self::Internal(err)
    }
}

impl From<BootstrapError> for Error {
    fn from(err: BootstrapError) -> Self {
        Self::Bootstrap(err)
    }
}

impl From<UpdateError> for Error {
    fn from(err: UpdateError) -> Self {
        Self::Update(err)
    }
}

impl From<Error> for i8 {
    fn from(err: Error) -> Self {
        match err {
            Error::Internal(e) => e as i8,
            Error::Bootstrap(e) => 0x60 + e as i8,
            Error::Update(e) => 0x60 + e as i8,
        }
    }
}
