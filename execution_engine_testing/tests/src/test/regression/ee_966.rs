use assert_matches::assert_matches;
use casper_wasm::builder;
use once_cell::sync::Lazy;

use casper_engine_test_support::{
    DeployItemBuilder, ExecuteRequest, ExecuteRequestBuilder, LmdbWasmTestBuilder,
    UpgradeRequestBuilder, ARG_AMOUNT, DEFAULT_ACCOUNT_ADDR, DEFAULT_PAYMENT,
    DEFAULT_PROTOCOL_VERSION, LOCAL_GENESIS_REQUEST,
};
use casper_execution_engine::{engine_state::Error, execution::ExecError};
use casper_types::{
    addressable_entity::DEFAULT_ENTRY_POINT_NAME, runtime_args, ApiError, EraId,
    HostFunctionCostsV1, HostFunctionCostsV2, MessageLimits, OpcodeCosts, ProtocolVersion,
    RuntimeArgs, WasmConfig, WasmV1Config, WasmV2Config, DEFAULT_MAX_STACK_HEIGHT,
    DEFAULT_WASM_MAX_MEMORY,
};

const CONTRACT_EE_966_REGRESSION: &str = "ee_966_regression.wasm";
const MINIMUM_INITIAL_MEMORY: u32 = 16;
const DEFAULT_ACTIVATION_POINT: EraId = EraId::new(0);

static DOUBLED_WASM_MEMORY_LIMIT: Lazy<WasmConfig> = Lazy::new(|| {
    let wasm_v1_config = WasmV1Config::new(
        DEFAULT_WASM_MAX_MEMORY * 2,
        DEFAULT_MAX_STACK_HEIGHT,
        OpcodeCosts::default(),
        HostFunctionCostsV1::default(),
    );
    let wasm_v2_config = WasmV2Config::new(
        DEFAULT_WASM_MAX_MEMORY * 2,
        OpcodeCosts::default(),
        HostFunctionCostsV2::default(),
    );
    WasmConfig::new(MessageLimits::default(), wasm_v1_config, wasm_v2_config)
});
const NEW_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::from_parts(
    DEFAULT_PROTOCOL_VERSION.value().major,
    DEFAULT_PROTOCOL_VERSION.value().minor,
    DEFAULT_PROTOCOL_VERSION.value().patch + 1,
);

fn make_session_code_with_memory_pages(initial_pages: u32, max_pages: Option<u32>) -> Vec<u8> {
    let module = builder::module()
        .function()
        // A signature with 0 params and no return type
        .signature()
        .build()
        .body()
        .build()
        .build()
        // Export above function
        .export()
        .field(DEFAULT_ENTRY_POINT_NAME)
        .build()
        // Memory section is mandatory
        .memory()
        // Produces entry `(memory (0) initial_pages [max_pages])`
        .with_min(initial_pages)
        .with_max(max_pages)
        .build()
        .build();
    casper_wasm::serialize(module).expect("should serialize")
}

fn make_request_with_session_bytes(session_code: Vec<u8>) -> ExecuteRequest {
    let deploy_item = DeployItemBuilder::new()
        .with_address(*DEFAULT_ACCOUNT_ADDR)
        .with_session_bytes(session_code, RuntimeArgs::new())
        .with_standard_payment(runtime_args! {
            ARG_AMOUNT => *DEFAULT_PAYMENT
        })
        .with_authorization_keys(&[*DEFAULT_ACCOUNT_ADDR])
        .with_deploy_hash([42; 32])
        .build();

    ExecuteRequestBuilder::from_deploy_item(&deploy_item).build()
}

#[ignore]
#[test]
fn should_run_ee_966_with_zero_min_and_zero_max_memory() {
    // A contract that has initial memory pages of 0 and maximum memory pages of 0 is valid
    let session_code = make_session_code_with_memory_pages(0, Some(0));

    let exec_request = make_request_with_session_bytes(session_code);

    let mut builder = LmdbWasmTestBuilder::default();

    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    builder.exec(exec_request).commit().expect_success();
}

#[ignore]
#[test]
fn should_run_ee_966_cant_have_too_much_initial_memory() {
    let session_code = make_session_code_with_memory_pages(DEFAULT_WASM_MAX_MEMORY + 1, None);

    let exec_request = make_request_with_session_bytes(session_code);

    let mut builder = LmdbWasmTestBuilder::default();

    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    builder.exec(exec_request).commit();

    let exec_result = &builder
        .get_exec_result_owned(0)
        .expect("should have exec response");
    let error = exec_result.error().expect("should have error");
    assert_matches!(error, Error::Exec(ExecError::Interpreter(_)));
}

