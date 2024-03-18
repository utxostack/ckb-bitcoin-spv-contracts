use ckb_testtool::{
    ckb_hash::blake2b_256,
    ckb_types::{bytes::Bytes, core::TransactionBuilder, packed::*, prelude::*},
    context::Context,
};

use crate::{prelude::*, utilities, Loader};

struct Case {
    inputs_capacity: Vec<u64>,
    outputs_capacity: Vec<u64>,
    unlocked: bool,
    should_pass: bool,
}

fn run_test(case: &Case) {
    utilities::setup();

    let loader = Loader::default();
    let mut context = Context::default();

    const PASSWORD: &[u8] = &[0x12, 0x34, 0x56, 0x78];

    // Deploy the lock script.
    let lock_script = {
        let lock_bin = loader.load_binary("can-update-without-ownership-lock");
        let lock_out_point = context.deploy_cell(lock_bin);
        let args = blake2b_256(PASSWORD);
        context
            .build_script(&lock_out_point, Default::default())
            .expect("script")
            .as_builder()
            .args((args[..]).pack())
            .build()
    };

    let inputs = case
        .inputs_capacity
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

    let outputs = case
        .outputs_capacity
        .iter()
        .map(|cap| {
            CellOutput::new_builder()
                .capacity(cap.pack())
                .lock(lock_script.clone())
                .build()
        })
        .collect::<Vec<_>>();
    let outputs_data = vec![Bytes::new(); outputs.len()];

    let tx_builder = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack());
    let tx = if case.unlocked {
        let witness = {
            let type_args = BytesOpt::new_builder()
                .set(Some(Pack::pack(PASSWORD)))
                .build();
            let witness_args = WitnessArgs::new_builder().lock(type_args).build();
            witness_args.as_bytes()
        };
        tx_builder.witness(witness.pack())
    } else {
        tx_builder
    }
    .build();
    let tx = context.complete_tx(tx);

    let inputs_total: u64 = case.inputs_capacity.iter().copied().sum();
    let outputs_total: u64 = case.outputs_capacity.iter().copied().sum();
    let expected_result = case.unlocked || inputs_total <= outputs_total;
    assert_eq!(case.should_pass, expected_result);
    if case.should_pass {
        let _ = context.should_be_passed(&tx, MAX_CYCLES);
    } else {
        let _ = context.should_be_failed(&tx, MAX_CYCLES);
    }
}

#[test]
fn unchanged_case_1() {
    let case = Case {
        inputs_capacity: vec![1000],
        outputs_capacity: vec![1000],
        unlocked: false,
        should_pass: true,
    };
    run_test(&case);
}

#[test]
fn unchanged_case_2() {
    let case = Case {
        inputs_capacity: vec![499, 501],
        outputs_capacity: vec![1000],
        unlocked: false,
        should_pass: true,
    };
    run_test(&case);
}

#[test]
fn unchanged_case_3() {
    let case = Case {
        inputs_capacity: vec![1000],
        outputs_capacity: vec![499, 501],
        unlocked: false,
        should_pass: true,
    };
    run_test(&case);
}

#[test]
fn increase_case_1() {
    let case = Case {
        inputs_capacity: vec![1000],
        outputs_capacity: vec![1001],
        unlocked: false,
        should_pass: true,
    };
    run_test(&case);
}

#[test]
fn increase_case_2() {
    let case = Case {
        inputs_capacity: vec![499, 501],
        outputs_capacity: vec![1001],
        unlocked: false,
        should_pass: true,
    };
    run_test(&case);
}

#[test]
fn increase_case_3() {
    let case = Case {
        inputs_capacity: vec![1000],
        outputs_capacity: vec![500, 501],
        unlocked: false,
        should_pass: true,
    };
    run_test(&case);
}

#[test]
fn decrease_case_1a() {
    let case = Case {
        inputs_capacity: vec![1000],
        outputs_capacity: vec![999],
        unlocked: false,
        should_pass: false,
    };
    run_test(&case);
}

#[test]
fn decrease_case_1b() {
    let case = Case {
        inputs_capacity: vec![1000],
        outputs_capacity: vec![999],
        unlocked: true,
        should_pass: true,
    };
    run_test(&case);
}

#[test]
fn decrease_case_2a() {
    let case = Case {
        inputs_capacity: vec![499, 501],
        outputs_capacity: vec![999],
        unlocked: false,
        should_pass: false,
    };
    run_test(&case);
}

#[test]
fn decrease_case_2b() {
    let case = Case {
        inputs_capacity: vec![499, 501],
        outputs_capacity: vec![999],
        unlocked: true,
        should_pass: true,
    };
    run_test(&case);
}

#[test]
fn decrease_case_3a() {
    let case = Case {
        inputs_capacity: vec![1000],
        outputs_capacity: vec![499, 500],
        unlocked: false,
        should_pass: false,
    };
    run_test(&case);
}

#[test]
fn decrease_case_3b() {
    let case = Case {
        inputs_capacity: vec![1000],
        outputs_capacity: vec![499, 500],
        unlocked: true,
        should_pass: true,
    };
    run_test(&case);
}
