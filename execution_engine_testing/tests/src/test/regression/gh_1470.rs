use std::collections::BTreeMap;

use casper_engine_test_support::{
    ExecuteRequestBuilder, LmdbWasmTestBuilder, TransferRequestBuilder, UpgradeRequestBuilder,
    DEFAULT_ACCOUNT_ADDR, DEFAULT_ACCOUNT_PUBLIC_KEY, LOCAL_GENESIS_REQUEST,
    MINIMUM_ACCOUNT_CREATION_BALANCE,
};
use casper_execution_engine::{engine_state::Error, execution::ExecError};
use casper_types::{
    account::AccountHash,
    runtime_args,
    system::{auction, auction::DelegationRate},
    AccessRights, AddressableEntityHash, CLTyped, CLValue, Digest, EraId, HoldBalanceHandling, Key,
    PackageHash, ProtocolVersion, RuntimeArgs, StoredValue, StoredValueTypeMismatch,
    SystemHashRegistry, Timestamp, URef, U512,
};

use crate::lmdb_fixture;

const ACCOUNT_1_ADDR: AccountHash = AccountHash::new([1u8; 32]);
const GH_1470_REGRESSION: &str = "gh_1470_regression.wasm";
const GH_1470_REGRESSION_CALL: &str = "gh_1470_regression_call.wasm";
const DEFAULT_ACTIVATION_POINT: EraId = EraId::new(1);

const CONTRACT_ADD_BID: &str = "add_bid.wasm";
const BOND_AMOUNT: u64 = 42;
const BID_DELEGATION_RATE: DelegationRate = auction::DELEGATION_RATE_DENOMINATOR;

const CONTRACT_TRANSFER_TO_ACCOUNT: &str = "transfer_to_account_u512.wasm";
const ARG_AMOUNT: &str = "amount";
const ARG_TARGET: &str = "target";

const PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::V1_0_0;

fn setup() -> LmdbWasmTestBuilder {
    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    let transfer = TransferRequestBuilder::new(MINIMUM_ACCOUNT_CREATION_BALANCE, ACCOUNT_1_ADDR)
        .with_transfer_id(42)
        .build();

    builder.transfer_and_commit(transfer).expect_success();

    let sem_ver = PROTOCOL_VERSION.value();
    let new_protocol_version =
        ProtocolVersion::from_parts(sem_ver.major, sem_ver.minor, sem_ver.patch + 1);

    let updated_chainspec = builder
        .chainspec()
        .clone()
        .with_strict_argument_checking(true);

    let mut upgrade_request = {
        UpgradeRequestBuilder::new()
            .with_current_protocol_version(PROTOCOL_VERSION)
            .with_new_protocol_version(new_protocol_version)
            .with_activation_point(DEFAULT_ACTIVATION_POINT)
            .build()
    };

    builder
        .with_chainspec(updated_chainspec)
        .upgrade(&mut upgrade_request)
        .expect_upgrade_success();

    builder
}

fn apply_global_state_update(
    builder: &LmdbWasmTestBuilder,
    post_state_hash: Digest,
) -> BTreeMap<Key, StoredValue> {
    let key = URef::new([0u8; 32], AccessRights::all()).into();

    let system_contract_hashes = builder
        .query(Some(post_state_hash), key, &Vec::new())
        .expect("Must have stored system contract hashes")
        .as_cl_value()
        .expect("must be CLValue")
        .clone()
        .into_t::<SystemHashRegistry>()
        .expect("must convert to btree map");

    let mut global_state_update = BTreeMap::<Key, StoredValue>::new();
    let registry = CLValue::from_t(system_contract_hashes)
        .expect("must convert to StoredValue")
        .into();

    global_state_update.insert(Key::SystemEntityRegistry, registry);

    global_state_update
}

