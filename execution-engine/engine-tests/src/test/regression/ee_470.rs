use std::collections::HashMap;

use crate::support::test_support::{InMemoryWasmTestBuilder, DEFAULT_BLOCK_TIME};
use engine_storage::global_state::in_memory::InMemoryGlobalState;

const GENESIS_ADDR: [u8; 32] = [12; 32];

#[ignore]
#[test]
fn regression_test_genesis_hash_mismatch() {
    let mut builder_base = InMemoryWasmTestBuilder::default();

    // Step 1.
    let builder = builder_base.run_genesis(GENESIS_ADDR, HashMap::new());

    // This is trie's post state hash after calling run_genesis endpoint.
    // Step 1a)
    let genesis_run_hash = builder.get_genesis_hash();
    let genesis_transforms = builder.get_genesis_transforms().clone();

    let empty_root_hash = {
        let gs = InMemoryGlobalState::empty().expect("Empty GlobalState.");
        gs.empty_root_hash
    };

    // This is trie's post state hash after committing genesis effects on top of
    // empty trie. Step 1b)
    let genesis_transforms_hash = builder
        .commit_effects(empty_root_hash.to_vec(), genesis_transforms)
        .get_post_state_hash();

    // They should match.
    assert_eq!(genesis_run_hash, genesis_transforms_hash);

    // Step 2.
    builder
        .exec(
            GENESIS_ADDR,
            "local_state.wasm",
            DEFAULT_BLOCK_TIME,
            [1u8; 32],
        )
        .commit()
        .expect_success();

    // No step 3.
    // Step 4.
    builder.run_genesis(GENESIS_ADDR, HashMap::new());

    // Step 4a)
    let second_genesis_run_hash = builder.get_genesis_hash();
    let second_genesis_transforms = builder.get_genesis_transforms().clone();

    // Step 4b)
    let second_genesis_transforms_hash = builder
        .commit_effects(empty_root_hash.to_vec(), second_genesis_transforms)
        .get_post_state_hash();

    assert_eq!(second_genesis_run_hash, second_genesis_transforms_hash);

    assert_eq!(second_genesis_run_hash, genesis_run_hash);
    assert_eq!(second_genesis_transforms_hash, genesis_transforms_hash);
}
