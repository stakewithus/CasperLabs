use std::collections::HashMap;

use grpc::RequestOptions;

use contract_ffi::key::Key;
use contract_ffi::value::account::PublicKey;
use contract_ffi::value::{Value, U512};
use engine_core::engine_state::genesis::{GenesisAccount, GenesisConfig};
use engine_core::engine_state::{EngineConfig, EngineState, SYSTEM_ACCOUNT_ADDR};
use engine_grpc_server::engine_server::ipc_grpc::ExecutionEngineService;
use engine_shared::motes::Motes;
use engine_storage::global_state::in_memory::InMemoryGlobalState;
use engine_wasm_prep::wasm_costs::WasmCosts;

use crate::support::test_support;
use crate::support::test_support::InMemoryWasmTestBuilder;

const MINT_INSTALL: &str = "mint_install.wasm";
const POS_INSTALL: &str = "pos_install.wasm";
const BAD_INSTALL: &str = "standard_payment.wasm";

const GENESIS_ADDR: [u8; 32] = [6u8; 32];
const CHAIN_NAME: &str = "Jeremiah";
const TIMESTAMP: u64 = 0;
const PROTOCOL_VERSION: u64 = 1;
const ACCOUNT_1_ADDR: [u8; 32] = [1u8; 32];
const ACCOUNT_2_ADDR: [u8; 32] = [2u8; 32];
const ACCOUNT_1_BONDED_AMOUNT: u64 = 1_000_000;
const ACCOUNT_2_BONDED_AMOUNT: u64 = 2_000_000;
const ACCOUNT_1_BALANCE: u64 = 1_000_000_000;
const ACCOUNT_2_BALANCE: u64 = 2_000_000_000;

#[ignore]
#[test]
fn should_run_genesis() {
    let engine_config = EngineConfig::new().set_use_payment_code(true);
    let global_state = InMemoryGlobalState::empty().expect("should create global state");
    let engine_state = EngineState::new(global_state, engine_config);

    let (genesis_request, _) = test_support::create_genesis_request(GENESIS_ADDR, HashMap::new());

    let request_options = RequestOptions::new();

    let genesis_response = engine_state
        .run_genesis(request_options, genesis_request)
        .wait_drop_metadata();

    let response = genesis_response.unwrap();

    let response_root_hash = response.get_success().get_poststate_hash();

    assert!(!response_root_hash.to_vec().is_empty());
}

#[ignore]
#[test]
fn test_genesis_hash_match() {
    let mut builder_base = InMemoryWasmTestBuilder::default();

    let builder = builder_base.run_genesis(GENESIS_ADDR, HashMap::new());

    // This is trie's post state hash after calling run_genesis endpoint.
    let genesis_run_hash = builder.get_genesis_hash();
    let genesis_transforms = builder.get_genesis_transforms().clone();

    let empty_root_hash = {
        let gs = InMemoryGlobalState::empty().expect("Empty GlobalState.");
        gs.empty_root_hash
    };

    // This is trie's post state hash after committing genesis effects on top of empty trie.
    let genesis_transforms_hash = builder
        .commit_effects(empty_root_hash.to_vec(), genesis_transforms)
        .get_post_state_hash();

    // They should match.
    assert_eq!(genesis_run_hash, genesis_transforms_hash);
}

/* --- NEW GENESIS STARTS HERE --- */

#[ignore]
#[test]
fn should_run_genesis_with_chainspec() {
    let account_1_balance = Motes::new(ACCOUNT_1_BALANCE.into());
    let account_1 = {
        let account_1_public_key = PublicKey::new(ACCOUNT_1_ADDR);
        let account_1_bonded_amount = Motes::new(ACCOUNT_1_BONDED_AMOUNT.into());
        GenesisAccount::new(
            account_1_public_key,
            account_1_balance,
            account_1_bonded_amount,
        )
    };

    let account_2_balance = Motes::new(ACCOUNT_2_BALANCE.into());
    let account_2 = {
        let account_2_public_key = PublicKey::new(ACCOUNT_2_ADDR);
        let account_2_bonded_amount = Motes::new(ACCOUNT_2_BONDED_AMOUNT.into());
        GenesisAccount::new(
            account_2_public_key,
            account_2_balance,
            account_2_bonded_amount,
        )
    };

    let name = CHAIN_NAME.to_string();
    let mint_installer_bytes = test_support::read_wasm_file_bytes(MINT_INSTALL);
    let pos_installer_bytes = test_support::read_wasm_file_bytes(POS_INSTALL);
    let accounts = vec![account_1, account_2];
    let wasm_costs = WasmCosts::from_version(PROTOCOL_VERSION).unwrap();

    let genesis_config = GenesisConfig::new(
        name,
        TIMESTAMP,
        PROTOCOL_VERSION,
        mint_installer_bytes,
        pos_installer_bytes,
        accounts,
        wasm_costs,
    );

    let mut builder = {
        let engine_config = EngineConfig::default().set_use_payment_code(true);
        InMemoryWasmTestBuilder::new(engine_config)
    };

    builder
        .run_genesis_with_genesis_config(genesis_config)
        .expect("should run genesis");

    let system_account = builder
        .get_account(Key::Account(SYSTEM_ACCOUNT_ADDR))
        .expect("system account should exist");

    let account_1 = builder
        .get_account(Key::Account(ACCOUNT_1_ADDR))
        .expect("account 1 should exist");

    let account_2 = builder
        .get_account(Key::Account(ACCOUNT_2_ADDR))
        .expect("account 2 should exist");

    let system_account_balance_actual = builder.get_purse_balance(system_account.purse_id());
    let account_1_balance_actual = builder.get_purse_balance(account_1.purse_id());
    let account_2_balance_actual = builder.get_purse_balance(account_2.purse_id());

    assert_eq!(system_account_balance_actual, U512::zero());
    assert_eq!(account_1_balance_actual, account_1_balance.value());
    assert_eq!(account_2_balance_actual, account_2_balance.value());

    let mint_contract_uref = builder.get_mint_contract_uref();
    let pos_contract_uref = builder.get_pos_contract_uref();

    if let Some(Value::Contract(_)) = builder.query(None, Key::URef(mint_contract_uref), &[]) {
        // Contract exists at mint contract URef
    } else {
        panic!("contract not found at mint uref");
    }

    if let Some(Value::Contract(_)) = builder.query(None, Key::URef(pos_contract_uref), &[]) {
        // Contract exists at pos contract URef
    } else {
        panic!("contract not found at pos uref");
    }
}

