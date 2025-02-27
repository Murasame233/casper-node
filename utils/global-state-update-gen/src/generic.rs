pub(crate) mod config;
mod state_reader;
mod state_tracker;
#[cfg(test)]
mod testing;
mod update;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

use clap::ArgMatches;
use itertools::Itertools;

use casper_engine_test_support::LmdbWasmTestBuilder;
use casper_execution_engine::engine_state::engine_config::DEFAULT_PROTOCOL_VERSION;

use casper_types::{
    system::auction::{
        Bid, BidKind, BidsExt, DelegatorBid, DelegatorKind, Reservation, SeigniorageRecipientV2,
        SeigniorageRecipientsSnapshotV2, Unbond, ValidatorBid, ValidatorCredit,
    },
    CLValue, EraId, PublicKey, StoredValue, U512,
};

use crate::utils::{hash_from_str, validators_diff, ValidatorInfo, ValidatorsDiff};

use self::{
    config::{AccountConfig, Config, Transfer},
    state_reader::StateReader,
    state_tracker::StateTracker,
    update::Update,
};

pub(crate) fn generate_generic_update(matches: &ArgMatches<'_>) {
    let data_dir = matches.value_of("data_dir").unwrap_or(".");
    let state_hash = hash_from_str(matches.value_of("hash").unwrap());
    let config_path = matches.value_of("config_file").unwrap();

    let config_bytes = fs::read(config_path).expect("couldn't read the config file");
    let config: Config = toml::from_slice(&config_bytes).expect("couldn't parse the config file");

    let builder = LmdbWasmTestBuilder::open_raw(
        data_dir,
        Default::default(),
        DEFAULT_PROTOCOL_VERSION,
        state_hash,
    );

    update_from_config(builder, config);
}

fn get_update<T: StateReader>(reader: T, config: Config) -> Update {
    let protocol_version = config.protocol_version;
    let mut state_tracker = StateTracker::new(reader, protocol_version);

    process_transfers(&mut state_tracker, &config.transfers);

    update_account_balances(&mut state_tracker, &config.accounts);

    let validators = update_auction_state(
        &mut state_tracker,
        &config.accounts,
        config.only_listed_validators,
        config.slash_instead_of_unbonding,
    );

    let entries = state_tracker.get_entries();

    Update::new(entries, validators)
}

pub(crate) fn update_from_config<T: StateReader>(reader: T, config: Config) {
    let update = get_update(reader, config);
    update.print();
}

fn process_transfers<T: StateReader>(state: &mut StateTracker<T>, transfers: &[Transfer]) {
    for transfer in transfers {
        state.execute_transfer(transfer);
    }
}

fn update_account_balances<T: StateReader>(
    state: &mut StateTracker<T>,
    accounts: &[AccountConfig],
) {
    for account in accounts {
        if let Some(target_balance) = account.balance {
            let account_hash = account.public_key.to_account_hash();
            if let Some(account) = state.get_account(&account_hash) {
                state.set_purse_balance(account.main_purse(), target_balance);
            } else {
                state.create_addressable_entity_for_account(account_hash, target_balance);
            }
        }
    }
}

