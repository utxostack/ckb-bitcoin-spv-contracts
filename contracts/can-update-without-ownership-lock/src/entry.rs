use alloc::vec::Vec;

#[cfg(debug_assertions)]
use ckb_std::ckb_types::prelude::*;
use ckb_std::{ckb_constants::Source, debug, high_level as hl};

use crate::error::{Error, Result};

pub fn main() -> Result<()> {
    debug!("{} Starting ...", module_path!());

    let script_hash = hl::load_script_hash()?;
    debug!("script hash = {:#x}", script_hash.pack());
    let args = hl::load_script()?.args();
    let expected_witness = args.raw_data().iter().cloned().rev().collect::<Vec<u8>>();

    let mut checked = false;

    // Indexes of all inputs which use this lock.
    let mut inputs_indexes = Vec::new();

    for (index, lock_hash) in hl::QueryIter::new(hl::load_cell_lock_hash, Source::Input).enumerate()
    {
        if lock_hash == script_hash {
            debug!(">>> checking input {index}");
            inputs_indexes.push(index);
        } else {
            debug!(
                ">>> skipping input {index} since it's lock is {:#x}",
                lock_hash.pack()
            );
            continue;
        }

        if let Ok(witness_args) = hl::load_witness_args(index, Source::Input) {
            if let Some(lock_witness) = witness_args.lock().to_opt() {
                if lock_witness.raw_data().as_ref() == expected_witness {
                    checked = true;
                    debug!(">>> >>> passed to check witness");
                } else {
                    debug!(
                        ">>> >>> failed to check witness: args: {:#x}, witness: {:#x}",
                        args, lock_witness
                    );
                    return Err(Error::WitnessIsIncorrect);
                }
            } else {
                debug!(">>> >>> failed to check witness: witness is empty");
            }
        } else {
            debug!(">>> >>> failed to load {index}-th witness");
        }
    }
    debug!("checked: {checked}");

    if !checked {
        debug!("calculating inputs capacity ...");
        let total_inputs_capacity = inputs_indexes.into_iter().try_fold(0u64, |total, index| {
            let added = hl::load_cell_capacity(index, Source::Input)?;
            let (tmp, of) = total.overflowing_add(added);
            debug!(">>> total = {tmp} (index: {index}, added: {added}, overflow: {of})");
            if of {
                Err(Error::InputsCapacityOverflow)
            } else {
                Ok(tmp)
            }
        })?;
        debug!("calculating outputs capacity ...");
        let total_outputs_capacity = hl::QueryIter::new(hl::load_cell_lock_hash, Source::Output)
            .enumerate()
            .try_fold(0u64, |total, (index, lock_hash)| {
                if lock_hash == script_hash {
                    debug!(">>> checking output {index}");
                } else {
                    debug!(
                        ">>> skipping output {index} since it's lock is {:#x}",
                        lock_hash.pack()
                    );
                    return Ok(total);
                }

                let added = hl::load_cell_capacity(index, Source::Output)?;
                let (tmp, of) = total.overflowing_add(added);
                debug!(">>> >>> total = {tmp} (index: {index}, added: {added}, overflow: {of})");
                if of {
                    Err(Error::OutputsCapacityOverflow)
                } else {
                    Ok(tmp)
                }
            })?;
        if total_inputs_capacity > total_outputs_capacity {
            debug!("lost capacity without ownership ({total_inputs_capacity} -> {total_outputs_capacity})");
            return Err(Error::LostCapacityWithoutOwnership);
        }
    }

    for (_index, type_hash_opt) in
        hl::QueryIter::new(hl::load_cell_type_hash, Source::Output).enumerate()
    {
        if let Some(type_hash) = type_hash_opt {
            if type_hash == script_hash {
                debug!("output {_index} uses current lock as type");
                return Err(Error::ShouldNotBeType);
            }
        }
    }

    debug!("{} DONE.", module_path!());

    Ok(())
}
