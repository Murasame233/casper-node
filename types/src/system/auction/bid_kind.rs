use crate::{
    bytesrepr::{self, FromBytes, ToBytes, U8_SERIALIZED_LENGTH},
    system::auction::{
        bid::VestingSchedule, Bid, BidAddr, DelegatorBid, ValidatorBid, ValidatorCredit,
    },
    CLType, CLTyped, EraId, PublicKey, URef, U512,
};

use crate::system::auction::{
    delegator_kind::DelegatorKind,
    unbond::{Unbond, UnbondKind},
    Bridge,
};
use alloc::{boxed::Box, vec::Vec};
#[cfg(feature = "datasize")]
use datasize::DataSize;
#[cfg(feature = "json-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Reservation;

/// BidKindTag variants.
#[allow(clippy::large_enum_variant)]
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum BidKindTag {
    /// Unified bid.
    Unified = 0,
    /// Validator bid.
    Validator = 1,
    /// Delegator bid.
    Delegator = 2,
    /// Bridge record.
    Bridge = 3,
    /// Validator credit bid.
    Credit = 4,
    /// Reservation bid.
    Reservation = 5,
    /// Unbond.
    Unbond = 6,
}

/// Auction bid variants.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "datasize", derive(DataSize))]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
pub enum BidKind {
    /// A unified record indexed on validator data, with an embedded collection of all delegator
    /// bids assigned to that validator. The Unified variant is for legacy retrograde support, new
    /// instances will not be created going forward.
    Unified(Box<Bid>),
    /// A bid record containing only validator data.
    Validator(Box<ValidatorBid>),
    /// A bid record containing only delegator data.
    Delegator(Box<DelegatorBid>),
    /// A bridge record pointing to a new `ValidatorBid` after the public key was changed.
    Bridge(Box<Bridge>),
    /// Credited amount.
    Credit(Box<ValidatorCredit>),
    /// Reservation
    Reservation(Box<Reservation>),
    /// Unbond
    Unbond(Box<Unbond>),
}

impl BidKind {
    /// Returns validator public key.
    pub fn validator_public_key(&self) -> PublicKey {
        match self {
            BidKind::Unified(bid) => bid.validator_public_key().clone(),
            BidKind::Validator(validator_bid) => validator_bid.validator_public_key().clone(),
            BidKind::Delegator(delegator_bid) => delegator_bid.validator_public_key().clone(),
            BidKind::Bridge(bridge) => bridge.old_validator_public_key().clone(),
            BidKind::Credit(validator_credit) => validator_credit.validator_public_key().clone(),
            BidKind::Reservation(reservation) => reservation.validator_public_key().clone(),
            BidKind::Unbond(unbond) => unbond.validator_public_key().clone(),
        }
    }

    /// Returns new validator public key if it was changed.
    pub fn new_validator_public_key(&self) -> Option<PublicKey> {
        match self {
            BidKind::Bridge(bridge) => Some(bridge.new_validator_public_key().clone()),
            BidKind::Unified(_)
            | BidKind::Validator(_)
            | BidKind::Delegator(_)
            | BidKind::Credit(_)
            | BidKind::Reservation(_)
            | BidKind::Unbond(_) => None,
        }
    }