#[ignore]
#[test]
fn should_fail_if_bad_mint_install_contract_is_provided() {
    let genesis_config = {
        let account_1 = {
            let account_1_public_key = PublicKey::new(ACCOUNT_1_ADDR);
            let account_1_balance = Motes::new(ACCOUNT_1_BALANCE.into());
            let account_1_bonded_amount = Motes::new(ACCOUNT_1_BONDED_AMOUNT.into());
            GenesisAccount::new(
                account_1_public_key,
                account_1_balance,
                account_1_bonded_amount,
            )
        };
        let account_2 = {
            let account_2_public_key = PublicKey::new(ACCOUNT_2_ADDR);
            let account_2_balance = Motes::new(ACCOUNT_2_BALANCE.into());
            let account_2_bonded_amount = Motes::new(ACCOUNT_2_BONDED_AMOUNT.into());
            GenesisAccount::new(
                account_2_public_key,
                account_2_balance,
                account_2_bonded_amount,
            )
        };
        let name = CHAIN_NAME.to_string();
        let mint_installer_bytes = test_support::read_wasm_file_bytes(BAD_INSTALL);
        let pos_installer_bytes = test_support::read_wasm_file_bytes(POS_INSTALL);
        let accounts = vec![account_1, account_2];
        let wasm_costs = WasmCosts::from_version(PROTOCOL_VERSION).unwrap();
        GenesisConfig::new(
            name,
            TIMESTAMP,
            PROTOCOL_VERSION,
            mint_installer_bytes,
            pos_installer_bytes,
            accounts,
            wasm_costs,
        )
    };

    let mut builder = {
        let engine_config = EngineConfig::default().set_use_payment_code(true);
        InMemoryWasmTestBuilder::new(engine_config)
    };

    if builder
        .run_genesis_with_genesis_config(genesis_config)
        .is_ok()
    {
        panic!("genesis should fail with a bad install contract")
    }
}

#[ignore]
#[test]
fn should_fail_if_bad_pos_install_contract_is_provided() {
    let genesis_config = {
        let account_1 = {
            let account_1_public_key = PublicKey::new(ACCOUNT_1_ADDR);
            let account_1_balance = Motes::new(ACCOUNT_1_BALANCE.into());
            let account_1_bonded_amount = Motes::new(ACCOUNT_1_BONDED_AMOUNT.into());
            GenesisAccount::new(
                account_1_public_key,
                account_1_balance,
                account_1_bonded_amount,
            )
        };
        let account_2 = {
            let account_2_public_key = PublicKey::new(ACCOUNT_2_ADDR);
            let account_2_balance = Motes::new(ACCOUNT_2_BALANCE.into());
            let account_2_bonded_amount = Motes::new(ACCOUNT_2_BONDED_AMOUNT.into());
            GenesisAccount::new(
                account_2_public_key,
                account_2_balance,
                account_2_bonded_amount,
            )
        };
        let name = CHAIN_NAME.to_string();
        let mint_installer_bytes = test_support::read_wasm_file_bytes(MINT_INSTALL);
        let pos_installer_bytes = test_support::read_wasm_file_bytes(BAD_INSTALL);
        let accounts = vec![account_1, account_2];
        let wasm_costs = WasmCosts::from_version(PROTOCOL_VERSION).unwrap();
        GenesisConfig::new(
            name,
            TIMESTAMP,
            PROTOCOL_VERSION,
            mint_installer_bytes,
            pos_installer_bytes,
            accounts,
            wasm_costs,
        )
    };

    let mut builder = {
        let engine_config = EngineConfig::default().set_use_payment_code(true);
        InMemoryWasmTestBuilder::new(engine_config)
    };

    if builder
        .run_genesis_with_genesis_config(genesis_config)
        .is_ok()
    {
        panic!("genesis should fail with a bad install contract")
    }
}
