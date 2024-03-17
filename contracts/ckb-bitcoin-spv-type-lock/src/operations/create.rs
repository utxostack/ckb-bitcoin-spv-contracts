use ckb_bitcoin_spv_verifier::types::{
    packed::{SpvBootstrapReader, SpvClientReader, SpvInfoReader, SpvTypeArgsReader},
    prelude::*,
};
use ckb_std::{ckb_constants::Source, debug, error::SysError, high_level as hl};

use crate::{
    error::{InternalError, Result},
    utilities,
};

pub(crate) fn create_cells(indexes: &[usize]) -> Result<()> {
    if indexes.len() < 1 + 1 + 2 {
        return Err(InternalError::CreateNotEnoughCells.into());
    }
    if indexes.windows(2).any(|pair| pair[0] + 1 != pair[1]) {
        return Err(InternalError::CreateShouldBeOrdered.into());
    }
    // Checks args of the client type script, then returns the clients count;
    let clients_count = {
        let type_args = {
            let script = hl::load_script()?;
            let script_args = script.args();
            let script_args_slice = script_args.as_reader().raw_data();
            SpvTypeArgsReader::from_slice(script_args_slice)
                .map_err(|_| SysError::Encoding)?
                .unpack()
        };
        let clients_count = usize::from(type_args.clients_count);
        let cells_count = 1 + clients_count;
        if indexes.len() != cells_count {
            return Err(InternalError::CreateCellsCountNotMatched.into());
        }
        let type_id = utilities::load_then_calculate_type_id(indexes.len())?;
        if type_id != type_args.type_id.as_ref() {
            return Err(InternalError::CreateIncorrectUniqueId.into());
        }
        clients_count
    };
    // First cell is the client info cell.
    let mut index = indexes[0];
    {
        debug!("check client info cell (index={index})");
        let output_data = hl::load_cell_data(index, Source::Output)?;
        let packed_info = SpvInfoReader::from_slice(&output_data)
            .map_err(|_| InternalError::CreateBadInfoCellData)?;
        debug!("actual client info cell: {packed_info}");
        let info = packed_info.unpack();
        if info.tip_client_id != 0 {
            return Err(InternalError::CreateInfoIndexShouldBeZero.into());
        }
    }
    // Gets the client bootstrap from the witness.
    let bootstrap = {
        let witness_args = hl::load_witness_args(index, Source::Output)?;
        if let Some(args) = witness_args.input_type().to_opt() {
            SpvBootstrapReader::from_slice(&args.raw_data())
                .map_err(|_| SysError::Encoding)?
                .to_entity()
        } else {
            return Err(InternalError::CreateWitnessIsNotExisted.into());
        }
    };
    // Gets the new client from the client bootstrap.
    let mut expected_client = bootstrap.initialize_spv_client()?;
    debug!("expected client cell (id=0): {}", expected_client.pack());
    // Next `clients_count` cells are the client cells;
    index += 1;
    for _id in 0..clients_count {
        debug!("check client cell (index={index}, id={_id})");
        let output_data = hl::load_cell_data(index, Source::Output)?;
        let actual = SpvClientReader::from_slice(&output_data)
            .map_err(|_| InternalError::CreateBadClientCellData)?;
        debug!("actual client cell: {actual}");
        let expected = expected_client.pack();
        if actual.as_slice() != expected.as_slice() {
            return Err(InternalError::CreateNewClientIsIncorrect.into());
        }
        expected_client.id += 1;
        index += 1;
    }

    Ok(())
}