#[ignore]
#[test]
fn gh_1470_call_contract_should_verify_group_access() {
    let mut builder = setup();

    let exec_request_1 = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        GH_1470_REGRESSION,
        RuntimeArgs::new(),
    )
    .build();

    builder.exec(exec_request_1).expect_success().commit();

    let account = builder
        .get_entity_with_named_keys_by_account_hash(*DEFAULT_ACCOUNT_ADDR)
        .expect("must have default contract package");

    let entity_hash_key = account
        .named_keys()
        .get(gh_1470_regression::CONTRACT_HASH_NAME)
        .cloned()
        .unwrap();
    let entity_hash = entity_hash_key
        .into_entity_hash_addr()
        .map(AddressableEntityHash::new)
        .unwrap();
    let package_hash_key = account
        .named_keys()
        .get(gh_1470_regression::PACKAGE_HASH_NAME)
        .cloned()
        .unwrap();
    let package_hash = package_hash_key
        .into_package_addr()
        .map(PackageHash::new)
        .unwrap();

    let call_contract_request = {
        let args = runtime_args! {
            gh_1470_regression_call::ARG_TEST_METHOD => gh_1470_regression_call::METHOD_CALL_DO_NOTHING,
            gh_1470_regression_call::ARG_CONTRACT_HASH => entity_hash,
        };
        ExecuteRequestBuilder::standard(ACCOUNT_1_ADDR, GH_1470_REGRESSION_CALL, args).build()
    };

    builder.exec(call_contract_request).commit();

    let exec_result = builder
        .get_last_exec_result()
        .expect("should have last response");
    let call_contract_error = exec_result.error().cloned().expect("should have error");

    let call_versioned_contract_request = {
        let args = runtime_args! {
            gh_1470_regression_call::ARG_TEST_METHOD => gh_1470_regression_call::METHOD_CALL_VERSIONED_DO_NOTHING,
            gh_1470_regression_call::ARG_CONTRACT_PACKAGE_HASH => package_hash,
        };
        ExecuteRequestBuilder::standard(ACCOUNT_1_ADDR, GH_1470_REGRESSION_CALL, args).build()
    };

    builder.exec(call_versioned_contract_request).commit();

    let exec_result = builder
        .get_last_exec_result()
        .expect("should have last response");
    let call_versioned_contract_error = exec_result.error().expect("should have error");

    match (&call_contract_error, &call_versioned_contract_error) {
        (Error::Exec(ExecError::InvalidContext), Error::Exec(ExecError::InvalidContext)) => (),
        _ => panic!("Both variants should raise same error."),
    }

    assert!(matches!(
        call_versioned_contract_error,
        Error::Exec(ExecError::InvalidContext)
    ));
    assert!(matches!(
        call_contract_error,
        Error::Exec(ExecError::InvalidContext)
    ));
}

// #[ignore]
// #[test]
// fn gh_1470_call_contract_should_verify_invalid_arguments_length() {
//     let mut builder = setup();

//     let exec_request_1 = ExecuteRequestBuilder::standard(
//         *DEFAULT_ACCOUNT_ADDR,
//         GH_1470_REGRESSION,
//         RuntimeArgs::new(),
//     )
//     .build();

//     builder.exec(exec_request_1).expect_success().commit();

//     let account_stored_value = builder
//         .query(None, Key::Account(*DEFAULT_ACCOUNT_ADDR), &[])
//         .unwrap();
//     let account = account_stored_value.as_account().cloned().unwrap();

//     let contract_hash_key = account
//         .named_keys()
//         .get(gh_1470_regression::CONTRACT_HASH_NAME)
//         .cloned()
//         .unwrap();
//     let contract_hash = contract_hash_key
//         .into_hash()
//         .map(ContractHash::new)
//         .unwrap();
//     let contract_package_hash_key = account
//         .named_keys()
//         .get(gh_1470_regression::CONTRACT_PACKAGE_HASH_NAME)
//         .cloned()
//         .unwrap();
//     let contract_package_hash = contract_package_hash_key
//         .into_hash()
//         .map(ContractPackageHash::new)
//         .unwrap();

//     let call_contract_request = {
//         let args = runtime_args! {
//             gh_1470_regression_call::ARG_TEST_METHOD =>
// gh_1470_regression_call::METHOD_CALL_DO_NOTHING_NO_ARGS,
// gh_1470_regression_call::ARG_CONTRACT_HASH => contract_hash,         };
//         ExecuteRequestBuilder::standard(*DEFAULT_ACCOUNT_ADDR, GH_1470_REGRESSION_CALL, args)
//             .build()
//     };

