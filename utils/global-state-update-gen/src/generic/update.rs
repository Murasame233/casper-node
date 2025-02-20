use std::collections::BTreeMap;
#[cfg(test)]
use std::collections::HashSet;

#[cfg(test)]
use casper_types::{account::AccountHash, AddressableEntity, CLValue, PublicKey, URef, U512};
use casper_types::{Key, StoredValue};

#[cfg(test)]
use casper_types::system::auction::{
    BidAddr, BidKind, DelegatorBid, DelegatorKind, UnbondKind, ValidatorBid,
};

#[cfg(test)]
use super::state_reader::StateReader;

use crate::utils::{print_entry, print_validators, ValidatorInfo};

#[derive(Debug)]
pub(crate) struct Update {
    entries: BTreeMap<Key, StoredValue>,
    // Holds the complete set of validators, only if the validator set changed
    validators: Option<Vec<ValidatorInfo>>,
}

impl Update {
    pub(crate) fn new(
        entries: BTreeMap<Key, StoredValue>,
        validators: Option<Vec<ValidatorInfo>>,
    ) -> Self {
        Self {
            entries,
            validators,
        }
    }

    pub(crate) fn print(&self) {
        if let Some(validators) = &self.validators {
            print_validators(validators);
        }
        for (key, value) in &self.entries {
            print_entry(key, value);
        }
    }
}

#[cfg(test)]
impl Update {
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn get_written_addressable_entity(
        &self,
        account_hash: AccountHash,
    ) -> AddressableEntity {
        let entity_key = self
            .entries
            .get(&Key::Account(account_hash))
            .expect("must have Key for account hash")
            .as_cl_value()
            .expect("must have underlying cl value")
            .to_owned()
            .into_t::<Key>()
            .expect("must convert to key");

        self.entries
            .get(&entity_key)
            .expect("must have addressable entity")
            .as_addressable_entity()
            .expect("should be addressable entity")
            .clone()
    }

    pub(crate) fn get_written_bid(&self, account: AccountHash) -> BidKind {
        self.entries
            .get(&Key::BidAddr(BidAddr::from(account)))
            .expect("stored value should exist")
            .as_bid_kind()
            .expect("stored value should be be BidKind")
            .clone()
    }

    #[track_caller]
    pub(crate) fn get_total_stake(&self, account: AccountHash) -> Option<U512> {
        let bid_addr = BidAddr::from(account);

        if let BidKind::Validator(validator_bid) = self
            .entries
            .get(&bid_addr.into())
            .expect("should create bid")
            .as_bid_kind()
            .expect("should be bid")
        {
            let delegator_stake: U512 = self
                .delegators(validator_bid)
                .iter()
                .map(|x| x.staked_amount())
                .sum();

            Some(validator_bid.staked_amount() + delegator_stake)
        } else {
            None
        }
    }

    #[track_caller]
    pub(crate) fn delegators(&self, validator_bid: &ValidatorBid) -> Vec<DelegatorBid> {
        let mut ret = vec![];

        for (_, v) in self.entries.clone() {
            if let StoredValue::BidKind(BidKind::Delegator(delegator)) = v {
                if delegator.validator_public_key() != validator_bid.validator_public_key() {
                    continue;
                }
                ret.push(*delegator);
            }
        }

        ret
    }

    #[track_caller]
    pub(crate) fn delegator(
        &self,
        validator_bid: &ValidatorBid,
        delegator_kind: &DelegatorKind,
    ) -> Option<DelegatorBid> {
        let delegators = self.delegators(validator_bid);
        for delegator in delegators {
            if delegator.delegator_kind() != delegator_kind {
                continue;
            }
            return Some(delegator);
        }
        None
    }

    #[track_caller]
    pub(crate) fn assert_written_balance(&self, purse: URef, balance: u64) {
        if let StoredValue::CLValue(cl_value) = self
            .entries
            .get(&Key::Balance(purse.addr()))
            .expect("must have balance")
        {
            let actual = CLValue::to_t::<U512>(cl_value).expect("must get u512");
            assert_eq!(actual, U512::from(balance))
        };
    }

    #[track_caller]
    pub(crate) fn assert_total_supply<R: StateReader>(&self, reader: &mut R, supply: u64) {
        let total = self
            .entries
            .get(&reader.get_total_supply_key())
            .expect("should have total supply")
            .as_cl_value()
            .expect("total supply should be CLValue")
            .clone()
            .into_t::<U512>()
            .expect("total supply should be a U512");
        let expected = U512::from(supply);
        assert_eq!(
            total, expected,
            "total supply ({total}) not as expected ({expected})",
        );
    }

    #[track_caller]
    pub(crate) fn assert_written_purse_is_unit(&self, purse: URef) {
        assert_eq!(
            self.entries.get(&Key::URef(purse)),
            Some(&StoredValue::from(
                CLValue::from_t(()).expect("should convert unit to CLValue")
            ))
        );
    }

    #[track_caller]
    pub(crate) fn assert_seigniorage_recipients_written<R: StateReader>(&self, reader: &mut R) {
        assert!(self
            .entries
            .contains_key(&reader.get_seigniorage_recipients_key()));
    }