    /// Returns BidAddr.
    pub fn bid_addr(&self) -> BidAddr {
        match self {
            BidKind::Unified(bid) => BidAddr::Unified(bid.validator_public_key().to_account_hash()),
            BidKind::Validator(validator_bid) => {
                BidAddr::Validator(validator_bid.validator_public_key().to_account_hash())
            }
            BidKind::Delegator(delegator_bid) => {
                let validator = delegator_bid.validator_public_key().to_account_hash();
                match delegator_bid.delegator_kind() {
                    DelegatorKind::PublicKey(pk) => BidAddr::DelegatedAccount {
                        validator,
                        delegator: pk.to_account_hash(),
                    },
                    DelegatorKind::Purse(addr) => BidAddr::DelegatedPurse {
                        validator,
                        delegator: *addr,
                    },
                }
            }
            BidKind::Bridge(bridge) => {
                BidAddr::Validator(bridge.old_validator_public_key().to_account_hash())
            }
            BidKind::Credit(credit) => {
                let validator = credit.validator_public_key().to_account_hash();
                let era_id = credit.era_id();
                BidAddr::Credit { validator, era_id }
            }
            BidKind::Reservation(reservation_bid) => {
                let validator = reservation_bid.validator_public_key().to_account_hash();
                match reservation_bid.delegator_kind() {
                    DelegatorKind::PublicKey(pk) => BidAddr::ReservedDelegationAccount {
                        validator,
                        delegator: pk.to_account_hash(),
                    },
                    DelegatorKind::Purse(addr) => BidAddr::ReservedDelegationPurse {
                        validator,
                        delegator: *addr,
                    },
                }
            }
            BidKind::Unbond(unbond) => {
                let validator = unbond.validator_public_key().to_account_hash();
                let unbond_kind = unbond.unbond_kind();
                match unbond_kind {
                    UnbondKind::Validator(pk) | UnbondKind::DelegatedPublicKey(pk) => {
                        BidAddr::UnbondAccount {
                            validator,
                            unbonder: pk.to_account_hash(),
                        }
                    }
                    UnbondKind::DelegatedPurse(addr) => BidAddr::UnbondPurse {
                        validator,
                        unbonder: *addr,
                    },
                }
            }
        }
    }

    /// Is this instance a unified bid?
    pub fn is_unified(&self) -> bool {
        matches!(self, BidKind::Unified(_))
    }

    /// Is this instance a validator bid?
    pub fn is_validator(&self) -> bool {
        matches!(self, BidKind::Validator(_))
    }

    /// Is this instance a delegator bid?
    pub fn is_delegator(&self) -> bool {
        matches!(self, BidKind::Delegator(_))
    }

    /// Is this instance a bridge record?
    pub fn is_bridge(&self) -> bool {
        matches!(self, BidKind::Bridge(_))
    }

    /// Is this instance a validator credit?
    pub fn is_credit(&self) -> bool {
        matches!(self, BidKind::Credit(_))
    }

    /// Is this instance a reservation?
    pub fn is_reservation(&self) -> bool {
        matches!(self, BidKind::Reservation(_))
    }

    /// Is this instance a unbond record?
    pub fn is_unbond(&self) -> bool {
        matches!(self, BidKind::Unbond(_))
    }

    /// The staked amount.
    pub fn staked_amount(&self) -> Option<U512> {
        match self {
            BidKind::Unified(bid) => Some(*bid.staked_amount()),
            BidKind::Validator(validator_bid) => Some(validator_bid.staked_amount()),
            BidKind::Delegator(delegator) => Some(delegator.staked_amount()),
            BidKind::Credit(credit) => Some(credit.amount()),
            BidKind::Bridge(_) | BidKind::Reservation(_) | BidKind::Unbond(_) => None,
        }
    }

    /// The bonding purse.
    pub fn bonding_purse(&self) -> Option<URef> {
        match self {
            BidKind::Unified(bid) => Some(*bid.bonding_purse()),
            BidKind::Validator(validator_bid) => Some(*validator_bid.bonding_purse()),
            BidKind::Delegator(delegator) => Some(*delegator.bonding_purse()),
            BidKind::Unbond(_)
            | BidKind::Bridge(_)
            | BidKind::Credit(_)
            | BidKind::Reservation(_) => None,
        }
    }

    /// The delegator kind, if relevant.
    pub fn delegator_kind(&self) -> Option<DelegatorKind> {
        match self {
            BidKind::Unified(_)
            | BidKind::Validator(_)
            | BidKind::Bridge(_)
            | BidKind::Credit(_) => None,
            BidKind::Unbond(unbond) => match unbond.unbond_kind() {
                UnbondKind::Validator(_) => None,
                UnbondKind::DelegatedPublicKey(pk) => Some(DelegatorKind::PublicKey(pk.clone())),
                UnbondKind::DelegatedPurse(addr) => Some(DelegatorKind::Purse(*addr)),
            },
            BidKind::Delegator(bid) => Some(bid.delegator_kind().clone()),
            BidKind::Reservation(bid) => Some(bid.delegator_kind().clone()),
        }
    }