//     builder.exec(call_contract_request).commit();

//     let response = builder
//         .get_last_exec_result()
//         .expect("should have last response");
//     assert_eq!(response.len(), 1);
//     let exec_response = response.last().expect("should have response");
//     let call_contract_error = exec_response
//         .as_error()
//         .cloned()
//         .expect("should have error");

//     let call_versioned_contract_request = {
//         let args = runtime_args! {
//             gh_1470_regression_call::ARG_TEST_METHOD =>
// gh_1470_regression_call::METHOD_CALL_VERSIONED_DO_NOTHING_NO_ARGS,
// gh_1470_regression_call::ARG_CONTRACT_PACKAGE_HASH => contract_package_hash,         };
//         ExecuteRequestBuilder::standard(*DEFAULT_ACCOUNT_ADDR, GH_1470_REGRESSION_CALL, args)
//             .build()
//     };

//     builder.exec(call_versioned_contract_request).commit();

//     let response = builder
//         .get_last_exec_result()
//         .expect("should have last response");
//     assert_eq!(response.len(), 1);
//     let exec_response = response.last().expect("should have response");
//     let call_versioned_contract_error = exec_response.as_error().expect("should have error");

//     match (&call_contract_error, &call_versioned_contract_error) {
//         (
//             Error::Exec(ExecError::MissingArgument { name: lhs_name }),
//             Error::Exec(ExecError::MissingArgument { name: rhs_name }),
//         ) if lhs_name == rhs_name => (),
//         _ => panic!(
//             "Both variants should raise same error: lhs={:?} rhs={:?}",
//             call_contract_error, call_versioned_contract_error
//         ),
//     }

//     assert!(
//         matches!(
//             &call_versioned_contract_error,
//             Error::Exec(ExecError::MissingArgument {
//                 name,
//             })
//             if name == gh_1470_regression::ARG1
//         ),
//         "{:?}",
//         call_versioned_contract_error
//     );
//     assert!(
//         matches!(
//             &call_contract_error,
//             Error::Exec(ExecError::MissingArgument {
//                 name,
//             })
//             if name == gh_1470_regression::ARG1
//         ),
//         "{:?}",
//         call_contract_error
//     );
// }

#[ignore]
#[test]
fn gh_1470_call_contract_should_ignore_optional_args() {
    let mut builder = setup();

    let exec_request_1 = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        GH_1470_REGRESSION,
        RuntimeArgs::new(),
    )
    .build();

    builder.exec(exec_request_1).expect_success().commit();

    let account = builder
        .get_entity_with_named_keys_by_account_hash(*DEFAULT_ACCOUNT_ADDR)
        .expect("must have default contract package");

    let contract_hash_key = account
        .named_keys()
        .get(gh_1470_regression::CONTRACT_HASH_NAME)
        .cloned()
        .unwrap();
    let entity_hash = contract_hash_key
        .into_entity_hash_addr()
        .map(AddressableEntityHash::new)
        .unwrap();
    let package_hash_key = account
        .named_keys()
        .get(gh_1470_regression::PACKAGE_HASH_NAME)
        .cloned()
        .unwrap();
    let package_hash = package_hash_key
        .into_package_addr()
        .map(PackageHash::new)
        .unwrap();

    let call_contract_request = {
        let args = runtime_args! {
            gh_1470_regression_call::ARG_TEST_METHOD => gh_1470_regression_call::METHOD_CALL_DO_NOTHING_NO_OPTIONALS,
            gh_1470_regression_call::ARG_CONTRACT_HASH => entity_hash,
        };
        ExecuteRequestBuilder::standard(*DEFAULT_ACCOUNT_ADDR, GH_1470_REGRESSION_CALL, args)
            .build()
    };

    builder
        .exec(call_contract_request)
        .expect_success()
        .commit();

    let call_versioned_contract_request = {
        let args = runtime_args! {
            gh_1470_regression_call::ARG_TEST_METHOD => gh_1470_regression_call::METHOD_CALL_VERSIONED_DO_NOTHING_NO_OPTIONALS,
            gh_1470_regression_call::ARG_CONTRACT_PACKAGE_HASH => package_hash,
        };
        ExecuteRequestBuilder::standard(*DEFAULT_ACCOUNT_ADDR, GH_1470_REGRESSION_CALL, args)
            .build()
    };

    builder
        .exec(call_versioned_contract_request)
        .expect_success()
        .commit();
}

