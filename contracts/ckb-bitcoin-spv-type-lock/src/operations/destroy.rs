use ckb_bitcoin_spv_verifier::types::core::SpvTypeArgs;
use ckb_std::debug;

use crate::error::{InternalError, Result};

pub(crate) fn destroy_cells(indexes: &[usize], type_args: SpvTypeArgs) -> Result<()> {
    debug!("destroyed count: {}", indexes.len());
    let clients_count = type_args.clients_count;
    debug!("clients count: {clients_count}");
    let cells_count = 1 + usize::from(clients_count);
    debug!("cells count: {cells_count}");
    if indexes.len() != cells_count {
        return Err(InternalError::DestroyNotEnoughCells.into());
    }
    Ok(())
}