    /// The unbond kind, if relevant.
    pub fn unbond_kind(&self) -> Option<UnbondKind> {
        match self {
            BidKind::Unified(_)
            | BidKind::Validator(_)
            | BidKind::Bridge(_)
            | BidKind::Credit(_)
            | BidKind::Delegator(_)
            | BidKind::Reservation(_) => None,
            BidKind::Unbond(unbond) => Some(unbond.unbond_kind().clone()),
        }
    }

    /// Is this bid inactive?
    pub fn inactive(&self) -> bool {
        match self {
            BidKind::Unified(bid) => bid.inactive(),
            BidKind::Validator(validator_bid) => validator_bid.inactive(),
            BidKind::Delegator(delegator) => delegator.staked_amount().is_zero(),
            BidKind::Credit(credit) => credit.amount().is_zero(),
            BidKind::Bridge(_) | BidKind::Reservation(_) | BidKind::Unbond(_) => false,
        }
    }

    /// Checks if a bid is still locked under a vesting schedule.
    ///
    /// Returns true if a timestamp falls below the initial lockup period + 91 days release
    /// schedule, otherwise false.
    pub fn is_locked(&self, timestamp_millis: u64) -> bool {
        match self {
            BidKind::Unified(bid) => bid.is_locked(timestamp_millis),
            BidKind::Validator(validator_bid) => validator_bid.is_locked(timestamp_millis),
            BidKind::Delegator(delegator) => delegator.is_locked(timestamp_millis),
            BidKind::Bridge(_)
            | BidKind::Credit(_)
            | BidKind::Reservation(_)
            | BidKind::Unbond(_) => false,
        }
    }

    /// Checks if a bid is still locked under a vesting schedule.
    ///
    /// Returns true if a timestamp falls below the initial lockup period + 91 days release
    /// schedule, otherwise false.
    pub fn is_locked_with_vesting_schedule(
        &self,
        timestamp_millis: u64,
        vesting_schedule_period_millis: u64,
    ) -> bool {
        match self {
            BidKind::Unified(bid) => bid
                .is_locked_with_vesting_schedule(timestamp_millis, vesting_schedule_period_millis),
            BidKind::Validator(validator_bid) => validator_bid
                .is_locked_with_vesting_schedule(timestamp_millis, vesting_schedule_period_millis),
            BidKind::Delegator(delegator) => delegator
                .is_locked_with_vesting_schedule(timestamp_millis, vesting_schedule_period_millis),
            BidKind::Bridge(_)
            | BidKind::Credit(_)
            | BidKind::Reservation(_)
            | BidKind::Unbond(_) => false,
        }
    }

    /// Returns a reference to the vesting schedule of the provided bid.  `None` if a non-genesis
    /// validator.
    pub fn vesting_schedule(&self) -> Option<&VestingSchedule> {
        match self {
            BidKind::Unified(bid) => bid.vesting_schedule(),
            BidKind::Validator(validator_bid) => validator_bid.vesting_schedule(),
            BidKind::Delegator(delegator) => delegator.vesting_schedule(),
            BidKind::Bridge(_)
            | BidKind::Credit(_)
            | BidKind::Reservation(_)
            | BidKind::Unbond(_) => None,
        }
    }

    /// BidKindTag.
    pub fn tag(&self) -> BidKindTag {
        match self {
            BidKind::Unified(_) => BidKindTag::Unified,
            BidKind::Validator(_) => BidKindTag::Validator,
            BidKind::Delegator(_) => BidKindTag::Delegator,
            BidKind::Bridge(_) => BidKindTag::Bridge,
            BidKind::Credit(_) => BidKindTag::Credit,
            BidKind::Reservation(_) => BidKindTag::Reservation,
            BidKind::Unbond(_) => BidKindTag::Unbond,
        }
    }