#[ignore]
#[test]
fn should_run_ee_966_should_request_exactly_maximum() {
    let session_code =
        make_session_code_with_memory_pages(DEFAULT_WASM_MAX_MEMORY, Some(DEFAULT_WASM_MAX_MEMORY));

    let exec_request = make_request_with_session_bytes(session_code);

    let mut builder = LmdbWasmTestBuilder::default();

    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    builder.exec(exec_request).commit().expect_success();
}

#[ignore]
#[test]
fn should_run_ee_966_should_request_exactly_maximum_as_initial() {
    let session_code = make_session_code_with_memory_pages(DEFAULT_WASM_MAX_MEMORY, None);

    let exec_request = make_request_with_session_bytes(session_code);

    let mut builder = LmdbWasmTestBuilder::default();

    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    builder.exec(exec_request).commit().expect_success();
}

#[ignore]
#[test]
fn should_run_ee_966_cant_have_too_much_max_memory() {
    let session_code = make_session_code_with_memory_pages(
        MINIMUM_INITIAL_MEMORY,
        Some(DEFAULT_WASM_MAX_MEMORY + 1),
    );

    let exec_request = make_request_with_session_bytes(session_code);

    let mut builder = LmdbWasmTestBuilder::default();

    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    builder.exec(exec_request).commit();

    let exec_result = &builder
        .get_exec_result_owned(0)
        .expect("should have exec response");
    let error = exec_result.error().expect("should have error");
    assert_matches!(error, Error::Exec(ExecError::Interpreter(_)));
}

#[ignore]
#[test]
fn should_run_ee_966_cant_have_way_too_much_max_memory() {
    let session_code = make_session_code_with_memory_pages(
        MINIMUM_INITIAL_MEMORY,
        Some(DEFAULT_WASM_MAX_MEMORY + 42),
    );

    let exec_request = make_request_with_session_bytes(session_code);

    let mut builder = LmdbWasmTestBuilder::default();

    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    builder.exec(exec_request).commit();

    let exec_result = &builder
        .get_exec_result_owned(0)
        .expect("should have exec response");
    let error = exec_result.error().expect("should have error");
    assert_matches!(error, Error::Exec(ExecError::Interpreter(_)));
}

#[ignore]
#[test]
fn should_run_ee_966_cant_have_larger_initial_than_max_memory() {
    let session_code =
        make_session_code_with_memory_pages(DEFAULT_WASM_MAX_MEMORY, Some(MINIMUM_INITIAL_MEMORY));

    let exec_request = make_request_with_session_bytes(session_code);

    let mut builder = LmdbWasmTestBuilder::default();

    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    builder.exec(exec_request).commit();

    let exec_result = &builder
        .get_exec_result_owned(0)
        .expect("should have exec response");
    let error = exec_result.error().expect("should have error");
    assert_matches!(error, Error::Exec(ExecError::Interpreter(_)));
}

#[ignore]
#[test]
fn should_run_ee_966_regression_fail_when_growing_mem_past_max() {
    let exec_request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_EE_966_REGRESSION,
        RuntimeArgs::default(),
    )
    .build();

    let mut builder = LmdbWasmTestBuilder::default();

    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    builder.exec(exec_request).commit();

    let exec_result = &builder
        .get_exec_result_owned(0)
        .expect("should have exec response");
    let error = exec_result.error().expect("should have error");
    assert_matches!(error, Error::Exec(ExecError::Revert(ApiError::OutOfMemory)));
}

#[ignore]
#[test]
fn should_run_ee_966_regression_when_growing_mem_after_upgrade() {
    let exec_request_1 = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_EE_966_REGRESSION,
        RuntimeArgs::default(),
    )
    .build();

    let mut builder = LmdbWasmTestBuilder::default();

    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    builder.exec(exec_request_1).commit();

    //
    // This request should fail - as it's exceeding default memory limit
    //

    let exec_result = &builder
        .get_exec_result_owned(0)
        .expect("should have exec response");
    let error = exec_result.error().expect("should have error");
    assert_matches!(error, Error::Exec(ExecError::Revert(ApiError::OutOfMemory)));

    //
    // Upgrade default memory limit
    //

    let mut upgrade_request = UpgradeRequestBuilder::new()
        .with_current_protocol_version(DEFAULT_PROTOCOL_VERSION)
        .with_new_protocol_version(NEW_PROTOCOL_VERSION)
        .with_activation_point(DEFAULT_ACTIVATION_POINT)
        .build();

    let updated_chainspec = builder
        .chainspec()
        .clone()
        .with_wasm_config(*DOUBLED_WASM_MEMORY_LIMIT);

    builder
        .with_chainspec(updated_chainspec)
        .upgrade(&mut upgrade_request);

    //
    // Now this request is working as the maximum memory limit is doubled.
    //

    let exec_request_2 = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_EE_966_REGRESSION,
        RuntimeArgs::default(),
    )
    .build();

    builder.exec(exec_request_2).commit().expect_success();
}
