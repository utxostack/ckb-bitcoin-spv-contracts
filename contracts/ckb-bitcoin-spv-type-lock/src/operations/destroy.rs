use ckb_std::debug;

use crate::{
    error::{InternalError, Result},
    utilities,
};

pub(crate) fn destroy_cells(indexes: &[usize]) -> Result<()> {
    debug!("destroyed count: {}", indexes.len());
    let clients_count: u8 = {
        let type_args = utilities::load_spv_type_args()?;
        type_args.clients_count
    };
    debug!("clients count: {clients_count}");
    let cells_count = 1 + usize::from(clients_count);
    debug!("cells count: {cells_count}");
    if indexes.len() != cells_count {
        return Err(InternalError::DestroyNotEnoughCells.into());
    }
    Ok(())
}
