use casper_engine_test_support::{
    ExecuteRequestBuilder, LmdbWasmTestBuilder, DEFAULT_ACCOUNT_ADDR, DEFAULT_ACCOUNT_PUBLIC_KEY,
    DEFAULT_GENESIS_TIMESTAMP_MILLIS, LOCAL_GENESIS_REQUEST,
};
use casper_types::{
    runtime_args,
    system::auction::{
        BidAddr, BidKind, BidsExt, DelegatorKind, EraInfo, ValidatorBid, ARG_AMOUNT, ARG_VALIDATOR,
    },
    Key, PublicKey, StoredValue, U512,
};

const STORED_STAKING_CONTRACT_NAME: &str = "staking_stored.wasm";

mod bids;
mod distribute;
mod reservations;

fn get_validator_bid(
    builder: &mut LmdbWasmTestBuilder,
    validator_public_key: PublicKey,
) -> Option<ValidatorBid> {
    let bids = builder.get_bids();
    bids.validator_bid(&validator_public_key)
}

pub fn get_delegator_staked_amount(
    builder: &mut LmdbWasmTestBuilder,
    validator_public_key: PublicKey,
    delegator_public_key: PublicKey,
) -> U512 {
    let bids = builder.get_bids();

    let delegator = bids
        .delegator_by_kind(&validator_public_key, &DelegatorKind::PublicKey(delegator_public_key.clone()))
        .expect("bid should exist for validator-{validator_public_key}, delegator-{delegator_public_key}");

    delegator.staked_amount()
}

pub fn get_era_info(builder: &mut LmdbWasmTestBuilder) -> EraInfo {
    let era_info_value = builder
        .query(None, Key::EraSummary, &[])
        .expect("should have value");

    era_info_value
        .as_era_info()
        .cloned()
        .expect("should be era info")
}

#[ignore]
#[test]
fn should_support_contract_staking() {
    let timestamp_millis = DEFAULT_GENESIS_TIMESTAMP_MILLIS;
    let purse_name = "staking_purse".to_string();
    let contract_name = "staking".to_string();
    let stake = "stake".to_string();
    let unstake = "unstake".to_string();
    let account = *DEFAULT_ACCOUNT_ADDR;
    let seed_amount = U512::from(10_000_000_000_000_000_u64);
    let delegate_amount = U512::from(5_000_000_000_000_000_u64);

    let mut builder = LmdbWasmTestBuilder::default();
    builder.run_genesis(LOCAL_GENESIS_REQUEST.clone());

    let account_main_purse = builder
        .get_entity_with_named_keys_by_account_hash(account)
        .expect("should have account")
        .main_purse();
    let starting_account_balance = builder.get_purse_balance(account_main_purse);

    builder
        .exec(
            ExecuteRequestBuilder::standard(
                account,
                STORED_STAKING_CONTRACT_NAME,
                runtime_args! {ARG_AMOUNT => seed_amount},
            )
            .build(),
        )
        .commit()
        .expect_success();

    let default_account = builder
        .get_entity_with_named_keys_by_account_hash(account)
        .expect("should have account");
    let named_keys = default_account.named_keys();
    let contract_purse = named_keys
        .get(&purse_name)
        .expect("purse_name key should exist")
        .into_uref()
        .expect("should be a uref");

    let post_install_account_balance = builder.get_purse_balance(account_main_purse);
    assert_eq!(
        post_install_account_balance,
        starting_account_balance.saturating_sub(seed_amount),
        "post install should be reduced due to seeding contract purse"
    );

    let pre_delegation_balance = builder.get_purse_balance(contract_purse);
    assert_eq!(pre_delegation_balance, seed_amount);

    let validator_pk = &*DEFAULT_ACCOUNT_PUBLIC_KEY;
    // stake from contract
    builder
        .exec(
            ExecuteRequestBuilder::contract_call_by_name(
                account,
                &contract_name,
                &stake,
                runtime_args! {
                    ARG_AMOUNT => delegate_amount,
                    ARG_VALIDATOR => validator_pk.clone(),
                },
            )
            .build(),
        )
        .commit()
        .expect_success();

    let post_delegation_balance = builder.get_purse_balance(contract_purse);
    assert_eq!(
        post_delegation_balance,
        pre_delegation_balance.saturating_sub(delegate_amount),
        "contract purse balance should be reduced by staked amount"
    );

    let delegation_key = Key::BidAddr(BidAddr::DelegatedPurse {
        validator: validator_pk.to_account_hash(),
        delegator: contract_purse.addr(),
    });

    if let StoredValue::BidKind(BidKind::Delegator(delegator)) = builder
        .query(None, delegation_key, &[])
        .expect("should have delegation bid")
    {
        assert_eq!(
            delegator.staked_amount(),
            delegate_amount,
            "staked amount should match delegation amount"
        );
    }

    builder.run_auction(timestamp_millis, vec![]);

    let increased_delegate_amount = if let StoredValue::BidKind(BidKind::Delegator(delegator)) =
        builder
            .query(None, delegation_key, &[])
            .expect("should have delegation bid")
    {
        assert_ne!(
            delegator.staked_amount(),
            delegate_amount,
            "staked amount should execeed delegation amount due to rewards"
        );
        delegator.staked_amount()
    } else {
        U512::zero()
    };

    // unstake from contract
    builder
        .exec(
            ExecuteRequestBuilder::contract_call_by_name(
                account,
                &contract_name,
                &unstake,
                runtime_args! {
                    ARG_AMOUNT => increased_delegate_amount,
                    ARG_VALIDATOR => validator_pk.clone(),
                },
            )
            .build(),
        )
        .commit()
        .expect_success();

    assert!(
        builder.query(None, delegation_key, &[]).is_err(),
        "delegation record should be removed"
    );

    let unbond_key = Key::BidAddr(BidAddr::UnbondPurse {
        validator: validator_pk.to_account_hash(),
        unbonder: contract_purse.addr(),
    });

    assert_eq!(
        post_delegation_balance,
        builder.get_purse_balance(contract_purse),
        "at this point, unstaked token has not been returned"
    );

    let unbonded_amount = if let StoredValue::BidKind(BidKind::Unbond(unbond)) = builder
        .query(None, unbond_key, &[])
        .expect("should have unbond")
    {
        let unbond_era = unbond.eras().first().expect("should have an era entry");
        assert_eq!(
            *unbond_era.amount(),
            increased_delegate_amount,
            "unbonded amount should match expectations"
        );
        *unbond_era.amount()
    } else {
        U512::zero()
    };

    for _ in 0..=builder.get_auction_delay() {
        // crank era
        builder.run_auction(timestamp_millis, vec![]);
    }

    assert_eq!(
        delegate_amount.saturating_add(unbonded_amount),
        builder.get_purse_balance(contract_purse),
        "unbonded amount should be available to contract staking purse"
    );
}
