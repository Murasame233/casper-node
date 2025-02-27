use casper_engine_test_support::{
    ExecuteRequestBuilder, LmdbWasmTestBuilder, DEFAULT_ACCOUNT_ADDR, DEFAULT_PAYMENT,
    LOCAL_GENESIS_REQUEST,
};
use casper_types::{account::AccountHash, runtime_args};

const CONTRACT_MAIN_PURSE: &str = "main_purse.wasm";
const CONTRACT_TRANSFER_PURSE_TO_ACCOUNT: &str = "transfer_purse_to_account.wasm";
const ACCOUNT_1_ADDR: AccountHash = AccountHash::new([1u8; 32]);
const ARG_TARGET: &str = "target";
const ARG_AMOUNT: &str = "amount";

#[ignore]
#[test]
fn should_run_main_purse_contract_default_account() {
    let mut builder = LmdbWasmTestBuilder::default();

    let builder = builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    let default_account = builder
        .get_entity_by_account_hash(*DEFAULT_ACCOUNT_ADDR)
        .expect("must have contract for default account");

    let exec_request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_MAIN_PURSE,
        runtime_args! { "purse" => default_account.main_purse() },
    )
    .build();

    builder.exec(exec_request).expect_success().commit();
}

#[ignore]
#[test]
fn should_run_main_purse_contract_account_1() {
    let mut builder = LmdbWasmTestBuilder::default();

    let exec_request_1 = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_TRANSFER_PURSE_TO_ACCOUNT,
        runtime_args! { ARG_TARGET => ACCOUNT_1_ADDR, ARG_AMOUNT => *DEFAULT_PAYMENT },
    )
    .build();

    let builder = builder
        .run_genesis(LOCAL_GENESIS_REQUEST.clone())
        .exec(exec_request_1)
        .expect_success()
        .commit();

    let account_1 = builder
        .get_entity_by_account_hash(ACCOUNT_1_ADDR)
        .expect("should get account");

    let exec_request_2 = ExecuteRequestBuilder::standard(
        ACCOUNT_1_ADDR,
        CONTRACT_MAIN_PURSE,
        runtime_args! { "purse" => account_1.main_purse() },
    )
    .build();

    builder.exec(exec_request_2).expect_success().commit();
}
