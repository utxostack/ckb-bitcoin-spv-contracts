use alloc::vec::Vec;

use ckb_bitcoin_spv_verifier::types::{
    core::{BitcoinChainType, SpvClient, SpvInfo, U256},
    packed::{self, SpvClientReader, SpvInfoReader, SpvTypeArgsReader, SpvUpdateReader},
    prelude::*,
};
#[cfg(debug_assertions)]
use ckb_std::ckb_types::prelude::Pack as StdPack;
use ckb_std::{ckb_constants::Source, debug, error::SysError, high_level as hl};

use crate::{
    error::{InternalError, Result},
    utilities,
};

pub(crate) fn reorg_clients(inputs: &[usize], outputs: &[usize], script_hash: &[u8]) -> Result<()> {
    // Checks the ids of the input client cells, then returns
    // - expected output info cell base on the input info cell,
    // - the new tip client id.
    // - the expected client ids, which will be the new tip client id and the ids of all the cleared clients.
    // - the previous chain work of the old tip client.
    // - the id of the last client, whose blocks are all in main chain.
    // - the flags in SPV script args
    let (
        expected_info,
        expected_tip_client_id,
        expected_client_ids,
        previous_chain_work,
        fork_client_id,
        flags,
    ) = {
        let (
            mut input_info,
            expected_tip_client_id,
            expected_client_ids,
            previous_chain_work,
            fork_client_id,
            flags,
        ) = load_inputs(inputs)?;
        input_info.tip_client_id = expected_tip_client_id;
        (
            input_info,
            expected_tip_client_id,
            expected_client_ids,
            previous_chain_work,
            fork_client_id,
            flags,
        )
    };
    // Checks the output info cell and the output client cells;
    // then returns new tip client and the index of the info cell.
    let (output_client, output_info_index) =
        load_outputs(outputs, &expected_info, expected_client_ids)?;
    {
        // Due to the block storm issue on testnet 3, a large number of blocks may be rolled back
        // during a reorg, making it necessary to limit the update height.
        // If there is a limit on the number of headers to update,
        // the current chain work might not be sufficient but still remain on the main chain.
        // Therefore, in this case, we no longer check the chain work.
        // This handling is specific to testnet 3 to address the frequent block storm reorgs.
        if BitcoinChainType::Testnet != flags.into() {
            let new_chain_work: U256 = output_client
                .headers_mmr_root()
                .partial_chain_work()
                .unpack();
            if previous_chain_work >= new_chain_work {
                return Err(InternalError::ReorgNotBetterChain.into());
            }
        }
    }
    // Finds the only one index of cell deps which use current script.
    // That cell should be the client which at the fork point.
    let cell_dep_index = find_cell_dep(script_hash)?;
    // Checks the id of the cell-dep client cell, then returns
    // the expected input client cell base on the cell-dep client cell,
    let expected_input_client = {
        let mut cell_dep_client = load_cell_dep(cell_dep_index, fork_client_id)?;
        cell_dep_client.id = expected_tip_client_id;
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

fn load_inputs(inputs: &[usize]) -> Result<(SpvInfo, u8, Vec<u8>, U256, u8, u8)> {
    let mut client_ids_with_indexes = Vec::new();
    let mut input_info_opt = None;
    for i in inputs {
        debug!("load cell data of inputs[{i}]");
        let input_data = hl::load_cell_data(*i, Source::Input)?;
        if let Ok(packed_input_info) = SpvInfoReader::from_slice(&input_data) {
            debug!("input info = {packed_input_info} (index={i})");
            if input_info_opt.is_some() {
                return Err(InternalError::ReorgInputInfoDuplicated.into());
            }
            let input_info: SpvInfo = packed_input_info.unpack();
            input_info_opt = Some(input_info);
        } else if let Ok(packed_input_client) = SpvClientReader::from_slice(&input_data) {
            debug!("input client = {packed_input_client} (index={i})");
            let input_client_id: u8 = packed_input_client.id().into();
            debug!("input client id = {input_client_id}");
            client_ids_with_indexes.push((*i, input_client_id));
        } else {
            return Err(InternalError::ReorgInputMalformed.into());
        }
    }

    if input_info_opt.is_none() {
        return Err(InternalError::ReorgInputInfoNotFound.into());
    }
    let input_info = input_info_opt.unwrap();
    let tip_client_id = input_info.tip_client_id;
    debug!("tip client id = {tip_client_id}");

    if client_ids_with_indexes.len() < 2 {
        return Err(InternalError::ReorgInputClientNotEnough.into());
    }
    debug!(
        "input client ids with indexes = {:?}",
        client_ids_with_indexes
    );

    let tip_client_index = client_ids_with_indexes
        .iter()
        .find(|(_, id)| *id == tip_client_id)
        .map(|(index, _)| *index)
        .ok_or(InternalError::ReorgInputTipClientNotFound)?;
    debug!("tip client index = {tip_client_index}");
    let tip_chain_work: U256 = {
        let input_data = hl::load_cell_data(tip_client_index, Source::Input)?;
        if let Ok(packed_input_client) = SpvClientReader::from_slice(&input_data) {
            debug!("tip client = {packed_input_client} (index={tip_client_index})");
            packed_input_client
                .headers_mmr_root()
                .partial_chain_work()
                .unpack()
        } else {
            return Err(InternalError::ReorgInputTipClientLoadFailed.into());
        }
    };

    let (clients_count, flags) = {
        let script = hl::load_script()?;
        let script_args = script.args();
        let script_args_slice = script_args.as_reader().raw_data();
        let args =
            SpvTypeArgsReader::from_slice(script_args_slice).map_err(|_| SysError::Encoding)?;
        let clients_count: u8 = args.clients_count().into();
        let flags: u8 = args.flags().into();
        (clients_count, flags)
    };
    debug!("clients count: {clients_count}, flags: {flags:08b}");

    let mut client_ids = client_ids_with_indexes
        .into_iter()
        .map(|(_, id)| id)
        .collect::<Vec<_>>();
    client_ids.sort();
    debug!("input client ids = {:?}", client_ids);

    let (expected_client_id, expected_client_ids, fork_client_id) = {
        let mut tmp_ids = Vec::new();
        let mut tmp_id = tip_client_id;
        let mut tmp_new_id = 0;
        for _ in 0..client_ids.len() {
            tmp_ids.push(tmp_id);
            tmp_new_id = tmp_id;
            tmp_id = utilities::prev_client_id(tmp_id, clients_count);
        }
        tmp_ids.sort();
        (tmp_new_id, tmp_ids, tmp_id)
    };

    debug!("expected client id = {expected_client_id}");
    debug!("expected client ids = {:?}", expected_client_ids);

    if client_ids != expected_client_ids {
        return Err(InternalError::ReorgInputClientIdsIsMismatch.into());
    }

    Ok((
        input_info,
        expected_client_id,
        expected_client_ids,
        tip_chain_work,
        fork_client_id,
        flags,
    ))
}

fn load_outputs(
    outputs: &[usize],
    expected_info: &SpvInfo,
    expected_client_ids: Vec<u8>,
) -> Result<(packed::SpvClient, usize)> {
    let mut client_ids = Vec::new();
    let mut output_info_opt = None;
    let mut tip_client_opt = None;
    let mut expected_client_opt: Option<SpvClient> = None;
    let mut info_index = 0;
    for i in outputs {
        debug!("load cell data of outputs[{i}]");
        let output_data = hl::load_cell_data(*i, Source::Output)?;
        if let Ok(packed_output_client) = SpvClientReader::from_slice(&output_data) {
            debug!("output client = {packed_output_client} (index={i})");
            let output_client_id: u8 = packed_output_client.id().into();
            debug!("output client id = {output_client_id}");
            // All output clients should have same data, expect their own IDs.
            if let Some(ref mut expected_client) = expected_client_opt {
                expected_client.id = output_client_id;
                debug!("actual client cell: {packed_output_client} (first)");
                let expected = expected_client.pack();
                if packed_output_client.as_slice() != expected.as_slice() {
                    return Err(InternalError::ReorgNewClientIsIncorrect.into());
                }
            } else {
                debug!("actual client cell: {packed_output_client}");
                expected_client_opt = Some(packed_output_client.unpack());
            }
            client_ids.push(output_client_id);
            // The new tip SPV client.
            if output_client_id == expected_info.tip_client_id {
                tip_client_opt = Some(packed_output_client.to_entity());
            }
        } else if let Ok(packed_output_info) = SpvInfoReader::from_slice(&output_data) {
            debug!("output info = {packed_output_info} (index={i})");
            info_index = *i;
            if output_info_opt.is_some() {
                return Err(InternalError::ReorgOutputInfoDuplicated.into());
            }
            let packed_expected_info = expected_info.pack();
            debug!("expected info = {packed_expected_info}");
            if packed_output_info.as_slice() != packed_expected_info.as_slice() {
                return Err(InternalError::UpdateOutputInfoChanged.into());
            }
            let output_info: SpvInfo = packed_output_info.unpack();
            output_info_opt = Some(output_info);
        } else {
            return Err(InternalError::ReorgOutputMalformed.into());
        }
    }

    if output_info_opt.is_none() {
        return Err(InternalError::ReorgOutputInfoNotFound.into());
    }
    if tip_client_opt.is_none() {
        return Err(InternalError::ReorgOutputTipClientNotFound.into());
    }
    let tip_client = tip_client_opt.unwrap();
    debug!("output tip client = {tip_client}");

    client_ids.sort();
    debug!("output client ids = {:?}", client_ids);

    if client_ids != expected_client_ids {
        return Err(InternalError::ReorgOutputClientIdsIsMismatch.into());
    }

    Ok((tip_client, info_index))
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
                    return Err(InternalError::ReorgCellDepMoreThanOne.into());
                }
            }
        }
    }
    if indexes.is_empty() {
        return Err(InternalError::ReorgCellDepNotFound.into());
    }
    Ok(indexes[0])
}

fn load_cell_dep(cell_dep_index: usize, fork_client_id: u8) -> Result<SpvClient> {
    debug!("load cell data of cell deps[{cell_dep_index}]");
    let cell_dep_data = hl::load_cell_data(cell_dep_index, Source::CellDep)?;

    let packed_cell_dep_client =
        if let Ok(cell_dep_client) = SpvClientReader::from_slice(&cell_dep_data) {
            debug!("cell-dep client = {cell_dep_client} (index={cell_dep_index})");
            cell_dep_client
        } else {
            return Err(InternalError::ReorgCellDepClientNotFound.into());
        };

    let cell_dep_client: SpvClient = packed_cell_dep_client.unpack();
    debug!("cell-dep client id = {}", cell_dep_client.id);
    if cell_dep_client.id != fork_client_id {
        return Err(InternalError::ReorgCellDepClientIdIsMismatch.into());
    }
    Ok(cell_dep_client)
}
