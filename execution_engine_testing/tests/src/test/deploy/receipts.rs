use std::collections::{BTreeMap, BTreeSet};

use once_cell::sync::Lazy;

use casper_engine_test_support::{
    ExecuteRequestBuilder, LmdbWasmTestBuilder, TransferRequestBuilder, DEFAULT_ACCOUNT_ADDR,
    LOCAL_GENESIS_REQUEST,
};
use casper_types::{
    account::AccountHash, runtime_args, system::mint, AccessRights, Gas, InitiatorAddr, PublicKey,
    SecretKey, Transfer, TransferV2, U512,
};

const CONTRACT_TRANSFER_PURSE_TO_ACCOUNT: &str = "transfer_purse_to_account.wasm";
const CONTRACT_TRANSFER_PURSE_TO_ACCOUNT_WITH_ID: &str = "transfer_purse_to_account_with_id.wasm";
const TRANSFER_ARG_TARGET: &str = "target";
const TRANSFER_ARG_AMOUNT: &str = "amount";
const TRANSFER_ARG_ID: &str = "id";

const CONTRACT_TRANSFER_PURSE_TO_ACCOUNTS: &str = "transfer_purse_to_accounts.wasm";
const TRANSFER_ARG_TARGETS: &str = "targets";

const CONTRACT_TRANSFER_PURSE_TO_ACCOUNTS_STORED: &str = "transfer_purse_to_accounts_stored.wasm";
const CONTRACT_TRANSFER_PURSE_TO_ACCOUNTS_SUBCALL: &str = "transfer_purse_to_accounts_subcall.wasm";

const HASH_KEY_NAME: &str = "transfer_purse_to_accounts_hash";
const PURSE_NAME: &str = "purse";

static ALICE_KEY: Lazy<PublicKey> = Lazy::new(|| {
    let secret_key = SecretKey::ed25519_from_bytes([3; 32]).unwrap();
    PublicKey::from(&secret_key)
});
static BOB_KEY: Lazy<PublicKey> = Lazy::new(|| {
    let secret_key = SecretKey::ed25519_from_bytes([5; 32]).unwrap();
    PublicKey::from(&secret_key)
});
static CAROL_KEY: Lazy<PublicKey> = Lazy::new(|| {
    let secret_key = SecretKey::ed25519_from_bytes([7; 32]).unwrap();
    PublicKey::from(&secret_key)
});

static ALICE_ADDR: Lazy<AccountHash> = Lazy::new(|| AccountHash::from(&*ALICE_KEY));
static BOB_ADDR: Lazy<AccountHash> = Lazy::new(|| AccountHash::from(&*BOB_KEY));
static CAROL_ADDR: Lazy<AccountHash> = Lazy::new(|| AccountHash::from(&*CAROL_KEY));

static TRANSFER_AMOUNT_1: Lazy<U512> = Lazy::new(|| U512::from(100_100_000));
static TRANSFER_AMOUNT_2: Lazy<U512> = Lazy::new(|| U512::from(200_100_000));
static TRANSFER_AMOUNT_3: Lazy<U512> = Lazy::new(|| U512::from(300_100_000));

