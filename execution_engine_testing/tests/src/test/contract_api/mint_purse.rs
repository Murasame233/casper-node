use casper_engine_test_support::{
    ExecuteRequestBuilder, LmdbWasmTestBuilder, DEFAULT_ACCOUNT_ADDR, LOCAL_GENESIS_REQUEST,
    MINIMUM_ACCOUNT_CREATION_BALANCE, SYSTEM_ADDR,
};
use casper_types::{runtime_args, RuntimeArgs, U512};

const CONTRACT_MINT_PURSE: &str = "mint_purse.wasm";
const CONTRACT_TRANSFER_TO_ACCOUNT: &str = "transfer_to_account_u512.wasm";
const TRANSFER_AMOUNT: u64 = MINIMUM_ACCOUNT_CREATION_BALANCE;

#[ignore]
#[test]
fn should_run_mint_purse_contract() {
    let exec_request_1 = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_TRANSFER_TO_ACCOUNT,
        runtime_args! { "target" => *SYSTEM_ADDR, "amount" => U512::from(TRANSFER_AMOUNT) },
    )
    .build();
    let exec_request_2 =
        ExecuteRequestBuilder::standard(*SYSTEM_ADDR, CONTRACT_MINT_PURSE, RuntimeArgs::default())
            .build();

    let mut builder = LmdbWasmTestBuilder::default();

    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    builder.exec(exec_request_1).commit().expect_success();
    builder.exec(exec_request_2).commit().expect_success();
}

#[ignore]
#[test]
fn should_not_allow_non_system_accounts_to_mint() {
    let exec_request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_MINT_PURSE,
        RuntimeArgs::default(),
    )
    .build();

    assert!(LmdbWasmTestBuilder::default()
        .run_genesis(LOCAL_GENESIS_REQUEST.clone())
        .exec(exec_request)
        .commit()
        .is_error());
}