#[ignore]
#[test]
fn gh_1470_call_contract_should_not_accept_extra_args() {
    let mut builder = setup();

    let exec_request_1 = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        GH_1470_REGRESSION,
        RuntimeArgs::new(),
    )
    .build();

    builder.exec(exec_request_1).expect_success().commit();

    let account = builder
        .get_entity_with_named_keys_by_account_hash(*DEFAULT_ACCOUNT_ADDR)
        .expect("must have default contract package");

    let contract_hash_key = account
        .named_keys()
        .get(gh_1470_regression::CONTRACT_HASH_NAME)
        .cloned()
        .unwrap();
    let entity_hash = contract_hash_key
        .into_entity_hash_addr()
        .map(AddressableEntityHash::new)
        .unwrap();
    let package_hash_key = account
        .named_keys()
        .get(gh_1470_regression::PACKAGE_HASH_NAME)
        .cloned()
        .unwrap();
    let package_hash = package_hash_key
        .into_package_addr()
        .map(PackageHash::new)
        .unwrap();

    let call_contract_request = {
        let args = runtime_args! {
            gh_1470_regression_call::ARG_TEST_METHOD => gh_1470_regression_call::METHOD_CALL_DO_NOTHING_EXTRA,
            gh_1470_regression_call::ARG_CONTRACT_HASH => entity_hash,
        };
        ExecuteRequestBuilder::standard(*DEFAULT_ACCOUNT_ADDR, GH_1470_REGRESSION_CALL, args)
            .build()
    };

    builder
        .exec(call_contract_request)
        .expect_success()
        .commit();

    let call_versioned_contract_request = {
        let args = runtime_args! {
            gh_1470_regression_call::ARG_TEST_METHOD => gh_1470_regression_call::METHOD_CALL_VERSIONED_DO_NOTHING_EXTRA,
            gh_1470_regression_call::ARG_CONTRACT_PACKAGE_HASH => package_hash,
        };
        ExecuteRequestBuilder::standard(*DEFAULT_ACCOUNT_ADDR, GH_1470_REGRESSION_CALL, args)
            .build()
    };

    builder
        .exec(call_versioned_contract_request)
        .expect_success()
        .commit();
}