#[ignore]
#[test]
fn should_record_wasmless_transfer() {
    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    let id = 0;

    let transfer_request = TransferRequestBuilder::new(*TRANSFER_AMOUNT_1, *ALICE_ADDR)
        .with_transfer_id(id)
        .build();

    let txn_hash = transfer_request.transaction_hash();

    builder
        .transfer_and_commit(transfer_request)
        .expect_success();

    let default_account = builder
        .get_entity_by_account_hash(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have default account");

    let alice_account = builder
        .get_entity_by_account_hash(*ALICE_ADDR)
        .expect("should have Alice's account");

    let alice_attenuated_main_purse = alice_account
        .main_purse()
        .with_access_rights(AccessRights::ADD);

    let execution_result = builder
        .get_last_exec_result()
        .expect("Expected execution results.");

    let transfers = execution_result.transfers();
    assert_eq!(transfers.len(), 1);

    let Transfer::V2(transfer) = transfers[0].clone() else {
        panic!("wrong transfer variant");
    };

    assert_eq!(transfer.transaction_hash, txn_hash);
    assert_eq!(
        transfer.from,
        InitiatorAddr::AccountHash(*DEFAULT_ACCOUNT_ADDR)
    );
    assert_eq!(transfer.to, Some(*ALICE_ADDR));
    assert_eq!(transfer.source, default_account.main_purse());
    assert_eq!(transfer.target, alice_attenuated_main_purse);
    assert_eq!(transfer.amount, *TRANSFER_AMOUNT_1);
    assert_eq!(
        transfer.gas,
        Gas::from(
            builder
                .chainspec()
                .system_costs_config
                .mint_costs()
                .transfer
        )
    );
    assert_eq!(transfer.id, Some(id));
}

#[ignore]
#[test]
fn should_record_wasm_transfer() {
    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    let transfer_request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_TRANSFER_PURSE_TO_ACCOUNT,
        runtime_args! {
            TRANSFER_ARG_TARGET => *ALICE_ADDR,
            TRANSFER_ARG_AMOUNT => *TRANSFER_AMOUNT_1
        },
    )
    .build();

    let txn_hash = transfer_request.session.transaction_hash;

    builder.exec(transfer_request).commit().expect_success();

    let default_account = builder
        .get_entity_by_account_hash(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have default account");

    let alice_account = builder
        .get_entity_by_account_hash(*ALICE_ADDR)
        .expect("should have Alice's account");

    let alice_attenuated_main_purse = alice_account
        .main_purse()
        .with_access_rights(AccessRights::ADD);

    let execution_result = builder
        .get_last_exec_result()
        .expect("Expected execution results.");

    assert_ne!(execution_result.consumed(), Gas::zero());
    let transfers = execution_result.transfers();
    assert_eq!(transfers.len(), 1);

    let Transfer::V2(transfer) = transfers[0].clone() else {
        panic!("wrong transfer variant");
    };

    assert_eq!(transfer.transaction_hash, txn_hash);
    assert_eq!(
        transfer.from,
        InitiatorAddr::AccountHash(*DEFAULT_ACCOUNT_ADDR)
    );
    assert_eq!(transfer.source, default_account.main_purse());
    assert_eq!(transfer.target, alice_attenuated_main_purse);
    assert_eq!(transfer.amount, *TRANSFER_AMOUNT_1);
    assert_eq!(
        transfer.gas,
        Gas::from(
            builder
                .chainspec()
                .system_costs_config
                .mint_costs()
                .transfer
        )
    )
}

#[ignore]
#[test]
fn should_record_wasm_transfer_with_id() {
    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    let id = Some(0);

    let transfer_request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_TRANSFER_PURSE_TO_ACCOUNT_WITH_ID,
        runtime_args! {
            TRANSFER_ARG_TARGET => *ALICE_ADDR,
            TRANSFER_ARG_AMOUNT => *TRANSFER_AMOUNT_1,
            TRANSFER_ARG_ID => id
        },
    )
    .build();

    let txn_hash = transfer_request.session.transaction_hash;

    builder.exec(transfer_request).commit().expect_success();

    let default_account = builder
        .get_entity_by_account_hash(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have default account");

    let alice_account = builder
        .get_entity_by_account_hash(*ALICE_ADDR)
        .expect("should have Alice's account");

    let alice_attenuated_main_purse = alice_account
        .main_purse()
        .with_access_rights(AccessRights::ADD);

    let execution_result = builder
        .get_last_exec_result()
        .expect("Expected execution results.");

    assert_ne!(execution_result.consumed(), Gas::zero());
    let transfers = execution_result.transfers();
    assert_eq!(transfers.len(), 1);

    let Transfer::V2(transfer) = transfers[0].clone() else {
        panic!("wrong transfer variant");
    };

    assert_eq!(transfer.transaction_hash, txn_hash);
    assert_eq!(
        transfer.from,
        InitiatorAddr::AccountHash(*DEFAULT_ACCOUNT_ADDR)
    );
    assert_eq!(transfer.source, default_account.main_purse());
    assert_eq!(transfer.target, alice_attenuated_main_purse);
    assert_eq!(transfer.amount, *TRANSFER_AMOUNT_1);
    assert_eq!(
        transfer.gas,
        Gas::from(
            builder
                .chainspec()
                .system_costs_config
                .mint_costs()
                .transfer
        )
    );
    assert_eq!(transfer.id, id);
}

#[ignore]
#[test]
fn should_record_wasm_transfers() {
    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    let alice_id = Some(0);
    let bob_id = Some(1);
    let carol_id = Some(2);

    let targets: BTreeMap<AccountHash, (U512, Option<u64>)> = {
        let mut tmp = BTreeMap::new();
        tmp.insert(*ALICE_ADDR, (*TRANSFER_AMOUNT_1, alice_id));
        tmp.insert(*BOB_ADDR, (*TRANSFER_AMOUNT_2, bob_id));
        tmp.insert(*CAROL_ADDR, (*TRANSFER_AMOUNT_3, carol_id));
        tmp
    };

    let transfer_request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_TRANSFER_PURSE_TO_ACCOUNTS,
        runtime_args! {
            mint::ARG_AMOUNT => *TRANSFER_AMOUNT_1 + *TRANSFER_AMOUNT_2 + *TRANSFER_AMOUNT_3,
            TRANSFER_ARG_TARGETS => targets,
        },
    )
    .build();

    let txn_hash = transfer_request.session.transaction_hash;

    builder.exec(transfer_request).commit().expect_success();

    let default_account = builder
        .get_entity_by_account_hash(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have default account");

    let alice_account = builder
        .get_entity_by_account_hash(*ALICE_ADDR)
        .expect("should have Alice's account");

    let bob_account = builder
        .get_entity_by_account_hash(*BOB_ADDR)
        .expect("should have Bob's account");

    let carol_account = builder
        .get_entity_by_account_hash(*CAROL_ADDR)
        .expect("should have Carol's account");

    let alice_attenuated_main_purse = alice_account
        .main_purse()
        .with_access_rights(AccessRights::ADD);

    let bob_attenuated_main_purse = bob_account
        .main_purse()
        .with_access_rights(AccessRights::ADD);

    let carol_attenuated_main_purse = carol_account
        .main_purse()
        .with_access_rights(AccessRights::ADD);

    let execution_result = builder
        .get_last_exec_result()
        .expect("Expected execution results.");

    assert_ne!(execution_result.consumed(), Gas::zero());
    const EXPECTED_LENGTH: usize = 3;
    assert_eq!(execution_result.transfers().len(), EXPECTED_LENGTH);
    assert_eq!(
        execution_result
            .transfers()
            .iter()
            .cloned()
            .collect::<BTreeSet<Transfer>>()
            .len(),
        EXPECTED_LENGTH
    );

    let transfers: BTreeSet<Transfer> = {
        let mut tmp = BTreeSet::new();
        for transfer in execution_result.transfers() {
            tmp.insert(transfer.clone());
        }
        tmp
    };

    assert_eq!(transfers.len(), EXPECTED_LENGTH);

    assert!(transfers.contains(&Transfer::V2(TransferV2 {
        transaction_hash: txn_hash,
        from: InitiatorAddr::AccountHash(*DEFAULT_ACCOUNT_ADDR),
        to: Some(*ALICE_ADDR),
        source: default_account.main_purse(),
        target: alice_attenuated_main_purse,
        amount: *TRANSFER_AMOUNT_1,
        gas: Gas::from(
            builder
                .chainspec()
                .system_costs_config
                .mint_costs()
                .transfer
        ),
        id: alice_id,
    })));

    assert!(transfers.contains(&Transfer::V2(TransferV2 {
        transaction_hash: txn_hash,
        from: InitiatorAddr::AccountHash(*DEFAULT_ACCOUNT_ADDR),
        to: Some(*BOB_ADDR),
        source: default_account.main_purse(),
        target: bob_attenuated_main_purse,
        amount: *TRANSFER_AMOUNT_2,
        gas: Gas::from(
            builder
                .chainspec()
                .system_costs_config
                .mint_costs()
                .transfer
        ),
        id: bob_id,
    })));

    assert!(transfers.contains(&Transfer::V2(TransferV2 {
        transaction_hash: txn_hash,
        from: InitiatorAddr::AccountHash(*DEFAULT_ACCOUNT_ADDR),
        to: Some(*CAROL_ADDR),
        source: default_account.main_purse(),
        target: carol_attenuated_main_purse,
        amount: *TRANSFER_AMOUNT_3,
        gas: Gas::from(
            builder
                .chainspec()
                .system_costs_config
                .mint_costs()
                .transfer
        ),
        id: carol_id,
    })));
}

#[ignore]
#[test]
fn should_record_wasm_transfers_with_subcall() {
    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    let alice_id = Some(0);
    let bob_id = Some(1);
    let carol_id = Some(2);

    let total_transfer_amount = *TRANSFER_AMOUNT_1 + *TRANSFER_AMOUNT_2 + *TRANSFER_AMOUNT_3;

    let store_request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_TRANSFER_PURSE_TO_ACCOUNTS_STORED,
        runtime_args! {
            mint::ARG_AMOUNT => total_transfer_amount,
        },
    )
    .build();

    let targets: BTreeMap<AccountHash, (U512, Option<u64>)> = {
        let mut tmp = BTreeMap::new();
        tmp.insert(*ALICE_ADDR, (*TRANSFER_AMOUNT_1, alice_id));
        tmp.insert(*BOB_ADDR, (*TRANSFER_AMOUNT_2, bob_id));
        tmp.insert(*CAROL_ADDR, (*TRANSFER_AMOUNT_3, carol_id));
        tmp
    };

    let transfer_request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_TRANSFER_PURSE_TO_ACCOUNTS_SUBCALL,
        runtime_args! {
            mint::ARG_AMOUNT => total_transfer_amount,
            TRANSFER_ARG_TARGETS => targets,
        },
    )
    .build();

    let transfer_txn_hash = transfer_request.session.transaction_hash;

    builder.exec(store_request).commit().expect_success();
    builder.exec(transfer_request).commit().expect_success();

    let default_account = builder
        .get_entity_with_named_keys_by_account_hash(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have default account");

    let entity_hash = default_account
        .named_keys()
        .get(HASH_KEY_NAME)
        .unwrap()
        .into_entity_hash()
        .expect("should have contract hash");

    let contract = builder
        .get_entity_with_named_keys_by_entity_hash(entity_hash)
        .expect("should have stored contract");

    let contract_purse = contract
        .named_keys()
        .get(PURSE_NAME)
        .unwrap()
        .into_uref()
        .expect("should have purse");

    let alice_account = builder
        .get_entity_by_account_hash(*ALICE_ADDR)
        .expect("should have Alice's account");

    let bob_account = builder
        .get_entity_by_account_hash(*BOB_ADDR)
        .expect("should have Bob's account");

    let carol_account = builder
        .get_entity_by_account_hash(*CAROL_ADDR)
        .expect("should have Carol's account");

    let alice_attenuated_main_purse = alice_account
        .main_purse()
        .with_access_rights(AccessRights::ADD);

    let bob_attenuated_main_purse = bob_account
        .main_purse()
        .with_access_rights(AccessRights::ADD);

    let carol_attenuated_main_purse = carol_account
        .main_purse()
        .with_access_rights(AccessRights::ADD);

    let execution_result = builder
        .get_last_exec_result()
        .expect("Expected execution results.");

    /*
    assert_eq!(txn_info.transaction_hash, transfer_txn_hash);
    assert_eq!(
        txn_info.from,
        InitiatorAddr::AccountHash(*DEFAULT_ACCOUNT_ADDR)
    );
    assert_eq!(txn_info.source, default_account.main_purse());
    */

    assert_ne!(execution_result.consumed(), Gas::zero());
    const EXPECTED_LENGTH: usize = 6;
    assert_eq!(execution_result.transfers().len(), EXPECTED_LENGTH);
    assert_eq!(
        execution_result
            .transfers()
            .iter()
            .cloned()
            .collect::<BTreeSet<Transfer>>()
            .len(),
        EXPECTED_LENGTH
    );

    let transfer_counts: BTreeMap<Transfer, usize> = {
        let mut tmp = BTreeMap::new();
        for transfer in execution_result.transfers() {
            tmp.entry(transfer.clone())
                .and_modify(|i| *i += 1)
                .or_insert(1);
        }
        tmp
    };

    let session_expected_alice = Transfer::V2(TransferV2 {
        transaction_hash: transfer_txn_hash,
        from: InitiatorAddr::AccountHash(*DEFAULT_ACCOUNT_ADDR),
        to: Some(*ALICE_ADDR),
        source: default_account.main_purse(),
        target: alice_attenuated_main_purse,
        amount: *TRANSFER_AMOUNT_1,
        gas: Gas::from(
            builder
                .chainspec()
                .system_costs_config
                .mint_costs()
                .transfer,
        ),
        id: alice_id,
    });

    let session_expected_bob = Transfer::V2(TransferV2 {
        transaction_hash: transfer_txn_hash,
        from: InitiatorAddr::AccountHash(*DEFAULT_ACCOUNT_ADDR),
        to: Some(*BOB_ADDR),
        source: default_account.main_purse(),
        target: bob_attenuated_main_purse,
        amount: *TRANSFER_AMOUNT_2,
        gas: Gas::from(
            builder
                .chainspec()
                .system_costs_config
                .mint_costs()
                .transfer,
        ),
        id: bob_id,
    });

    let session_expected_carol = Transfer::V2(TransferV2 {
        transaction_hash: transfer_txn_hash,
        from: InitiatorAddr::AccountHash(*DEFAULT_ACCOUNT_ADDR),
        to: Some(*CAROL_ADDR),
        source: default_account.main_purse(),
        target: carol_attenuated_main_purse,
        amount: *TRANSFER_AMOUNT_3,
        gas: Gas::from(
            builder
                .chainspec()
                .system_costs_config
                .mint_costs()
                .transfer,
        ),
        id: carol_id,
    });

    const SESSION_EXPECTED_COUNT: Option<usize> = Some(1);
    for (i, expected) in [
        session_expected_alice,
        session_expected_bob,
        session_expected_carol,
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(
            transfer_counts.get(expected).cloned(),
            SESSION_EXPECTED_COUNT,
            "transfer {} has unexpected value",
            i
        );
    }

    let stored_expected_alice = Transfer::V2(TransferV2 {
        transaction_hash: transfer_txn_hash,
        from: InitiatorAddr::AccountHash(*DEFAULT_ACCOUNT_ADDR),
        to: Some(*ALICE_ADDR),
        source: contract_purse,
        target: alice_attenuated_main_purse,
        amount: *TRANSFER_AMOUNT_1,
        gas: Gas::from(
            builder
                .chainspec()
                .system_costs_config
                .mint_costs()
                .transfer,
        ),
        id: alice_id,
    });

    let stored_expected_bob = Transfer::V2(TransferV2 {
        transaction_hash: transfer_txn_hash,
        from: InitiatorAddr::AccountHash(*DEFAULT_ACCOUNT_ADDR),
        to: Some(*BOB_ADDR),
        source: contract_purse,
        target: bob_attenuated_main_purse,
        amount: *TRANSFER_AMOUNT_2,
        gas: Gas::from(
            builder
                .chainspec()
                .system_costs_config
                .mint_costs()
                .transfer,
        ),
        id: bob_id,
    });

    let stored_expected_carol = Transfer::V2(TransferV2 {
        transaction_hash: transfer_txn_hash,
        from: InitiatorAddr::AccountHash(*DEFAULT_ACCOUNT_ADDR),
        to: Some(*CAROL_ADDR),
        source: contract_purse,
        target: carol_attenuated_main_purse,
        amount: *TRANSFER_AMOUNT_3,
        gas: Gas::from(
            builder
                .chainspec()
                .system_costs_config
                .mint_costs()
                .transfer,
        ),
        id: carol_id,
    });

    const STORED_EXPECTED_COUNT: Option<usize> = Some(1);
    for (i, expected) in [
        stored_expected_alice,
        stored_expected_bob,
        stored_expected_carol,
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(
            transfer_counts.get(expected).cloned(),
            STORED_EXPECTED_COUNT,
            "transfer {} has unexpected value",
            i
        );
    }
}
