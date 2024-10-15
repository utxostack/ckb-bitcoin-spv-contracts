use alloc::vec::Vec;

use ckb_bitcoin_spv_verifier::types::{
    core::{SpvClient, SpvInfo},
    packed::{self, SpvClientReader, SpvInfoReader, SpvUpdateReader},
    prelude::*,
};
#[cfg(debug_assertions)]
use ckb_std::ckb_types::prelude::Pack as StdPack;
use ckb_std::{ckb_constants::Source, debug, error::SysError, high_level as hl};

use crate::{
    error::{InternalError, Result},
    utilities,
};

pub(crate) fn update_client(
    inputs: (usize, usize),
    outputs: (usize, usize),
    script_hash: &[u8],
) -> Result<()> {
    // Checks the id of the input client cell, then returns
    // - expected output info cell base on the input info cell,
    // - the tip client id.
    // - the expected client id, which will be the next tip client id.
    let (expected_info, tip_client_id, expected_client_id, flags) = {
        let (mut input_info, tip_client_id, expected_client_id, flags) = load_inputs(inputs)?;
        input_info.tip_client_id = expected_client_id;
        (input_info, tip_client_id, expected_client_id, flags)
    };
    // Checks the output info cell, then returns the client cell and the index of the info cell.
    let (output_client, output_info_index) = load_outputs(outputs, &expected_info)?;
    // Finds the only one index of cell deps which use current script.
    // That cell should be the current tip client.
    let cell_dep_index = find_cell_dep(script_hash)?;
    // Checks the id of the cell-dep client cell, then returns
    // the expected input client cell base on the cell-dep client cell,
    let expected_input_client = {
        let mut cell_dep_client = load_cell_dep(cell_dep_index, tip_client_id)?;
        cell_dep_client.id = expected_client_id;
        cell_dep_client.pack()
    };
    // Gets the update from the witness.
    let update = {
        let witness_args = hl::load_witness_args(output_info_index, Source::Output)?;
        if let Some(args) = witness_args.output_type().to_opt() {
            SpvUpdateReader::from_slice(&args.raw_data())
                .map_err(|_| SysError::Encoding)?
                .to_entity()
        } else {
            return Err(InternalError::UpdateWitnessIsNotExisted.into());
        }
    };

    expected_input_client.verify_new_client(&output_client, update, flags)?;

    Ok(())
}

fn load_inputs(inputs: (usize, usize)) -> Result<(SpvInfo, u8, u8, u8)> {
    debug!("load cell data of inputs[{}]", inputs.0);
    let input_data_0 = hl::load_cell_data(inputs.0, Source::Input)?;
    debug!("load cell data of inputs[{}]", inputs.1);
    let input_data_1 = hl::load_cell_data(inputs.1, Source::Input)?;

    let (packed_input_info, packed_input_client) =
        if let Ok(input_info) = SpvInfoReader::from_slice(&input_data_0) {
            debug!("input info = {input_info} (index={})", inputs.0);
            if let Ok(input_client) = SpvClientReader::from_slice(&input_data_1) {
                debug!("input client = {input_client} (index={})", inputs.1);
                (input_info, input_client)
            } else {
                return Err(InternalError::UpdateInputClientNotFound.into());
            }
        } else if let Ok(input_info) = SpvInfoReader::from_slice(&input_data_1) {
            debug!("input info = {input_info} (index={})", inputs.1);
            if let Ok(input_client) = SpvClientReader::from_slice(&input_data_0) {
                debug!("input client = {input_client} (index={})", inputs.0);
                (input_info, input_client)
            } else {
                return Err(InternalError::UpdateInputClientNotFound.into());
            }
        } else {
            return Err(InternalError::UpdateInputInfoNotFound.into());
        };

    let input_info: SpvInfo = packed_input_info.unpack();
    let tip_client_id = input_info.tip_client_id;
    debug!("tip client id = {tip_client_id}");
    let input_client_id: u8 = packed_input_client.id().into();
    debug!("input client id = {input_client_id}");

    let (clients_count, flags) = {
        let type_args = utilities::load_spv_type_args()?;
        (type_args.clients_count, type_args.flags)
    };
    debug!("clients count: {clients_count}, flags: {flags:08b}");

    let expected_client_id = utilities::next_client_id(input_info.tip_client_id, clients_count);
    debug!("expected client id = {expected_client_id}");
    if input_client_id != expected_client_id {
        return Err(InternalError::UpdateInputClientIdIsMismatch.into());
    }

    Ok((input_info, tip_client_id, expected_client_id, flags))
}

