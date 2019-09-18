#[macro_use]
extern crate criterion;
extern crate contract_ffi;
extern crate engine_core;
extern crate engine_shared;
extern crate engine_storage;
use std::collections::HashMap;

use criterion::{Criterion, Throughput};
use tempfile::TempDir;

use casperlabs_engine_tests::support::test_support::{
    DeployBuilder, ExecRequestBuilder, LmdbWasmTestBuilder, WasmTestResult, DEFAULT_BLOCK_TIME,
    STANDARD_PAYMENT_CONTRACT,
};
use contract_ffi::value::account::PublicKey;
use contract_ffi::value::U512;
use engine_core::engine_state::EngineConfig;
use engine_core::engine_state::MAX_PAYMENT;
use engine_storage::global_state::lmdb::LmdbGlobalState;

/// Size of batch used in multiple execs benchmark, and multiple deploys per exec cases.
const TRANSFER_BATCH_SIZE: u64 = 3;

const GENESIS_ADDR: [u8; 32] = [1; 32];
const TARGET_ADDR: [u8; 32] = [127; 32];

fn engine_with_payments() -> EngineConfig {
    EngineConfig::new().set_use_payment_code(true)
}

fn bootstrap(accounts: &[PublicKey]) -> (WasmTestResult<LmdbGlobalState>, TempDir) {
    let accounts_bytes: Vec<Vec<u8>> = accounts
        .iter()
        .map(|public_key| public_key.value().to_vec())
        .collect();
    let amount = U512::one();

    let data_dir = TempDir::new().expect("should create temp dir");
    let result = LmdbWasmTestBuilder::new_with_config(&data_dir.path(), engine_with_payments())
        .run_genesis(GENESIS_ADDR, HashMap::new())
        .exec_with_args(
            GENESIS_ADDR,
            STANDARD_PAYMENT_CONTRACT,
            (U512::from(MAX_PAYMENT),),
            "create_accounts.wasm",
            (accounts_bytes, amount), //args
            DEFAULT_BLOCK_TIME,       // blocktime
            [1; 32],                  // deploy_hash
        )
        .expect_success()
        .commit()
        .finish();

    (result, data_dir)
}

/// Uses multiple exec requests with a single deploy to transfer tokens. Executes all transfers in
/// batch determined by value of TRANSFER_BATCH_SIZE.
fn transfer_to_account_multiple_execs(builder: &mut LmdbWasmTestBuilder, account: PublicKey) {
    let amount = U512::one();
    // To see raw numbers take current time
    for i in 0..TRANSFER_BATCH_SIZE {
        builder
            .exec_with_args(
                GENESIS_ADDR,
                STANDARD_PAYMENT_CONTRACT,
                (U512::from(MAX_PAYMENT),),
                "transfer_to_existing_account.wasm",
                (account, amount),  //args
                DEFAULT_BLOCK_TIME, // blocktime
                [2 + i as u8; 32],  // deploy_hash
            )
            .expect_success()
            .commit();
    }
}

/// Executes multiple deploys per single exec with based on TRANSFER_BATCH_SIZE.
fn transfer_to_account_multiple_deploys(builder: &mut LmdbWasmTestBuilder, account: PublicKey) {
    let mut exec_builder = ExecRequestBuilder::new();

    for i in 0..TRANSFER_BATCH_SIZE {
        let deploy = DeployBuilder::default()
            .with_address(GENESIS_ADDR)
            .with_payment_code(STANDARD_PAYMENT_CONTRACT, (U512::from(MAX_PAYMENT),))
            .with_session_code("transfer_to_existing_account.wasm", (account, U512::one()))
            .with_authorization_keys(&[PublicKey::new(GENESIS_ADDR)])
            .with_deploy_hash([2 + i as u8; 32]) // deploy_hash
            .build();
        exec_builder = exec_builder.push_deploy(deploy);
    }

    builder
        .exec_with_exec_request(exec_builder.build())
        .expect_success()
        .commit();
}

pub fn transfer_bench(c: &mut Criterion) {
    let target_account = PublicKey::new(TARGET_ADDR);
    let bootstrap_accounts = vec![target_account];

    let mut group = c.benchmark_group("tps");

    // Minimize no of samples and measurement times to decrease the total time of this benchmark
    // possibly not decreasing quality of the numbers that much.
    group.sample_size(10);

    // Measure by elements where one element/s is one transaction per second
    group.throughput(Throughput::Elements(TRANSFER_BATCH_SIZE));

    // Bootstrap database once
    let (result_1, _source_dir_1) = bootstrap(&bootstrap_accounts);
    let mut builder_1 = LmdbWasmTestBuilder::from_result(result_1);

    group.bench_function(
        format!(
            "transfer_to_existing_account_multiple_execs/{}",
            TRANSFER_BATCH_SIZE
        ),
        |b| {
            b.iter(|| {
                // Execute multiple deploys with one exec request each
                transfer_to_account_multiple_execs(&mut builder_1, target_account)
            })
        },
    );

    let (result_2, _source_dir_2) = bootstrap(&bootstrap_accounts);
    let mut builder_2 = LmdbWasmTestBuilder::from_result(result_2);

    group.bench_function(
        format!(
            "transfer_to_existing_account_multiple_deploys_per_exec/{}",
            TRANSFER_BATCH_SIZE
        ),
        |b| {
            // Create new directory with copied contents of existing bootstrapped LMDB database
            b.iter(|| {
                // Execute multiple deploys with a single exec request
                transfer_to_account_multiple_deploys(&mut builder_2, target_account)
            })
        },
    );
    group.finish();
}

criterion_group!(benches, transfer_bench);
criterion_main!(benches);