#[ignore]
#[test]
fn gh_1470_call_contract_should_verify_wrong_argument_types() {
    let mut builder = setup();

    let exec_request_1 = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        GH_1470_REGRESSION,
        RuntimeArgs::new(),
    )
    .build();

    builder.exec(exec_request_1).expect_success().commit();

    let account = builder
        .get_entity_with_named_keys_by_account_hash(*DEFAULT_ACCOUNT_ADDR)
        .expect("must have contract");

    let entity_hash_key = account
        .named_keys()
        .get(gh_1470_regression::CONTRACT_HASH_NAME)
        .cloned()
        .unwrap();
    let entity_hash = entity_hash_key
        .into_entity_hash_addr()
        .map(AddressableEntityHash::new)
        .unwrap();
    let package_hash_key = account
        .named_keys()
        .get(gh_1470_regression::PACKAGE_HASH_NAME)
        .cloned()
        .unwrap();
    let package_hash = package_hash_key
        .into_package_addr()
        .map(PackageHash::new)
        .unwrap();

    let call_contract_request = {
        let args = runtime_args! {
                    gh_1470_regression_call::ARG_TEST_METHOD =>
        gh_1470_regression_call::METHOD_CALL_DO_NOTHING_TYPE_MISMATCH,
        gh_1470_regression_call::ARG_CONTRACT_HASH => entity_hash,         };
        ExecuteRequestBuilder::standard(*DEFAULT_ACCOUNT_ADDR, GH_1470_REGRESSION_CALL, args)
            .build()
    };

    builder.exec(call_contract_request).commit();

    let exec_result = builder
        .get_last_exec_result()
        .expect("should have last response");
    let call_contract_error = exec_result.error().cloned().expect("should have error");

    let call_versioned_contract_request = {
        let args = runtime_args! {
                    gh_1470_regression_call::ARG_TEST_METHOD =>
        gh_1470_regression_call::METHOD_CALL_VERSIONED_DO_NOTHING_TYPE_MISMATCH,
        gh_1470_regression_call::ARG_CONTRACT_PACKAGE_HASH => package_hash,         };
        ExecuteRequestBuilder::standard(*DEFAULT_ACCOUNT_ADDR, GH_1470_REGRESSION_CALL, args)
            .build()
    };

    builder.exec(call_versioned_contract_request).commit();

    let exec_result = builder
        .get_last_exec_result()
        .expect("should have last response");
    let call_versioned_contract_error = exec_result.error().expect("should have error");

    let expected = gh_1470_regression::Arg1Type::cl_type();
    let found = gh_1470_regression::Arg3Type::cl_type();

    let expected_type_mismatch =
        StoredValueTypeMismatch::new(format!("{:?}", expected), format!("{:?}", found));

    match (&call_contract_error, &call_versioned_contract_error) {
        (
            Error::Exec(ExecError::TypeMismatch(lhs_type_mismatch)),
            Error::Exec(ExecError::TypeMismatch(rhs_type_mismatch)),
        ) if lhs_type_mismatch == &expected_type_mismatch
            && rhs_type_mismatch == &expected_type_mismatch => {}
        _ => panic!(
            "Both variants should raise same error: lhs={:?} rhs={:?}",
            call_contract_error, call_versioned_contract_error
        ),
    }

    assert!(matches!(
        call_versioned_contract_error,
        Error::Exec(ExecError::TypeMismatch(type_mismatch))
            if *type_mismatch == expected_type_mismatch
    ));
    assert!(matches!(
        call_contract_error,
        Error::Exec(ExecError::TypeMismatch(type_mismatch))
            if type_mismatch == expected_type_mismatch
    ));
}

#[ignore]
#[test]
fn gh_1470_call_contract_should_verify_wrong_optional_argument_types() {
    let mut builder = setup();

    let exec_request_1 = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        GH_1470_REGRESSION,
        RuntimeArgs::new(),
    )
    .build();

    builder.exec(exec_request_1).expect_success().commit();

    let account = builder
        .get_entity_with_named_keys_by_account_hash(*DEFAULT_ACCOUNT_ADDR)
        .expect("must have default contract package");

    let entity_hash_key = account
        .named_keys()
        .get(gh_1470_regression::CONTRACT_HASH_NAME)
        .cloned()
        .unwrap();
    let entity_hash = entity_hash_key
        .into_entity_hash_addr()
        .map(AddressableEntityHash::new)
        .unwrap();
    let package_hash_key = account
        .named_keys()
        .get(gh_1470_regression::PACKAGE_HASH_NAME)
        .cloned()
        .unwrap();
    let package_hash = package_hash_key
        .into_package_addr()
        .map(PackageHash::new)
        .unwrap();

    let call_contract_request = {
        let args = runtime_args! {
            gh_1470_regression_call::ARG_TEST_METHOD =>
            gh_1470_regression_call::METHOD_CALL_DO_NOTHING_OPTIONAL_TYPE_MISMATCH,
            gh_1470_regression_call::ARG_CONTRACT_HASH => entity_hash,
        };
        ExecuteRequestBuilder::standard(*DEFAULT_ACCOUNT_ADDR, GH_1470_REGRESSION_CALL, args)
            .build()
    };

    builder
        .exec(call_contract_request)
        .expect_failure()
        .commit();

    let exec_result = builder
        .get_last_exec_result()
        .expect("should have last response");
    let call_contract_error = exec_result.error().cloned().expect("should have error");

    let call_versioned_contract_request = {
        let args = runtime_args! {
            gh_1470_regression_call::ARG_TEST_METHOD => gh_1470_regression_call::METHOD_CALL_VERSIONED_DO_NOTHING_OPTIONAL_TYPE_MISMATCH,
            gh_1470_regression_call::ARG_CONTRACT_PACKAGE_HASH => package_hash,
        };
        ExecuteRequestBuilder::standard(*DEFAULT_ACCOUNT_ADDR, GH_1470_REGRESSION_CALL, args)
            .build()
    };

    builder.exec(call_versioned_contract_request).commit();

    let exec_result = builder
        .get_last_exec_result()
        .expect("should have last response");
    let call_versioned_contract_error = exec_result.error().expect("should have error");

    let expected = gh_1470_regression::Arg3Type::cl_type();
    let found = gh_1470_regression::Arg4Type::cl_type();

    let expected_type_mismatch =
        StoredValueTypeMismatch::new(format!("{:?}", expected), format!("{:?}", found));

    match (&call_contract_error, &call_versioned_contract_error) {
        (
            Error::Exec(ExecError::TypeMismatch(lhs_type_mismatch)),
            Error::Exec(ExecError::TypeMismatch(rhs_type_mismatch)),
        ) if lhs_type_mismatch == &expected_type_mismatch
            && rhs_type_mismatch == &expected_type_mismatch => {}
        _ => panic!(
            "Both variants should raise same error: lhs={:?} rhs={:?}",
            call_contract_error, call_versioned_contract_error
        ),
    }

    assert!(matches!(
        call_versioned_contract_error,
        Error::Exec(ExecError::TypeMismatch(type_mismatch))
        if *type_mismatch == expected_type_mismatch
    ));
    assert!(matches!(
        call_contract_error,
        Error::Exec(ExecError::TypeMismatch(type_mismatch))
        if type_mismatch == expected_type_mismatch
    ));
}

