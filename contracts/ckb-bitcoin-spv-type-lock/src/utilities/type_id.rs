use ckb_bitcoin_spv_verifier::types::prelude::*;
use ckb_hash::{new_blake2b, BLAKE2B_LEN};
use ckb_std::{ckb_constants::Source, high_level as hl};

use crate::error::Result;

pub(crate) fn calculate_type_id(outputs_count: usize) -> Result<[u8; BLAKE2B_LEN]> {
    let input = hl::load_input(0, Source::Input)?;
    let mut blake2b = new_blake2b();
    blake2b.update(input.as_slice());
    blake2b.update(&(outputs_count as u64).to_le_bytes());
    let mut ret = [0; BLAKE2B_LEN];
    blake2b.finalize(&mut ret);
    Ok(ret)
}