    /// The `[EraId]` associated with this `[BidKind]`, if any.
    pub fn era_id(&self) -> Option<EraId> {
        match self {
            BidKind::Bridge(bridge) => Some(*bridge.era_id()),
            BidKind::Credit(credit) => Some(credit.era_id()),
            BidKind::Unified(_)
            | BidKind::Validator(_)
            | BidKind::Delegator(_)
            | BidKind::Reservation(_)
            | BidKind::Unbond(_) => None,
        }
    }
}

impl CLTyped for BidKind {
    fn cl_type() -> CLType {
        CLType::Any
    }
}

impl ToBytes for BidKind {
    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        let mut result = bytesrepr::allocate_buffer(self)?;
        let (tag, mut serialized_data) = match self {
            BidKind::Unified(bid) => (BidKindTag::Unified, bid.to_bytes()?),
            BidKind::Validator(validator_bid) => (BidKindTag::Validator, validator_bid.to_bytes()?),
            BidKind::Delegator(delegator_bid) => (BidKindTag::Delegator, delegator_bid.to_bytes()?),
            BidKind::Bridge(bridge) => (BidKindTag::Bridge, bridge.to_bytes()?),
            BidKind::Credit(credit) => (BidKindTag::Credit, credit.to_bytes()?),
            BidKind::Reservation(reservation) => (BidKindTag::Reservation, reservation.to_bytes()?),
            BidKind::Unbond(unbond) => (BidKindTag::Unbond, unbond.to_bytes()?),
        };
        result.push(tag as u8);
        result.append(&mut serialized_data);
        Ok(result)
    }

    fn serialized_length(&self) -> usize {
        U8_SERIALIZED_LENGTH
            + match self {
                BidKind::Unified(bid) => bid.serialized_length(),
                BidKind::Validator(validator_bid) => validator_bid.serialized_length(),
                BidKind::Delegator(delegator_bid) => delegator_bid.serialized_length(),
                BidKind::Bridge(bridge) => bridge.serialized_length(),
                BidKind::Credit(credit) => credit.serialized_length(),
                BidKind::Reservation(reservation) => reservation.serialized_length(),
                BidKind::Unbond(unbond) => unbond.serialized_length(),
            }
    }

    fn write_bytes(&self, writer: &mut Vec<u8>) -> Result<(), bytesrepr::Error> {
        writer.push(self.tag() as u8);
        match self {
            BidKind::Unified(bid) => bid.write_bytes(writer)?,
            BidKind::Validator(validator_bid) => validator_bid.write_bytes(writer)?,
            BidKind::Delegator(delegator_bid) => delegator_bid.write_bytes(writer)?,
            BidKind::Bridge(bridge) => bridge.write_bytes(writer)?,
            BidKind::Credit(credit) => credit.write_bytes(writer)?,
            BidKind::Reservation(reservation) => reservation.write_bytes(writer)?,
            BidKind::Unbond(unbond) => unbond.write_bytes(writer)?,
        };
        Ok(())
    }
}

impl FromBytes for BidKind {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        let (tag, remainder): (u8, &[u8]) = FromBytes::from_bytes(bytes)?;
        match tag {
            tag if tag == BidKindTag::Unified as u8 => Bid::from_bytes(remainder)
                .map(|(bid, remainder)| (BidKind::Unified(Box::new(bid)), remainder)),
            tag if tag == BidKindTag::Validator as u8 => {
                ValidatorBid::from_bytes(remainder).map(|(validator_bid, remainder)| {
                    (BidKind::Validator(Box::new(validator_bid)), remainder)
                })
            }
            tag if tag == BidKindTag::Delegator as u8 => {
                DelegatorBid::from_bytes(remainder).map(|(delegator_bid, remainder)| {
                    (BidKind::Delegator(Box::new(delegator_bid)), remainder)
                })
            }
            tag if tag == BidKindTag::Bridge as u8 => Bridge::from_bytes(remainder)
                .map(|(bridge, remainder)| (BidKind::Bridge(Box::new(bridge)), remainder)),
            tag if tag == BidKindTag::Credit as u8 => ValidatorCredit::from_bytes(remainder)
                .map(|(credit, remainder)| (BidKind::Credit(Box::new(credit)), remainder)),
            tag if tag == BidKindTag::Reservation as u8 => {
                Reservation::from_bytes(remainder).map(|(reservation, remainder)| {
                    (BidKind::Reservation(Box::new(reservation)), remainder)
                })
            }
            tag if tag == BidKindTag::Unbond as u8 => Unbond::from_bytes(remainder)
                .map(|(unbond, remainder)| (BidKind::Unbond(Box::new(unbond)), remainder)),
            _ => Err(bytesrepr::Error::Formatting),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BidKind, *};
    use crate::{
        bytesrepr,
        system::auction::{delegator_kind::DelegatorKind, DelegationRate},
        AccessRights, SecretKey,
    };

