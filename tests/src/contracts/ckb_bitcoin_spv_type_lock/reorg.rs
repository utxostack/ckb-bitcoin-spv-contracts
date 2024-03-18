use std::{cmp::Ordering, mem};

use ckb_bitcoin_spv_prover::DummyService;
use ckb_bitcoin_spv_verifier::types::{core, packed, prelude::Pack as VPack};
use ckb_testtool::{
    ckb_types::{
        bytes::Bytes,
        core::{DepType, TransactionBuilder},
        packed::*,
        prelude::*,
    },
    context::Context,
};

use crate::{prelude::*, utilities, Loader};

#[test]
fn normal_case_1() {
    let case = NormalCase {
        headers_path: "case-0822528_0830592",
        start_height: 822528,
        stale_height: 823226,
        clients_count: 5,
        stale_client_id: 1,
        reorg_clients_count: 3,
    };
    test_normal(case);
}

struct NormalCase<'a> {
    headers_path: &'a str,
    start_height: u32,
    stale_height: u32,
    clients_count: u8,
    stale_client_id: u8,
    reorg_clients_count: u8,
}

fn test_normal(case: NormalCase) {
    utilities::setup();

    let mut header_bins_iter = {
        let headers_path = format!("main-chain/headers/continuous/{}", case.headers_path);
        utilities::find_bin_files(&headers_path, "").into_iter()
    };

    let mut service = {
        let header = loop {
            let header_bin = header_bins_iter.next().unwrap();
            let height: u32 = header_bin
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .parse()
                .unwrap();
            match height.cmp(&case.start_height) {
                Ordering::Equal => {
                    let header: core::Header =
                        utilities::decode_from_bin_file(&header_bin).unwrap();
                    break header;
                }
                Ordering::Greater => {
                    panic!("not enough headers");
                }
                Ordering::Less => {}
            }
        };

        DummyService::bootstrap(case.start_height, header).unwrap()
    };

    let loader = Loader::default();
    let mut context = Context::default();

    let lock_script = {
        let bin = loader.load_binary("can-update-without-ownership-lock");
        let out_point = context.deploy_cell(bin);
        context
            .build_script(&out_point, Default::default())
            .expect("lock script")
            .as_builder()
            .args([0u8, 1, 2, 3].pack())
            .build()
    };

    let type_script = {
        let cells_count = usize::from(case.clients_count) + 1;
        let capacity = SPV_CELL_CAP * (u64::from(case.clients_count) + 1);
        let original_input = {
            let output = CellOutput::new_builder()
                .capacity(capacity.pack())
                .lock(lock_script.clone())
                .build();
            let out_point = context.create_cell(output, Bytes::new());
            CellInput::new_builder().previous_output(out_point).build()
        };

        let type_id_array = utilities::calculate_type_id(original_input, cells_count);
        let type_id = core::Hash::from_bytes_ref(&type_id_array);
        let args = packed::SpvTypeArgs::new_builder()
            .type_id(type_id.pack())
            .clients_count(case.clients_count.into())
            .build();
        let bin = loader.load_binary("ckb-bitcoin-spv-type-lock");
        let out_point = context.deploy_cell(bin);
        context
            .build_script(&out_point, Default::default())
            .expect("type script")
            .as_builder()
            .args(args.as_slice().pack())
            .build()
    };

    let mut headers = Vec::new();

    // Stop at the parent block of the stale header.
    loop {
        let header_bin = header_bins_iter.next().unwrap();

        let header: core::Header = utilities::decode_from_bin_file(&header_bin).unwrap();
        let height: u32 = header_bin
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        log::trace!("process header-{height} from file {}", header_bin.display());

        headers.push(header);
        if height + 1 < case.stale_height && headers.len() < SPV_HEADERS_GROUP_SIZE {
            continue;
        }
        let _update = service.update(mem::take(&mut headers)).unwrap();
        if height + 1 >= case.stale_height {
            break;
        }
    }

    let stale_client = {
        let stale_header: core::Header = {
            let headers_path = "main-chain/headers/stale";
            let filename = format!("{:07}.bin", case.stale_height);
            let header_bin = utilities::find_bin_file(headers_path, &filename);
            log::trace!(
                "process stale header-{} from file {}",
                case.stale_height,
                header_bin.display()
            );
            utilities::decode_from_bin_file(&header_bin).unwrap()
        };
        headers.push(stale_header);
        let prev_client = service.tip_client();
        let _update = service.update(mem::take(&mut headers)).unwrap();
        let mut stale_client = service.tip_client();
        service.rollback_to(prev_client).unwrap();
        stale_client.id = case.stale_client_id;
        stale_client
    };

    log::trace!(
        "stale client id {}, reorg size {}",
        case.stale_client_id,
        case.reorg_clients_count
    );
    let (reorg_client_ids, new_tip_client_id) = {
        let mut reorg_client_ids = Vec::new();
        let mut reorg_client_id = case.stale_client_id;
        let mut new_tip_client_id = case.stale_client_id;
        for _ in 0..case.reorg_clients_count {
            reorg_client_ids.push(reorg_client_id);
            new_tip_client_id = reorg_client_id;
            reorg_client_id = utilities::prev_client_id(reorg_client_id, case.clients_count)
        }
        (reorg_client_ids, new_tip_client_id)
    };
    let cell_dep_client_id = utilities::prev_client_id(new_tip_client_id, case.clients_count);
    log::trace!("new tip client id will be {new_tip_client_id}");
    log::trace!("cell dep client id is {cell_dep_client_id}");
    log::trace!("reorg client ids are {:?}", reorg_client_ids);

    let input_spv_info = {
        let spv_info = packed::SpvInfo::new_builder()
            .tip_client_id(case.stale_client_id.into())
            .build();
        let output = CellOutput::new_builder()
            .capacity(SPV_CELL_CAP.pack())
            .lock(lock_script.clone())
            .type_(Some(type_script.clone()).pack())
            .build();
        let out_point = context.create_cell(output, spv_info.as_bytes());
        CellInput::new_builder().previous_output(out_point).build()
    };
    let cell_dep_spv_client = {
        let mut tip_spv_client = service.tip_client();
        tip_spv_client.id = cell_dep_client_id;
        let spv_client: packed::SpvClient = tip_spv_client.pack();
        let output = CellOutput::new_builder()
            .capacity(SPV_CELL_CAP.pack())
            .lock(lock_script.clone())
            .type_(Some(type_script.clone()).pack())
            .build();
        let out_point = context.create_cell(output, spv_client.as_bytes());
        CellDep::new_builder()
            .out_point(out_point)
            .dep_type(DepType::Code.into())
            .build()
    };

    let update = {
        for _ in 0..2 {
            let header_bin = header_bins_iter.next().unwrap();

            let header: core::Header = utilities::decode_from_bin_file(&header_bin).unwrap();
            let height: u32 = header_bin
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .parse()
                .unwrap();
            log::trace!("process header-{height} from file {}", header_bin.display());

            headers.push(header);
        }

        service.update(mem::take(&mut headers)).unwrap()
    };

    let reorg_clients_count = usize::from(case.reorg_clients_count);

    let inputs = {
        let mut inputs = Vec::new();
        let mut spv_client = stale_client;
        inputs.push(input_spv_info);
        for i in &reorg_client_ids {
            spv_client.id = *i;
            let packed_spv_client = spv_client.pack();
            let output = CellOutput::new_builder()
                .capacity(SPV_CELL_CAP.pack())
                .lock(lock_script.clone())
                .type_(Some(type_script.clone()).pack())
                .build();
            let out_point = context.create_cell(output, packed_spv_client.as_bytes());
            let input_spv_client = CellInput::new_builder().previous_output(out_point).build();
            inputs.push(input_spv_client);
        }
        inputs
    };

    let outputs = {
        let output = CellOutput::new_builder()
            .capacity(SPV_CELL_CAP.pack())
            .lock(lock_script.clone())
            .type_(Some(type_script.clone()).pack())
            .build();
        vec![output; reorg_clients_count + 1]
    };

    let outputs_data = {
        let mut outputs_data = Vec::new();
        let output_spv_info = packed::SpvInfo::new_builder()
            .tip_client_id(new_tip_client_id.into())
            .build();
        outputs_data.push(output_spv_info.as_slice().pack());
        let mut spv_client = service.tip_client();
        for i in &reorg_client_ids {
            spv_client.id = *i;
            let packed_spv_client = spv_client.pack();
            outputs_data.push(packed_spv_client.as_slice().pack());
        }
        outputs_data
    };

    let witnesses = {
        let mut witnesses = vec![Default::default(); reorg_clients_count + 1];
        let witness_spv_client = {
            let type_args = BytesOpt::new_builder()
                .set(Some(Pack::pack(update.as_slice())))
                .build();
            let witness_args = WitnessArgs::new_builder().output_type(type_args).build();
            witness_args.as_slice().pack()
        };
        witnesses[0] = witness_spv_client;
        witnesses
    };

    let tx = TransactionBuilder::default()
        .cell_dep(cell_dep_spv_client)
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data)
        .witnesses(witnesses)
        .build();
    let tx = context.complete_tx(tx);

    let _ = context.should_be_passed(&tx, MAX_CYCLES);
}