fn load_outputs(
    outputs: (usize, usize),
    expected_info: &SpvInfo,
) -> Result<(packed::SpvClient, usize)> {
    debug!("load cell data of outputs[{}]", outputs.0);
    let output_data_0 = hl::load_cell_data(outputs.0, Source::Output)?;
    debug!("load cell data of outputs[{}]", outputs.1);
    let output_data_1 = hl::load_cell_data(outputs.1, Source::Output)?;

    let (packed_output_info, packed_output_client, output_info_index) =
        if let Ok(output_info) = SpvInfoReader::from_slice(&output_data_0) {
            debug!("output info = {output_info} (index={})", outputs.0);
            if let Ok(output_client) = SpvClientReader::from_slice(&output_data_1) {
                debug!("output client = {output_client} (index={})", outputs.1);
                (output_info, output_client, outputs.0)
            } else {
                return Err(InternalError::UpdateOutputClientNotFound.into());
            }
        } else if let Ok(output_info) = SpvInfoReader::from_slice(&output_data_1) {
            debug!("output info = {output_info} (index={})", outputs.1);
            if let Ok(output_client) = SpvClientReader::from_slice(&output_data_0) {
                debug!("output client = {output_client} (index={})", outputs.0);
                (output_info, output_client, outputs.1)
            } else {
                return Err(InternalError::UpdateOutputClientNotFound.into());
            }
        } else {
            return Err(InternalError::UpdateOutputInfoNotFound.into());
        };

    let packed_expected_info = expected_info.pack();
    debug!("expected info = {packed_expected_info}");
    if packed_output_info.as_slice() != packed_expected_info.as_slice() {
        return Err(InternalError::UpdateOutputInfoChanged.into());
    }

    Ok((packed_output_client.to_entity(), output_info_index))
}

fn find_cell_dep(script_hash: &[u8]) -> Result<usize> {
    let mut indexes = Vec::new();
    for (index, type_hash_opt) in
        hl::QueryIter::new(hl::load_cell_type_hash, Source::CellDep).enumerate()
    {
        if let Some(type_hash) = type_hash_opt {
            debug!(
                "{index}-th type hash of cell-deps: {:#x}",
                StdPack::pack(&type_hash)
            );
            if type_hash == script_hash {
                if indexes.is_empty() {
                    indexes.push(index);
                } else {
                    return Err(InternalError::UpdateCellDepMoreThanOne.into());
                }
            }
        }
    }
    if indexes.is_empty() {
        return Err(InternalError::UpdateCellDepNotFound.into());
    }
    Ok(indexes[0])
}

fn load_cell_dep(cell_dep_index: usize, tip_client_id: u8) -> Result<SpvClient> {
    debug!("load cell data of cell deps[{cell_dep_index}]");
    let cell_dep_data = hl::load_cell_data(cell_dep_index, Source::CellDep)?;

    let packed_cell_dep_client =
        if let Ok(cell_dep_client) = SpvClientReader::from_slice(&cell_dep_data) {
            debug!("cell-dep client = {cell_dep_client} (index={cell_dep_index})");
            cell_dep_client
        } else {
            return Err(InternalError::UpdateCellDepClientNotFound.into());
        };

    let cell_dep_client: SpvClient = packed_cell_dep_client.unpack();
    debug!("cell-dep client id = {}", cell_dep_client.id);
    if cell_dep_client.id != tip_client_id {
        return Err(InternalError::UpdateCellDepClientIdIsMismatch.into());
    }
    Ok(cell_dep_client)
}
