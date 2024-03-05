use ckb_bitcoin_spv_verifier::types::{packed::SpvTypeArgsReader, prelude::*};
use ckb_std::{debug, error::SysError, high_level as hl};

use crate::error::{InternalError, Result};

pub(crate) fn destroy_cells(indexes: &[usize]) -> Result<()> {
    debug!("destroyed count: {}", indexes.len());
    let clients_count: u8 = {
        let script = hl::load_script()?;
        let script_args = script.args();
        let script_args_slice = script_args.as_reader().raw_data();
        SpvTypeArgsReader::from_slice(script_args_slice)
            .map_err(|_| SysError::Encoding)?
            .clients_count()
            .into()
    };
    debug!("clients count: {clients_count}");
    let cells_count = 1 + usize::from(clients_count);
    debug!("cells count: {cells_count}");
    if indexes.len() != cells_count {
        return Err(InternalError::DestroyNotEnoughCells.into());
    }
    Ok(())
}