#[ignore]
#[test]
fn should_transfer_after_major_version_bump_from_1_2_0() {
    let (mut builder, lmdb_fixture_state, _temp_dir) =
        lmdb_fixture::builder_from_global_state_fixture(lmdb_fixture::RELEASE_1_3_1);

    let previous_protocol_version = lmdb_fixture_state.genesis_protocol_version();

    let current_protocol_version = lmdb_fixture_state.genesis_protocol_version();

    let new_protocol_version =
        ProtocolVersion::from_parts(current_protocol_version.value().major + 1, 0, 0);

    let global_state_update =
        apply_global_state_update(&builder, lmdb_fixture_state.post_state_hash);

    let mut upgrade_request = {
        UpgradeRequestBuilder::new()
            .with_current_protocol_version(previous_protocol_version)
            .with_new_protocol_version(new_protocol_version)
            .with_activation_point(DEFAULT_ACTIVATION_POINT)
            .with_global_state_update(global_state_update)
            .with_new_gas_hold_handling(HoldBalanceHandling::Accrued)
            .with_new_gas_hold_interval(1200u64)
            .build()
    };

    builder
        .with_block_time(Timestamp::now().into())
        .upgrade(&mut upgrade_request)
        .expect_upgrade_success();

    let transfer = TransferRequestBuilder::new(1, AccountHash::new([3; 32]))
        .with_transfer_id(1)
        .build();

    builder.transfer_and_commit(transfer).expect_success();
}

#[ignore]
#[test]
fn should_transfer_after_minor_version_bump_from_1_2_0() {
    let (mut builder, lmdb_fixture_state, _temp_dir) =
        lmdb_fixture::builder_from_global_state_fixture(lmdb_fixture::RELEASE_1_3_1);

    let current_protocol_version = lmdb_fixture_state.genesis_protocol_version();

    let new_protocol_version = ProtocolVersion::from_parts(
        current_protocol_version.value().major,
        current_protocol_version.value().minor + 1,
        0,
    );

    let global_state_update =
        apply_global_state_update(&builder, lmdb_fixture_state.post_state_hash);

    let mut upgrade_request = {
        UpgradeRequestBuilder::new()
            .with_current_protocol_version(current_protocol_version)
            .with_new_protocol_version(new_protocol_version)
            .with_activation_point(DEFAULT_ACTIVATION_POINT)
            .with_global_state_update(global_state_update)
            .with_new_gas_hold_handling(HoldBalanceHandling::Accrued)
            .with_new_gas_hold_interval(1200u64)
            .build()
    };

    builder
        .with_block_time(Timestamp::now().into())
        .upgrade(&mut upgrade_request)
        .expect_upgrade_success();

    let transfer = TransferRequestBuilder::new(1, AccountHash::new([3; 32]))
        .with_transfer_id(1)
        .build();
    builder.transfer_and_commit(transfer).expect_success();
}

