use ckb_bitcoin_spv_prover::DummyService;
use ckb_bitcoin_spv_verifier::types::{core, packed, prelude::Pack as VPack};
use ckb_testtool::{
    ckb_types::{bytes::Bytes, core::TransactionBuilder, packed::*, prelude::*},
    context::Context,
};

use crate::{prelude::*, utilities, Loader};

#[test]
fn normal_case_1() {
    let case = NormalCase {
        headers_path: "case-0822528_0830592",
        height: 822528,
        clients_count: 3,
    };
    test_normal(case);
}

#[test]
fn normal_case_2() {
    let case = NormalCase {
        headers_path: "case-0822528_0830592",
        height: 824544,
        clients_count: 5,
    };
    test_normal(case);
}

#[test]
fn normal_case_3() {
    let case = NormalCase {
        headers_path: "case-0822528_0830592",
        height: 826560,
        clients_count: 10,
    };
    test_normal(case);
}

#[test]
fn normal_case_4() {
    let case = NormalCase {
        headers_path: "case-0822528_0830592",
        height: 828576,
        clients_count: 20,
    };
    test_normal(case);
}

#[test]
fn normal_case_5() {
    let case = NormalCase {
        headers_path: "case-0822528_0830592",
        height: 830592,
        clients_count: 30,
    };
    test_normal(case);
}

struct NormalCase<'a> {
    headers_path: &'a str,
    height: u32,
    clients_count: u8,
}

fn test_normal(case: NormalCase) {
    utilities::setup();

    let (service, bootstrap) = {
        let headers_path = format!("main-chain/headers/continuous/{}", case.headers_path);
        let filename = format!("{:07}.bin", case.height);

        let header_bin = utilities::find_bin_file(&headers_path, &filename);
        let header: core::Header = utilities::decode_from_bin_file(&header_bin).unwrap();

        log::trace!(
            "process header-{} from file {}",
            case.height,
            header_bin.display()
        );

        let bootstrap = packed::SpvBootstrap::new_builder()
            .height(VPack::pack(&case.height))
            .header(header.pack())
            .build();

        let service = DummyService::bootstrap(case.height, header).unwrap();
        (service, bootstrap)
    };

    let outputs_data = {
        let spv_info = packed::SpvInfo::new_builder().build();
        let mut outputs_data = vec![spv_info.as_bytes()];
        let mut client = service.tip_client();
        for id in 0..case.clients_count {
            client.id = id;
            let packed_client: packed::SpvClient = client.pack();
            outputs_data.push(packed_client.as_bytes());
        }
        outputs_data
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

    let input = {
        let capacity = SPV_CELL_CAP * (u64::from(case.clients_count) + 1);
        let output = CellOutput::new_builder()
            .capacity(capacity.pack())
            .lock(lock_script.clone())
            .build();
        let out_point = context.create_cell(output, Bytes::new());
        CellInput::new_builder().previous_output(out_point).build()
    };

    let type_script = {
        let cells_count = usize::from(case.clients_count) + 1;
        let type_id_array = utilities::calculate_type_id(input.clone(), cells_count);
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

    let outputs = {
        let spv_cell = CellOutput::new_builder()
            .capacity(SPV_CELL_CAP.pack())
            .lock(lock_script.clone())
            .type_(Some(type_script.clone()).pack())
            .build();
        vec![spv_cell.clone(); usize::from(case.clients_count) + 1]
    };

    let witness = {
        let type_args = BytesOpt::new_builder()
            .set(Some(Pack::pack(bootstrap.as_slice())))
            .build();
        let witness_args = WitnessArgs::new_builder().output_type(type_args).build();
        witness_args.as_bytes()
    };

    let tx = TransactionBuilder::default()
        .input(input)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .witness(Pack::pack(&witness))
        .build();
    let tx = context.complete_tx(tx);

    let _ = context.should_be_passed(&tx, MAX_CYCLES);
}
