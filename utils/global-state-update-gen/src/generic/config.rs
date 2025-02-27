use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use casper_types::{
    account::AccountHash,
    system::auction::{DelegationRate, DelegatorKind},
    ProtocolVersion, PublicKey, U512,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub transfers: Vec<Transfer>,
    #[serde(default)]
    pub accounts: Vec<AccountConfig>,
    #[serde(default)]
    pub only_listed_validators: bool,
    #[serde(default)]
    pub slash_instead_of_unbonding: bool,
    #[serde(default)]
    pub protocol_version: ProtocolVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transfer {
    pub from: AccountHash,
    pub to: AccountHash,
    pub amount: U512,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    pub public_key: PublicKey,
    pub balance: Option<U512>,
    pub validator: Option<ValidatorConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValidatorConfig {
    pub bonded_amount: U512,
    pub delegation_rate: Option<u8>,
    pub delegators: Option<Vec<DelegatorConfig>>,
    pub reservations: Option<Vec<ReservationConfig>>,
}

impl ValidatorConfig {
    pub fn delegators_map(&self) -> Option<BTreeMap<DelegatorKind, U512>> {
        self.delegators.as_ref().map(|delegators| {
            delegators
                .iter()
                .map(|delegator| {
                    (
                        DelegatorKind::PublicKey(delegator.public_key.clone()),
                        delegator.delegated_amount,
                    )
                })
                .collect()
        })
    }

    pub fn reservations_map(&self) -> Option<BTreeMap<DelegatorKind, DelegationRate>> {
        self.reservations.as_ref().map(|reservations| {
            reservations
                .iter()
                .map(|reservation| {
                    (
                        DelegatorKind::PublicKey(reservation.public_key.clone()),
                        reservation.delegation_rate,
                    )
                })
                .collect()
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatorConfig {
    pub public_key: PublicKey,
    pub delegated_amount: U512,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservationConfig {
    pub public_key: PublicKey,
    pub delegation_rate: DelegationRate,
}