#[ignore]
#[test]
fn should_add_bid_after_major_bump() {
    let (mut builder, lmdb_fixture_state, _temp_dir) =
        lmdb_fixture::builder_from_global_state_fixture(lmdb_fixture::RELEASE_1_3_1);

    let current_protocol_version = lmdb_fixture_state.genesis_protocol_version();

    let new_protocol_version =
        ProtocolVersion::from_parts(current_protocol_version.value().major + 1, 0, 0);

    let global_state_update =
        apply_global_state_update(&builder, lmdb_fixture_state.post_state_hash);

    let mut upgrade_request = {
        UpgradeRequestBuilder::new()
            .with_current_protocol_version(current_protocol_version)
            .with_new_protocol_version(new_protocol_version)
            .with_activation_point(DEFAULT_ACTIVATION_POINT)
            .with_global_state_update(global_state_update)
            .with_new_gas_hold_handling(HoldBalanceHandling::Accrued)
            .with_new_gas_hold_interval(1200u64)
            .build()
    };

    builder
        .with_block_time(Timestamp::now().into())
        .upgrade(&mut upgrade_request)
        .expect_upgrade_success();

    let add_bid_request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_ADD_BID,
        runtime_args! {
            auction::ARG_PUBLIC_KEY => DEFAULT_ACCOUNT_PUBLIC_KEY.clone(),
            auction::ARG_AMOUNT => U512::from(BOND_AMOUNT),
            auction::ARG_DELEGATION_RATE => BID_DELEGATION_RATE,
        },
    )
    .build();

    builder.exec(add_bid_request).expect_success().commit();

    let _default_account = builder
        .get_entity_by_account_hash(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have default account");
}

#[ignore]
#[test]
fn should_add_bid_after_minor_bump() {
    let (mut builder, lmdb_fixture_state, _temp_dir) =
        lmdb_fixture::builder_from_global_state_fixture(lmdb_fixture::RELEASE_1_3_1);

    let current_protocol_version = lmdb_fixture_state.genesis_protocol_version();

    let new_protocol_version = ProtocolVersion::from_parts(
        current_protocol_version.value().major,
        current_protocol_version.value().minor + 1,
        0,
    );

    let global_state_update =
        apply_global_state_update(&builder, lmdb_fixture_state.post_state_hash);

    let mut upgrade_request = {
        UpgradeRequestBuilder::new()
            .with_current_protocol_version(current_protocol_version)
            .with_new_protocol_version(new_protocol_version)
            .with_activation_point(DEFAULT_ACTIVATION_POINT)
            .with_global_state_update(global_state_update)
            .with_new_gas_hold_handling(HoldBalanceHandling::Accrued)
            .with_new_gas_hold_interval(1200u64)
            .build()
    };

    builder
        .with_block_time(Timestamp::now().into())
        .upgrade(&mut upgrade_request)
        .expect_upgrade_success();

    let add_bid_request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_ADD_BID,
        runtime_args! {
            auction::ARG_PUBLIC_KEY => DEFAULT_ACCOUNT_PUBLIC_KEY.clone(),
            auction::ARG_AMOUNT => U512::from(BOND_AMOUNT),
            auction::ARG_DELEGATION_RATE => BID_DELEGATION_RATE,
        },
    )
    .build();

    builder.exec(add_bid_request).expect_success().commit();

    let _default_account = builder
        .get_entity_by_account_hash(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have default account");
}