/// Returns the complete set of validators immediately after the upgrade,
/// if the validator set changed.
fn update_auction_state<T: StateReader>(
    state: &mut StateTracker<T>,
    accounts: &[AccountConfig],
    only_listed_validators: bool,
    slash_instead_of_unbonding: bool,
) -> Option<Vec<ValidatorInfo>> {
    // Read the old SeigniorageRecipientsSnapshot
    let (snapshot_key, old_snapshot) = state.read_snapshot();

    // Create a new snapshot based on the old one and the supplied validators.
    let new_snapshot = if only_listed_validators {
        gen_snapshot_only_listed(
            *old_snapshot.keys().next().unwrap(),
            old_snapshot.len() as u64,
            accounts,
        )
    } else {
        gen_snapshot_from_old(old_snapshot.clone(), accounts)
    };

    if new_snapshot == old_snapshot {
        return None;
    }

    // Save the write to the snapshot key.
    state.write_entry(
        snapshot_key,
        StoredValue::from(CLValue::from_t(new_snapshot.clone()).unwrap()),
    );

    let validators_diff = validators_diff(&old_snapshot, &new_snapshot);

    let bids = state.get_bids();
    if slash_instead_of_unbonding {
        // zero the unbonds for the removed validators independently of set_bid; set_bid will take
        // care of zeroing the delegators if necessary
        for bid_kind in bids {
            if validators_diff
                .removed
                .contains(&bid_kind.validator_public_key())
            {
                if let Some(bonding_purse) = bid_kind.bonding_purse() {
                    state.remove_withdraws_and_unbonds_with_bonding_purse(&bonding_purse);
                }
            }
        }
    }

    add_and_remove_bids(
        state,
        &validators_diff,
        &new_snapshot,
        only_listed_validators,
        slash_instead_of_unbonding,
    );

    // We need to output the validators for the next era, which are contained in the first entry
    // in the snapshot.
    Some(
        new_snapshot
            .values()
            .next()
            .expect("snapshot should have at least one entry")
            .iter()
            .map(|(public_key, seigniorage_recipient)| ValidatorInfo {
                public_key: public_key.clone(),
                weight: seigniorage_recipient
                    .total_stake()
                    .expect("total validator stake too large"),
            })
            .collect(),
    )
}

/// Generates a new `SeigniorageRecipientsSnapshotV2` based on:
/// - The starting era ID (the era ID at which the snapshot should start).
/// - Count - the number of eras to be included in the snapshot.
/// - The list of configured accounts.
fn gen_snapshot_only_listed(
    starting_era_id: EraId,
    count: u64,
    accounts: &[AccountConfig],
) -> SeigniorageRecipientsSnapshotV2 {
    let mut new_snapshot = BTreeMap::new();
    let mut era_validators = BTreeMap::new();
    for account in accounts {
        // don't add validators with zero stake to the snapshot
        let validator_cfg = match &account.validator {
            Some(validator) if validator.bonded_amount != U512::zero() => validator,
            _ => continue,
        };
        let seigniorage_recipient = SeigniorageRecipientV2::new(
            validator_cfg.bonded_amount,
            validator_cfg.delegation_rate.unwrap_or_default(),
            validator_cfg.delegators_map().unwrap_or_default(),
            validator_cfg.reservations_map().unwrap_or_default(),
        );
        let _ = era_validators.insert(account.public_key.clone(), seigniorage_recipient);
    }
    for era_id in starting_era_id.iter(count) {
        let _ = new_snapshot.insert(era_id, era_validators.clone());
    }
    new_snapshot
}

/// Generates a new `SeigniorageRecipientsSnapshotV2` by modifying the stakes listed in the old
/// snapshot according to the supplied list of configured accounts.
fn gen_snapshot_from_old(
    mut snapshot: SeigniorageRecipientsSnapshotV2,
    accounts: &[AccountConfig],
) -> SeigniorageRecipientsSnapshotV2 {
    // Read the modifications to be applied to the validators set from the config.
    let validators_map: BTreeMap<_, _> = accounts
        .iter()
        .filter_map(|acc| {
            acc.validator
                .as_ref()
                .map(|validator| (acc.public_key.clone(), validator.clone()))
        })
        .collect();

    // We will be modifying the entries in the old snapshot passed in as `snapshot` according to
    // the config.
    for recipients in snapshot.values_mut() {
        // We use `retain` to drop some entries and modify some of the ones that will be retained.
        recipients.retain(
            |public_key, recipient| match validators_map.get(public_key) {
                // If the validator's stake is configured to be zero, we drop them from the
                // snapshot.
                Some(validator) if validator.bonded_amount.is_zero() => false,
                // Otherwise, we keep them, but modify the properties.
                Some(validator) => {
                    *recipient = SeigniorageRecipientV2::new(
                        validator.bonded_amount,
                        validator
                            .delegation_rate
                            // If the delegation rate wasn't specified in the config, keep the one
                            // from the old snapshot.
                            .unwrap_or(*recipient.delegation_rate()),
                        validator
                            .delegators_map()
                            // If the delegators weren't specified in the config, keep the ones
                            // from the old snapshot.
                            .unwrap_or_else(|| recipient.delegator_stake().clone()),
                        validator
                            .reservations_map()
                            // If the delegators weren't specified in the config, keep the ones
                            // from the old snapshot.
                            .unwrap_or_else(|| recipient.reservation_delegation_rates().clone()),
                    );
                    true
                }
                // Validators not present in the config will be kept unmodified.
                None => true,
            },
        );

        // Add the validators that weren't present in the old snapshot.
        for (public_key, validator) in &validators_map {
            if recipients.contains_key(public_key) {
                continue;
            }

            if validator.bonded_amount != U512::zero() {
                recipients.insert(
                    public_key.clone(),
                    SeigniorageRecipientV2::new(
                        validator.bonded_amount,
                        // Unspecified delegation rate will be treated as 0.
                        validator.delegation_rate.unwrap_or_default(),
                        // Unspecified delegators will be treated as an empty list.
                        validator.delegators_map().unwrap_or_default(),
                        // Unspecified reservation delegation rates will be treated as an empty
                        // list.
                        validator.reservations_map().unwrap_or_default(),
                    ),
                );
            }
        }
    }

    // Return the modified snapshot.
    snapshot
}

