use casper_engine_test_support::{
    ExecuteRequestBuilder, LmdbWasmTestBuilder, DEFAULT_ACCOUNT_ADDR, DEFAULT_PAYMENT,
    LOCAL_GENESIS_REQUEST,
};
use casper_types::{account::AccountHash, runtime_args, Key, U512};

const CONTRACT_CREATE: &str = "ee_572_regression_create.wasm";
const CONTRACT_ESCALATE: &str = "ee_572_regression_escalate.wasm";
const CONTRACT_TRANSFER: &str = "transfer_purse_to_account.wasm";
const CREATE: &str = "create";

const ACCOUNT_1_ADDR: AccountHash = AccountHash::new([1u8; 32]);
const ACCOUNT_2_ADDR: AccountHash = AccountHash::new([2u8; 32]);

#[ignore]
#[test]
fn should_run_ee_572_regression() {
    let account_amount: U512 = *DEFAULT_PAYMENT + U512::from(100);
    let account_1_creation_args = runtime_args! {
        "target" => ACCOUNT_1_ADDR,
        "amount" => account_amount
    };
    let account_2_creation_args = runtime_args! {
        "target" => ACCOUNT_2_ADDR,
        "amount" => account_amount,
    };

    // This test runs a contract that's after every call extends the same key with
    // more data
    let mut builder = LmdbWasmTestBuilder::default();

    let exec_request_1 = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_TRANSFER,
        account_1_creation_args,
    )
    .build();
    let exec_request_2 = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_TRANSFER,
        account_2_creation_args.clone(),
    )
    .build();

    let exec_request_3 =
        ExecuteRequestBuilder::standard(ACCOUNT_1_ADDR, CONTRACT_CREATE, account_2_creation_args)
            .build();

    // Create Accounts
    builder
        .run_genesis(LOCAL_GENESIS_REQUEST.clone())
        .exec(exec_request_1)
        .expect_success()
        .commit();

    builder.exec(exec_request_2).expect_success().commit();

    // Store the creation contract
    builder.exec(exec_request_3).expect_success().commit();

    let contract: Key = {
        let account = builder
            .get_entity_with_named_keys_by_account_hash(ACCOUNT_1_ADDR)
            .expect("must have default contract package");
        *account
            .named_keys()
            .get(CREATE)
            .expect("Could not find contract pointer")
    };

    // Attempt to forge a new URef with escalated privileges
    let exec_request_4 = ExecuteRequestBuilder::standard(
        ACCOUNT_2_ADDR,
        CONTRACT_ESCALATE,
        runtime_args! {
            "contract_hash" => contract.into_entity_hash_addr().expect("should be hash"),
        },
    )
    .build();

    // Attempt to forge a new URef with escalated privileges
    let _ = builder
        .exec(exec_request_4)
        .get_exec_result_owned(3)
        .expect("should have a response");

    let error_message = builder.get_error_message().unwrap();

    assert!(
        error_message.contains("Forged reference"),
        "{}",
        error_message
    );
}
