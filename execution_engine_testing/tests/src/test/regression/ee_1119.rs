use num_traits::Zero;
use once_cell::sync::Lazy;

use casper_engine_test_support::{
    utils, ExecuteRequestBuilder, LmdbWasmTestBuilder, DEFAULT_ACCOUNTS, DEFAULT_ACCOUNT_ADDR,
    DEFAULT_ACCOUNT_INITIAL_BALANCE, DEFAULT_ACCOUNT_PUBLIC_KEY, DEFAULT_GENESIS_TIMESTAMP_MILLIS,
    DEFAULT_LOCKED_FUNDS_PERIOD_MILLIS, MINIMUM_ACCOUNT_CREATION_BALANCE, SYSTEM_ADDR,
};
use casper_execution_engine::engine_state::engine_config::DEFAULT_MINIMUM_DELEGATION_AMOUNT;
use casper_types::{
    account::AccountHash,
    runtime_args,
    system::{
        auction::{
            BidsExt, DelegationRate, UnbondKind, ARG_DELEGATOR, ARG_VALIDATOR,
            ARG_VALIDATOR_PUBLIC_KEYS, METHOD_SLASH,
        },
        mint::TOTAL_SUPPLY_KEY,
    },
    EntityAddr, GenesisAccount, GenesisValidator, Motes, PublicKey, SecretKey, U512,
};

const CONTRACT_TRANSFER_TO_ACCOUNT: &str = "transfer_to_account_u512.wasm";
const CONTRACT_WITHDRAW_BID: &str = "withdraw_bid.wasm";
const CONTRACT_DELEGATE: &str = "delegate.wasm";
const CONTRACT_UNDELEGATE: &str = "undelegate.wasm";

const DELEGATE_AMOUNT_1: u64 = 95_000 + DEFAULT_MINIMUM_DELEGATION_AMOUNT;
const UNDELEGATE_AMOUNT_1: u64 = 17_000;

const TRANSFER_AMOUNT: u64 = MINIMUM_ACCOUNT_CREATION_BALANCE;

const ARG_AMOUNT: &str = "amount";
const ARG_PUBLIC_KEY: &str = "public_key";

static VALIDATOR_1: Lazy<PublicKey> = Lazy::new(|| {
    let secret_key = SecretKey::ed25519_from_bytes([3; SecretKey::ED25519_LENGTH]).unwrap();
    PublicKey::from(&secret_key)
});
static VALIDATOR_1_ADDR: Lazy<AccountHash> = Lazy::new(|| AccountHash::from(&*VALIDATOR_1));
const VALIDATOR_1_STAKE: u64 = 250_000;

const VESTING_WEEKS: u64 = 14;