/// Generates a set of writes necessary to "fix" the bids, ie.:
/// - set the bids of the new validators to their desired stakes,
/// - remove the bids of the old validators that are no longer validators,
/// - if `only_listed_validators` is true, remove all the bids that are larger than the smallest bid
///   among the new validators (necessary, because such bidders would outbid the validators decided
///   by the social consensus).
pub fn add_and_remove_bids<T: StateReader>(
    state: &mut StateTracker<T>,
    validators_diff: &ValidatorsDiff,
    new_snapshot: &SeigniorageRecipientsSnapshotV2,
    only_listed_validators: bool,
    slash_instead_of_unbonding: bool,
) {
    let to_unbid = if only_listed_validators {
        let large_bids = find_large_bids(state, new_snapshot);
        validators_diff
            .removed
            .union(&large_bids)
            .cloned()
            .collect()
    } else {
        validators_diff.removed.clone()
    };

    for (pub_key, seigniorage_recipient) in new_snapshot.values().next_back().unwrap() {
        create_or_update_bid(
            state,
            pub_key,
            seigniorage_recipient,
            slash_instead_of_unbonding,
        );
    }

    // Refresh the bids - we modified them above.
    let bids = state.get_bids();
    for public_key in to_unbid {
        for bid_kind in bids
            .iter()
            .filter(|x| x.validator_public_key() == public_key)
        {
            let reset_bid = match bid_kind {
                BidKind::Unified(bid) => BidKind::Unified(Box::new(Bid::empty(
                    public_key.clone(),
                    *bid.bonding_purse(),
                ))),
                BidKind::Validator(validator_bid) => {
                    let mut new_bid =
                        ValidatorBid::empty(public_key.clone(), *validator_bid.bonding_purse());
                    new_bid.set_delegation_amount_boundaries(
                        validator_bid.minimum_delegation_amount(),
                        validator_bid.maximum_delegation_amount(),
                    );
                    BidKind::Validator(Box::new(new_bid))
                }
                BidKind::Delegator(delegator_bid) => {
                    BidKind::Delegator(Box::new(DelegatorBid::empty(
                        public_key.clone(),
                        delegator_bid.delegator_kind().clone(),
                        *delegator_bid.bonding_purse(),
                    )))
                }
                // there should be no need to modify bridge records
                // since they don't influence the bidding process
                BidKind::Bridge(_) => continue,
                BidKind::Credit(credit) => BidKind::Credit(Box::new(ValidatorCredit::empty(
                    public_key.clone(),
                    credit.era_id(),
                ))),
                BidKind::Reservation(reservation_bid) => {
                    BidKind::Reservation(Box::new(Reservation::new(
                        public_key.clone(),
                        reservation_bid.delegator_kind().clone(),
                        *reservation_bid.delegation_rate(),
                    )))
                }
                BidKind::Unbond(unbond) => BidKind::Unbond(Box::new(Unbond::new(
                    unbond.validator_public_key().clone(),
                    unbond.unbond_kind().clone(),
                    unbond.eras().clone(),
                ))),
            };
            state.set_bid(reset_bid, slash_instead_of_unbonding);
        }
    }
}

