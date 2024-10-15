mod type_id;

use ckb_bitcoin_spv_verifier::types::{core::SpvTypeArgs, packed::SpvTypeArgsReader, prelude::*};
use ckb_std::{error::SysError, high_level as hl};

use crate::error::Result;

pub(crate) use self::type_id::load_then_calculate_type_id;

pub(crate) fn prev_client_id(current: u8, count: u8) -> u8 {
    if current == 0 {
        count - 1
    } else {
        current - 1
    }
}

pub(crate) fn next_client_id(current: u8, count: u8) -> u8 {
    if current + 1 < count {
        current + 1
    } else {
        0
    }
}

pub(crate) fn load_spv_type_args() -> Result<SpvTypeArgs> {
    let script = hl::load_script()?;
    let script_args = script.args();
    let script_args_slice = script_args.as_reader().raw_data();
    let args = SpvTypeArgsReader::from_slice(script_args_slice)
        .map_err(|_| SysError::Encoding)?
        .unpack();
    Ok(args)
}