#[ignore]
#[test]
fn should_slash_validator_and_their_delegators() {
    let accounts = {
        let validator_1 = GenesisAccount::account(
            VALIDATOR_1.clone(),
            Motes::new(DEFAULT_ACCOUNT_INITIAL_BALANCE),
            Some(GenesisValidator::new(
                Motes::new(VALIDATOR_1_STAKE),
                DelegationRate::zero(),
            )),
        );

        let mut tmp: Vec<GenesisAccount> = DEFAULT_ACCOUNTS.clone();
        tmp.push(validator_1);
        tmp
    };
    let run_genesis_request = utils::create_run_genesis_request(accounts);

    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(run_genesis_request);

    let fund_system_exec_request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_TRANSFER_TO_ACCOUNT,
        runtime_args! {
            "target" => *SYSTEM_ADDR,
            "amount" => U512::from(TRANSFER_AMOUNT)
        },
    )
    .build();

    builder
        .exec(fund_system_exec_request)
        .expect_success()
        .commit();

    let auction = builder.get_auction_contract_hash();

    //
    // Validator delegates funds on other genesis validator
    //

    let delegate_exec_request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_DELEGATE,
        runtime_args! {
            ARG_AMOUNT => U512::from(DELEGATE_AMOUNT_1),
            ARG_VALIDATOR => VALIDATOR_1.clone(),
            ARG_DELEGATOR => DEFAULT_ACCOUNT_PUBLIC_KEY.clone(),
        },
    )
    .build();

    builder
        .exec(delegate_exec_request)
        .expect_success()
        .commit();

    let bids = builder.get_bids();
    let validator_1_bid = bids.validator_bid(&VALIDATOR_1).expect("should have bid");
    let bid_purse = validator_1_bid.bonding_purse();
    assert_eq!(
        builder.get_purse_balance(*bid_purse),
        U512::from(VALIDATOR_1_STAKE),
    );

    let unbond_purses = builder.get_unbonds();
    assert_eq!(unbond_purses.len(), 0);

    //
    // Unlock funds of genesis validators
    //
    builder.run_auction(
        DEFAULT_GENESIS_TIMESTAMP_MILLIS + DEFAULT_LOCKED_FUNDS_PERIOD_MILLIS,
        Vec::new(),
    );

    //
    // Partial unbond through undelegate on other genesis validator
    //

    let unbond_amount = U512::from(VALIDATOR_1_STAKE / VESTING_WEEKS);

    let undelegate_exec_request = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        CONTRACT_UNDELEGATE,
        runtime_args! {
            ARG_AMOUNT => U512::from(UNDELEGATE_AMOUNT_1),
            ARG_VALIDATOR => VALIDATOR_1.clone(),
            ARG_DELEGATOR => DEFAULT_ACCOUNT_PUBLIC_KEY.clone(),
        },
    )
    .build();
    builder
        .exec(undelegate_exec_request)
        .commit()
        .expect_success();

    //
    // Other genesis validator withdraws withdraws his bid
    //

    let withdraw_bid_request = ExecuteRequestBuilder::standard(
        *VALIDATOR_1_ADDR,
        CONTRACT_WITHDRAW_BID,
        runtime_args! {
            ARG_AMOUNT => unbond_amount,
            ARG_PUBLIC_KEY => VALIDATOR_1.clone(),
        },
    )
    .build();

    builder.exec(withdraw_bid_request).expect_success().commit();

    let unbond_purses = builder.get_unbonds();
    assert_eq!(unbond_purses.len(), 2);

    let unbond_kind = UnbondKind::Validator(VALIDATOR_1.clone());

    let unbonds = unbond_purses
        .get(&unbond_kind)
        .cloned()
        .expect("should have unbond");
    assert_eq!(unbonds.len(), 1);
    let unbond = unbonds.first().expect("must get unbond");
    assert_eq!(unbond.eras().len(), 1);
    assert_eq!(unbond.validator_public_key(), &*VALIDATOR_1,);
    assert_eq!(
        unbond.unbond_kind(),
        &UnbondKind::Validator(VALIDATOR_1.clone()),
    );
    assert!(unbond.is_validator());
    let era = unbond.eras().first().expect("should have eras");
    assert_eq!(era.amount(), &unbond_amount);

    assert!(
        unbond_purses.contains_key(&unbond_kind),
        "should be part of unbonds"
    );

    let slash_request_1 = ExecuteRequestBuilder::contract_call_by_hash(
        *SYSTEM_ADDR,
        auction,
        METHOD_SLASH,
        runtime_args! {
            ARG_VALIDATOR_PUBLIC_KEYS => vec![DEFAULT_ACCOUNT_PUBLIC_KEY.clone()]
        },
    )
    .build();

    builder.exec(slash_request_1).expect_success().commit();

    let unbond_purses_noop = builder.get_unbonds();
    assert_eq!(
        unbond_purses, unbond_purses_noop,
        "slashing default validator should be noop because no unbonding was done"
    );

    let bids = builder.get_bids();
    assert!(!bids.is_empty());
    bids.validator_bid(&VALIDATOR_1).expect("bids should exist");

    //
    // Slash - only `withdraw_bid` amount is slashed
    //
    let total_supply_before_slashing: U512 = builder.get_value(
        EntityAddr::System(builder.get_mint_contract_hash().value()),
        TOTAL_SUPPLY_KEY,
    );

    let slash_request_2 = ExecuteRequestBuilder::contract_call_by_hash(
        *SYSTEM_ADDR,
        auction,
        METHOD_SLASH,
        runtime_args! {
            ARG_VALIDATOR_PUBLIC_KEYS => vec![VALIDATOR_1.clone()]
        },
    )
    .build();

    builder.exec(slash_request_2).expect_success().commit();

    let unbond_purses = builder.get_unbonds();
    assert_eq!(unbond_purses.len(), 0);

    let bids = builder.get_bids();
    assert!(bids.validator_bid(&VALIDATOR_1).is_none());

    let total_supply_after_slashing: U512 = builder.get_value(
        EntityAddr::System(builder.get_mint_contract_hash().value()),
        TOTAL_SUPPLY_KEY,
    );

    assert_eq!(
        total_supply_after_slashing + VALIDATOR_1_STAKE + DELEGATE_AMOUNT_1,
        total_supply_before_slashing,
    );
}