/// Returns the set of public keys that have bids larger than the smallest bid among the new
/// validators.
fn find_large_bids<T: StateReader>(
    state: &mut StateTracker<T>,
    snapshot: &SeigniorageRecipientsSnapshotV2,
) -> BTreeSet<PublicKey> {
    let seigniorage_recipients = snapshot.values().next().unwrap();
    let min_bid = seigniorage_recipients
        .values()
        .map(|recipient| {
            recipient
                .total_stake()
                .expect("should have valid total stake")
        })
        .min()
        .unwrap();
    let new_validators: BTreeSet<_> = seigniorage_recipients.keys().collect();

    let mut ret = BTreeSet::new();

    let validator_bids = state
        .get_bids()
        .iter()
        .filter(|x| x.is_validator() || x.is_delegator())
        .cloned()
        .collect_vec();

    for bid_kind in validator_bids {
        if let BidKind::Unified(bid) = bid_kind {
            if bid.total_staked_amount().unwrap_or_default() > min_bid
                && !new_validators.contains(bid.validator_public_key())
            {
                ret.insert(bid.validator_public_key().clone());
            }
        } else if let BidKind::Validator(validator_bid) = bid_kind {
            if new_validators.contains(validator_bid.validator_public_key()) {
                // The validator is still going to be a validator - we don't remove their bid.
                continue;
            }
            if validator_bid.staked_amount() > min_bid {
                ret.insert(validator_bid.validator_public_key().clone());
                continue;
            }
            let delegator_stake = state
                .get_bids()
                .iter()
                .filter(|x| {
                    x.validator_public_key() == *validator_bid.validator_public_key()
                        && x.is_delegator()
                })
                .map(|x| x.staked_amount().unwrap())
                .sum();

            let total = validator_bid
                .staked_amount()
                .checked_add(delegator_stake)
                .unwrap_or_default();
            if total > min_bid {
                ret.insert(validator_bid.validator_public_key().clone());
            }
        }
    }
    ret
}