    #[track_caller]
    pub(crate) fn assert_written_bid(&self, account: AccountHash, bid: BidKind) {
        assert_eq!(
            self.entries.get(&Key::BidAddr(BidAddr::from(account))),
            Some(&StoredValue::from(bid))
        );
    }

    #[track_caller]
    pub(crate) fn assert_withdraw_purse(
        &self,
        bid_purse: URef,
        validator_key: &PublicKey,
        unbonder_key: &PublicKey,
        amount: u64,
    ) {
        let account_hash = validator_key.to_account_hash();
        let withdraws = self
            .entries
            .get(&Key::Withdraw(account_hash))
            .expect("should have withdraws for the account")
            .as_withdraw()
            .expect("should be withdraw purses");
        assert!(withdraws.iter().any(
            |withdraw_purse| withdraw_purse.bonding_purse() == &bid_purse
                && withdraw_purse.validator_public_key() == validator_key
                && withdraw_purse.unbonder_public_key() == unbonder_key
                && withdraw_purse.amount() == &U512::from(amount)
        ))
    }

    #[track_caller]
    #[allow(unused)]
    pub(crate) fn assert_unbonding_purse(
        &self,
        bid_purse: URef,
        validator_key: &PublicKey,
        unbonder_key: &PublicKey,
        amount: u64,
    ) {
        let account_hash = unbonder_key.to_account_hash();
        let unbonds = self
            .entries
            .get(&Key::Unbond(account_hash))
            .expect("should have unbonds for the account")
            .as_unbonding()
            .expect("should be unbonding purses");
        assert!(unbonds.iter().any(
            |unbonding_purse| unbonding_purse.bonding_purse() == &bid_purse
                && unbonding_purse.validator_public_key() == validator_key
                && unbonding_purse.unbonder_public_key() == unbonder_key
                && unbonding_purse.amount() == &U512::from(amount)
        ))
    }

    /// `expected`: (bid_purse, unbonder_key, amount)
    #[track_caller]
    #[allow(unused)]
    pub(crate) fn assert_unbonding_purses<'a>(
        &self,
        account_hash: AccountHash,
        expected: impl IntoIterator<Item = (URef, &'a PublicKey, u64)>,
    ) {
        let mut expected: Vec<_> = expected
            .into_iter()
            .map(|(bid_purse, unbonder_key, amount)| {
                (account_hash, bid_purse, unbonder_key, U512::from(amount))
            })
            .collect();
        let mut data: Vec<_> = self
            .entries
            .get(&Key::Unbond(account_hash))
            .expect("should have unbonds for the account")
            .as_unbonding()
            .expect("should be unbonding purses")
            .iter()
            .map(|unbonding_purse| {
                (
                    unbonding_purse.unbonder_public_key().to_account_hash(),
                    *unbonding_purse.bonding_purse(),
                    unbonding_purse.unbonder_public_key(),
                    *unbonding_purse.amount(),
                )
            })
            .collect();

        expected.sort();
        data.sort();

        assert_eq!(
            data, expected,
            "\nThe data we got:\n{data:#?}\nExpected values:\n{expected:#?}"
        );
    }

    #[track_caller]
    pub(crate) fn assert_unbond_bid_kind(
        &self,
        bid_purse: URef,
        validator_key: &PublicKey,
        unbond_kind: &UnbondKind,
        amount: u64,
    ) {
        println!(
            "assert_unbond_bid_kind {:?} {:?}",
            validator_key,
            validator_key.to_account_hash()
        );
        println!("assert_unbond_bid_kind {:?}", unbond_kind);
        let bid_addr = match unbond_kind {
            UnbondKind::Validator(pk) | UnbondKind::DelegatedPublicKey(pk) => {
                BidAddr::UnbondAccount {
                    validator: validator_key.to_account_hash(),
                    unbonder: pk.to_account_hash(),
                }
            }
            UnbondKind::DelegatedPurse(addr) => BidAddr::UnbondPurse {
                validator: validator_key.to_account_hash(),
                unbonder: *addr,
            },
        };

        println!("assert_unbond_bid_kind {:?}", Key::BidAddr(bid_addr));

        let entries = &self.entries;
        let unbonds = entries
            .get(&Key::BidAddr(bid_addr))
            .expect("should have record")
            .as_unbond()
            .expect("should be unbond");

        assert!(unbonds
            .eras()
            .iter()
            .any(|unbond_era| unbond_era.bonding_purse() == &bid_purse
                && unbond_era.amount() == &U512::from(amount)))
    }

    #[track_caller]
    pub(crate) fn assert_key_absent(&self, key: &Key) {
        assert!(!self.entries.contains_key(key))
    }

    #[track_caller]
    pub(crate) fn assert_validators(&self, validators: &[ValidatorInfo]) {
        let self_set: HashSet<_> = self.validators.as_ref().unwrap().iter().collect();
        let other_set: HashSet<_> = validators.iter().collect();
        assert_eq!(self_set, other_set);
    }

    #[track_caller]
    pub(crate) fn assert_validators_unchanged(&self) {
        assert!(self.validators.is_none());
    }
}