    #[test]
    fn serialization_roundtrip() {
        let validator_public_key = PublicKey::from(
            &SecretKey::ed25519_from_bytes([0u8; SecretKey::ED25519_LENGTH]).unwrap(),
        );
        let bonding_purse = URef::new([42; 32], AccessRights::READ_ADD_WRITE);
        let bid = Bid::unlocked(
            validator_public_key.clone(),
            bonding_purse,
            U512::one(),
            DelegationRate::MAX,
        );
        let unified_bid = BidKind::Unified(Box::new(bid.clone()));
        let validator_bid = ValidatorBid::from(bid.clone());

        let delegator_public_key = PublicKey::from(
            &SecretKey::ed25519_from_bytes([1u8; SecretKey::ED25519_LENGTH]).unwrap(),
        );

        let delegator_kind = DelegatorKind::PublicKey(delegator_public_key);

        let delegator = DelegatorBid::unlocked(
            delegator_kind,
            U512::one(),
            bonding_purse,
            validator_public_key.clone(),
        );
        let delegator_bid = BidKind::Delegator(Box::new(delegator));

        let credit = ValidatorCredit::new(validator_public_key, EraId::new(0), U512::one());
        let credit_bid = BidKind::Credit(Box::new(credit));

        bytesrepr::test_serialization_roundtrip(&bid);
        bytesrepr::test_serialization_roundtrip(&unified_bid);
        bytesrepr::test_serialization_roundtrip(&validator_bid);
        bytesrepr::test_serialization_roundtrip(&delegator_bid);
        bytesrepr::test_serialization_roundtrip(&credit_bid);
    }
}

#[cfg(test)]
mod prop_test_bid_kind_unified {
    use proptest::prelude::*;

    use crate::{bytesrepr, gens};

    proptest! {
        #[test]
        fn test_value_bid_kind_unified(bid_kind in gens::unified_bid_arb(0..3)) {
            bytesrepr::test_serialization_roundtrip(&bid_kind);
        }
    }
}

#[cfg(test)]
mod prop_test_bid_kind_validator {
    use proptest::prelude::*;

    use crate::{bytesrepr, gens};

    proptest! {
        #[test]
        fn test_value_bid_kind_validator(bid_kind in gens::validator_bid_arb()) {
            bytesrepr::test_serialization_roundtrip(&bid_kind);
        }
    }
}

#[cfg(test)]
mod prop_test_bid_kind_delegator {
    use proptest::prelude::*;

    use crate::{bytesrepr, gens};

    proptest! {
        #[test]
        fn test_value_bid_kind_delegator(bid_kind in gens::delegator_bid_kind_arb()) {
            bytesrepr::test_serialization_roundtrip(&bid_kind);
        }
    }
}

#[cfg(test)]
mod prop_test_bid_kind_validator_credit {
    use proptest::prelude::*;

    use crate::{bytesrepr, gens};

    proptest! {
        #[test]
        fn test_value_bid_kind_validator_credit(bid_kind in gens::credit_bid_arb()) {
            bytesrepr::test_serialization_roundtrip(&bid_kind);
        }
    }
}

#[cfg(test)]
mod prop_test_bid_kind_reservation {
    use proptest::prelude::*;

    use crate::{bytesrepr, gens};

    proptest! {
        #[test]
        fn test_value_bid_kind_reservation(bid_kind in gens::reservation_bid_arb()) {
            bytesrepr::test_serialization_roundtrip(&bid_kind);
        }
    }
}