/// Updates the amount of an existing bid for the given public key, or creates a new one.
fn create_or_update_bid<T: StateReader>(
    state: &mut StateTracker<T>,
    validator_public_key: &PublicKey,
    updated_recipient: &SeigniorageRecipientV2,
    slash_instead_of_unbonding: bool,
) {
    let existing_bids = state.get_bids();

    let maybe_existing_recipient = existing_bids
        .iter()
        .find(|x| {
            (x.is_unified() || x.is_validator())
                && &x.validator_public_key() == validator_public_key
        })
        .map(|existing_bid| {
            let reservation_delegation_rates =
                match existing_bids.reservations_by_validator_public_key(validator_public_key) {
                    None => BTreeMap::new(),
                    Some(reservations) => reservations
                        .iter()
                        .map(|reservation| {
                            (
                                reservation.delegator_kind().clone(),
                                *reservation.delegation_rate(),
                            )
                        })
                        .collect(),
                };

            match existing_bid {
                BidKind::Unified(bid) => {
                    let delegator_stake = bid
                        .delegators()
                        .iter()
                        .map(|(k, d)| (DelegatorKind::PublicKey(k.clone()), d.staked_amount()))
                        .collect();

                    (
                        bid.bonding_purse(),
                        SeigniorageRecipientV2::new(
                            *bid.staked_amount(),
                            *bid.delegation_rate(),
                            delegator_stake,
                            reservation_delegation_rates,
                        ),
                        0,
                        u64::MAX,
                        0,
                    )
                }
                BidKind::Validator(validator_bid) => {
                    let delegator_stake = match existing_bids
                        .delegators_by_validator_public_key(validator_public_key)
                    {
                        None => BTreeMap::new(),
                        Some(delegators) => delegators
                            .iter()
                            .map(|d| (d.delegator_kind().clone(), d.staked_amount()))
                            .collect(),
                    };

                    (
                        validator_bid.bonding_purse(),
                        SeigniorageRecipientV2::new(
                            validator_bid.staked_amount(),
                            *validator_bid.delegation_rate(),
                            delegator_stake,
                            reservation_delegation_rates,
                        ),
                        validator_bid.minimum_delegation_amount(),
                        validator_bid.maximum_delegation_amount(),
                        validator_bid.reserved_slots(),
                    )
                }
                _ => unreachable!(),
            }
        });

    // existing bid
    if let Some((
        bonding_purse,
        existing_recipient,
        min_delegation_amount,
        max_delegation_amount,
        reserved_slots,
    )) = maybe_existing_recipient
    {
        if existing_recipient == *updated_recipient {
            return; // noop
        }

        let delegators = existing_bids
            .delegators_by_validator_public_key(validator_public_key)
            .unwrap_or_default();

        for delegator in delegators {
            let delegator_bid = match updated_recipient
                .delegator_stake()
                .get(delegator.delegator_kind())
            {
                None => {
                    // todo!() this is a remove; the global state update tool does not
                    // yet support prune so in the meantime, setting the amount
                    // to 0.
                    DelegatorBid::empty(
                        delegator.validator_public_key().clone(),
                        delegator.delegator_kind().clone(),
                        *delegator.bonding_purse(),
                    )
                }
                Some(updated_delegator_stake) => DelegatorBid::unlocked(
                    delegator.delegator_kind().clone(),
                    *updated_delegator_stake,
                    *delegator.bonding_purse(),
                    validator_public_key.clone(),
                ),
            };
            if delegator.staked_amount() == delegator_bid.staked_amount() {
                continue; // effectively noop
            }
            state.set_bid(
                BidKind::Delegator(Box::new(delegator_bid)),
                slash_instead_of_unbonding,
            );
        }

        for (delegator_pub_key, delegator_stake) in updated_recipient.delegator_stake() {
            if existing_recipient
                .delegator_stake()
                .contains_key(delegator_pub_key)
            {
                // we handled this scenario above
                continue;
            }
            // this is a entirely new delegator
            let delegator_bonding_purse = state.create_purse(*delegator_stake);
            let delegator_bid = DelegatorBid::unlocked(
                delegator_pub_key.clone(),
                *delegator_stake,
                delegator_bonding_purse,
                validator_public_key.clone(),
            );

            state.set_bid(
                BidKind::Delegator(Box::new(delegator_bid)),
                slash_instead_of_unbonding,
            );
        }

        if *existing_recipient.stake() == *updated_recipient.stake() {
            // if the delegators changed, do the above, but if the validator's
            // personal stake is unchanged their bid doesn't need to be modified.
            return;
        }

        let updated_bid = ValidatorBid::unlocked(
            validator_public_key.clone(),
            *bonding_purse,
            *updated_recipient.stake(),
            *updated_recipient.delegation_rate(),
            min_delegation_amount,
            max_delegation_amount,
            reserved_slots,
        );

        state.set_bid(
            BidKind::Validator(Box::new(updated_bid)),
            slash_instead_of_unbonding,
        );
        return;
    }

    // new bid
    let stake = *updated_recipient.stake();
    if stake.is_zero() {
        return;
    }

    for (delegator_pub_key, delegator_stake) in updated_recipient.delegator_stake() {
        let delegator_bonding_purse = state.create_purse(*delegator_stake);
        let delegator_bid = DelegatorBid::unlocked(
            delegator_pub_key.clone(),
            *delegator_stake,
            delegator_bonding_purse,
            validator_public_key.clone(),
        );

        state.set_bid(
            BidKind::Delegator(Box::new(delegator_bid)),
            slash_instead_of_unbonding,
        );
    }

    let bonding_purse = state.create_purse(stake);
    let validator_bid = ValidatorBid::unlocked(
        validator_public_key.clone(),
        bonding_purse,
        stake,
        *updated_recipient.delegation_rate(),
        0,
        u64::MAX,
        0,
    );
    state.set_bid(
        BidKind::Validator(Box::new(validator_bid)),
        slash_instead_of_unbonding,
    );
}