#[ignore]
#[test]
fn should_wasm_transfer_after_major_bump() {
    let (mut builder, lmdb_fixture_state, _temp_dir) =
        lmdb_fixture::builder_from_global_state_fixture(lmdb_fixture::RELEASE_1_3_1);

    let current_protocol_version = lmdb_fixture_state.genesis_protocol_version();

    let new_protocol_version =
        ProtocolVersion::from_parts(current_protocol_version.value().major + 1, 0, 0);

    let global_state_update =
        apply_global_state_update(&builder, lmdb_fixture_state.post_state_hash);

    let mut upgrade_request = {
        UpgradeRequestBuilder::new()
            .with_current_protocol_version(current_protocol_version)
            .with_new_protocol_version(new_protocol_version)
            .with_activation_point(DEFAULT_ACTIVATION_POINT)
            .with_global_state_update(global_state_update)
            .with_new_gas_hold_handling(HoldBalanceHandling::Accrued)
            .with_new_gas_hold_interval(1200u64)
            .build()
    };

    builder
        .with_block_time(Timestamp::now().into())
        .upgrade(&mut upgrade_request)
        .expect_upgrade_success();

    let wasm_transfer = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_TRANSFER_TO_ACCOUNT,
        runtime_args! {
            ARG_AMOUNT => U512::one(),
            ARG_TARGET => AccountHash::new([1; 32]),
        },
    )
    .build();

    builder.exec(wasm_transfer).expect_success().commit();

    let _default_account = builder
        .get_entity_by_account_hash(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have default account");
}

#[ignore]
#[test]
fn should_wasm_transfer_after_minor_bump() {
    let (mut builder, lmdb_fixture_state, _temp_dir) =
        lmdb_fixture::builder_from_global_state_fixture(lmdb_fixture::RELEASE_1_3_1);

    let current_protocol_version = lmdb_fixture_state.genesis_protocol_version();

    let new_protocol_version = ProtocolVersion::from_parts(
        current_protocol_version.value().major,
        current_protocol_version.value().minor + 1,
        0,
    );

    let global_state_update =
        apply_global_state_update(&builder, lmdb_fixture_state.post_state_hash);

    let mut upgrade_request = {
        UpgradeRequestBuilder::new()
            .with_current_protocol_version(current_protocol_version)
            .with_new_protocol_version(new_protocol_version)
            .with_activation_point(DEFAULT_ACTIVATION_POINT)
            .with_global_state_update(global_state_update)
            .with_new_gas_hold_handling(HoldBalanceHandling::Accrued)
            .with_new_gas_hold_interval(1200u64)
            .build()
    };

    builder
        .with_block_time(Timestamp::now().into())
        .upgrade(&mut upgrade_request)
        .expect_upgrade_success();

    let wasm_transfer = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_TRANSFER_TO_ACCOUNT,
        runtime_args! {
            ARG_AMOUNT => U512::one(),
            ARG_TARGET => AccountHash::new([1; 32]),
        },
    )
    .build();

    builder.exec(wasm_transfer).expect_success().commit();

    let _default_account = builder
        .get_entity_by_account_hash(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have default account");
}

#[ignore]
#[test]
fn should_upgrade_from_1_3_1_rel_fixture() {
    let (mut builder, lmdb_fixture_state, _temp_dir) =
        lmdb_fixture::builder_from_global_state_fixture(lmdb_fixture::RELEASE_1_3_1);

    let previous_protocol_version = lmdb_fixture_state.genesis_protocol_version();

    let new_protocol_version = ProtocolVersion::from_parts(
        previous_protocol_version.value().major,
        previous_protocol_version.value().minor + 1,
        0,
    );

    let global_state_update =
        apply_global_state_update(&builder, lmdb_fixture_state.post_state_hash);

    let mut upgrade_request = {
        UpgradeRequestBuilder::new()
            .with_current_protocol_version(previous_protocol_version)
            .with_new_protocol_version(new_protocol_version)
            .with_activation_point(DEFAULT_ACTIVATION_POINT)
            .with_global_state_update(global_state_update)
            .build()
    };

    builder
        .upgrade(&mut upgrade_request)
        .expect_upgrade_success();
}
