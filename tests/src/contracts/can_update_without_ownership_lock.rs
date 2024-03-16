use ckb_testtool::{
    ckb_types::{bytes::Bytes, core::TransactionBuilder, packed::*, prelude::*},
    context::Context,
};

use crate::{prelude::*, utilities, Loader};

fn run_test(inputs_capacity: &[u64], outputs_capacity: &[u64]) {
    utilities::setup();

    let loader = Loader::default();
    let mut context = Context::default();

    // Deploy the lock script.
    let lock_script = {
        let lock_bin = loader.load_binary("can-update-without-ownership-lock");
        let lock_out_point = context.deploy_cell(lock_bin);
        context
            .build_script(&lock_out_point, Default::default())
            .expect("script")
            .as_builder()
            .args([0u8, 1, 2, 3].pack())
            .build()
    };

    let inputs = inputs_capacity
        .iter()
        .map(|cap| {
            let output = CellOutput::new_builder()
                .capacity(cap.pack())
                .lock(lock_script.clone())
                .build();
            let out_point = context.create_cell(output, Bytes::new());
            CellInput::new_builder().previous_output(out_point).build()
        })
        .collect::<Vec<_>>();

    let outputs = outputs_capacity
        .iter()
        .map(|cap| {
            CellOutput::new_builder()
                .capacity(cap.pack())
                .lock(lock_script.clone())
                .build()
        })
        .collect::<Vec<_>>();
    let outputs_data = vec![Bytes::new(); outputs.len()];

    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .build();
    let tx = context.complete_tx(tx);

    let inputs_total: u64 = inputs_capacity.iter().copied().sum();
    let outputs_total: u64 = outputs_capacity.iter().copied().sum();
    if inputs_total > outputs_total {
        let _ = context.should_be_failed(&tx, MAX_CYCLES);
    } else {
        let _ = context.should_be_passed(&tx, MAX_CYCLES);
    }
}

#[test]
fn lost_capacity_case_1() {
    let inputs_capacity = vec![1000];
    let outputs_capacity = vec![1000];
    run_test(&inputs_capacity, &outputs_capacity);
}

#[test]
fn lost_capacity_case_2() {
    let inputs_capacity = vec![999];
    let outputs_capacity = vec![1000];
    run_test(&inputs_capacity, &outputs_capacity);
}

#[test]
fn lost_capacity_case_3() {
    let inputs_capacity = vec![1000];
    let outputs_capacity = vec![999];
    run_test(&inputs_capacity, &outputs_capacity);
}

#[test]
fn lost_capacity_case_4() {
    let inputs_capacity = vec![500, 500];
    let outputs_capacity = vec![999];
    run_test(&inputs_capacity, &outputs_capacity);
}

#[test]
fn lost_capacity_case_5() {
    let inputs_capacity = vec![500, 500];
    let outputs_capacity = vec![1001];
    run_test(&inputs_capacity, &outputs_capacity);
}

#[test]
fn lost_capacity_case_6() {
    let inputs_capacity = vec![250, 250, 250, 250];
    let outputs_capacity = vec![500, 501];
    run_test(&inputs_capacity, &outputs_capacity);
}

#[test]
fn lost_capacity_case_7() {
    let inputs_capacity = vec![250, 250, 250, 250];
    let outputs_capacity = vec![500, 499];
    run_test(&inputs_capacity, &outputs_capacity);
}
