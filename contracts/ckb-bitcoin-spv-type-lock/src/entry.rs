use alloc::vec::Vec;

#[cfg(debug_assertions)]
use ckb_std::ckb_types::prelude::*;
use ckb_std::{ckb_constants::Source, debug, high_level as hl};

use crate::{
    error::{InternalError, Result},
    operations,
};

pub fn main() -> Result<()> {
    debug!("{} Starting ...", module_path!());

    let script_hash = hl::load_script_hash()?;
    debug!("script hash = {:#x}", script_hash.pack());

    // Find all input cells which use current script.
    let indexes_of_inputs = {
        let mut indexes = Vec::new();
        for (index, type_hash_opt) in
            hl::QueryIter::new(hl::load_cell_type_hash, Source::Input).enumerate()
        {
            if let Some(type_hash) = type_hash_opt {
                debug!("{index}-th type hash of inputs: {:#x}", type_hash.pack());
                if type_hash == script_hash {
                    debug!("found cell: inputs[{index}]");
                    indexes.push(index);
                }
            }
        }
        indexes
    };

    // Find all output cells which use current script.
    let indexes_of_outputs = {
        let mut indexes = Vec::new();
        for (index, type_hash_opt) in
            hl::QueryIter::new(hl::load_cell_type_hash, Source::Output).enumerate()
        {
            if let Some(type_hash) = type_hash_opt {
                debug!("{index}-th type hash of outputs: {:#x}", type_hash.pack());
                if type_hash == script_hash {
                    debug!("found cell: outputs[{index}]");
                    indexes.push(index);
                }
            }
        }
        indexes
    };

    debug!("cells in  inputs: {indexes_of_inputs:?}");
    debug!("cells in outputs: {indexes_of_outputs:?}");

    match (indexes_of_inputs.len(), indexes_of_outputs.len()) {
        (0, _) => {
            debug!("create all cells");
            operations::create_cells(&indexes_of_outputs)?;
        }
        // TODO Disable destroy operations in first version.
        // (_, 0) => {
        //     debug!("destroy all cells");
        //     operations::destroy_cells(&indexes_of_inputs)?;
        // }
        (2, 2) => {
            debug!("update a client cell and the info cell");
            operations::update_client(
                (indexes_of_inputs[0], indexes_of_inputs[1]),
                (indexes_of_outputs[0], indexes_of_outputs[1]),
                script_hash.as_slice(),
            )?;
        }
        (m, n) if m == n && m > 2 => {
            debug!("reorg client cells");
            operations::reorg_clients(
                &indexes_of_inputs,
                &indexes_of_outputs,
                script_hash.as_slice(),
            )?;
        }
        (_m, _n) => {
            debug!("unknown operation: {_m} inputs and {_n} outputs");
            return Err(InternalError::UnknownOperation.into());
        }
    }

    debug!("{} DONE.", module_path!());

    Ok(())
}
