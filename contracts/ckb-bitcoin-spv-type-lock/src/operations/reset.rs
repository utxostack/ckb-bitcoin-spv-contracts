use ckb_bitcoin_spv_verifier::types::{core::SpvTypeArgs, packed::SpvInfoReader, prelude::*};
use ckb_std::{ckb_constants::Source, debug, high_level as hl};

use crate::{
    error::{InternalError, Result},
    utilities,
};

pub(crate) fn reset_cells(
    inputs: &[usize],
    outputs: &[usize],
    type_args: SpvTypeArgs,
) -> Result<()> {
    if inputs.windows(2).any(|pair| pair[1] + 1 != pair[0]) {
        return Err(InternalError::CreateShouldBeOrdered.into());
    }
    if outputs.windows(2).any(|pair| pair[1] + 1 != pair[0]) {
        return Err(InternalError::CreateShouldBeOrdered.into());
    }
    // Checks args of the client type script, then returns the clients count;
    let _clients_count = {
        let clients_count = usize::from(type_args.clients_count);
        let cells_count = 1 + clients_count;
        if outputs.len() != cells_count {
            return Err(InternalError::CreateCellsCountNotMatched.into());
        }
        let type_id = utilities::load_then_calculate_type_id(outputs.len())?;
        if type_id != type_args.type_id.as_ref() {
            return Err(InternalError::CreateIncorrectUniqueId.into());
        }
        clients_count
    };
    // First cell is the client info cell.
    let index = outputs[0];
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

    Ok(())
}
