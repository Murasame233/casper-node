//! This module contains types and functions for working with keys associated with an account.

use alloc::{
    collections::{btree_map::Entry, BTreeMap, BTreeSet},
    vec::Vec,
};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[cfg(feature = "datasize")]
use datasize::DataSize;
#[cfg(feature = "json-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[cfg(feature = "json-schema")]
use serde_map_to_array::KeyValueJsonSchema;
use serde_map_to_array::{BTreeMapToArray, KeyValueLabels};

use crate::{
    account::{AccountHash, TryFromIntError, Weight},
    bytesrepr::{self, FromBytes, ToBytes},
};

/// Errors that can occur while adding a new [`AccountHash`] to an account's associated keys map.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
#[repr(i32)]
#[non_exhaustive]
pub enum AddKeyFailure {
    /// There are already maximum [`AccountHash`]s associated with the given account.
    MaxKeysLimit = 1,
    /// The given [`AccountHash`] is already associated with the given account.
    DuplicateKey = 2,
    /// Caller doesn't have sufficient permissions to associate a new [`AccountHash`] with the
    /// given account.
    PermissionDenied = 3,
}

impl Display for AddKeyFailure {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            AddKeyFailure::MaxKeysLimit => formatter.write_str(
                "Unable to add new associated key because maximum amount of keys is reached",
            ),
            AddKeyFailure::DuplicateKey => formatter
                .write_str("Unable to add new associated key because given key already exists"),
            AddKeyFailure::PermissionDenied => formatter
                .write_str("Unable to add new associated key due to insufficient permissions"),
        }
    }
}

// This conversion is not intended to be used by third party crates.
#[doc(hidden)]
impl TryFrom<i32> for AddKeyFailure {
    type Error = TryFromIntError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            d if d == AddKeyFailure::MaxKeysLimit as i32 => Ok(AddKeyFailure::MaxKeysLimit),
            d if d == AddKeyFailure::DuplicateKey as i32 => Ok(AddKeyFailure::DuplicateKey),
            d if d == AddKeyFailure::PermissionDenied as i32 => Ok(AddKeyFailure::PermissionDenied),
            _ => Err(TryFromIntError(())),
        }
    }
}

/// Errors that can occur while removing a [`AccountHash`] from an account's associated keys map.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[repr(i32)]
#[non_exhaustive]
pub enum RemoveKeyFailure {
    /// The given [`AccountHash`] is not associated with the given account.
    MissingKey = 1,
    /// Caller doesn't have sufficient permissions to remove an associated [`AccountHash`] from the
    /// given account.
    PermissionDenied = 2,
    /// Removing the given associated [`AccountHash`] would cause the total weight of all remaining
    /// `AccountHash`s to fall below one of the action thresholds for the given account.
    ThresholdViolation = 3,
}

impl Display for RemoveKeyFailure {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            RemoveKeyFailure::MissingKey => {
                formatter.write_str("Unable to remove a key that does not exist")
            }
            RemoveKeyFailure::PermissionDenied => formatter
                .write_str("Unable to remove associated key due to insufficient permissions"),
            RemoveKeyFailure::ThresholdViolation => formatter.write_str(
                "Unable to remove a key which would violate action threshold constraints",
            ),
        }
    }
}

// This conversion is not intended to be used by third party crates.
#[doc(hidden)]
impl TryFrom<i32> for RemoveKeyFailure {
    type Error = TryFromIntError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            d if d == RemoveKeyFailure::MissingKey as i32 => Ok(RemoveKeyFailure::MissingKey),
            d if d == RemoveKeyFailure::PermissionDenied as i32 => {
                Ok(RemoveKeyFailure::PermissionDenied)
            }
            d if d == RemoveKeyFailure::ThresholdViolation as i32 => {
                Ok(RemoveKeyFailure::ThresholdViolation)
            }
            _ => Err(TryFromIntError(())),
        }
    }
}

/// Errors that can occur while updating the [`crate::addressable_entity::Weight`] of a
/// [`AccountHash`] in an account's associated keys map.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
#[repr(i32)]
#[non_exhaustive]
pub enum UpdateKeyFailure {
    /// The given [`AccountHash`] is not associated with the given account.
    MissingKey = 1,
    /// Caller doesn't have sufficient permissions to update an associated [`AccountHash`] from the
    /// given account.
    PermissionDenied = 2,
    /// Updating the [`crate::addressable_entity::Weight`] of the given associated [`AccountHash`]
    /// would cause the total weight of all `AccountHash`s to fall below one of the action
    /// thresholds for the given account.
    ThresholdViolation = 3,
}

impl Display for UpdateKeyFailure {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            UpdateKeyFailure::MissingKey => formatter.write_str(
                "Unable to update the value under an associated key that does not exist",
            ),
            UpdateKeyFailure::PermissionDenied => formatter
                .write_str("Unable to update associated key due to insufficient permissions"),
            UpdateKeyFailure::ThresholdViolation => formatter.write_str(
                "Unable to update weight that would fall below any of action thresholds",
            ),
        }
    }
}

// This conversion is not intended to be used by third party crates.
#[doc(hidden)]
impl TryFrom<i32> for UpdateKeyFailure {
    type Error = TryFromIntError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            d if d == UpdateKeyFailure::MissingKey as i32 => Ok(UpdateKeyFailure::MissingKey),
            d if d == UpdateKeyFailure::PermissionDenied as i32 => {
                Ok(UpdateKeyFailure::PermissionDenied)
            }
            d if d == UpdateKeyFailure::ThresholdViolation as i32 => {
                Ok(UpdateKeyFailure::ThresholdViolation)
            }
            _ => Err(TryFromIntError(())),
        }
    }
}

/// A collection of weighted public keys (represented as account hashes) associated with an account.
#[derive(Default, PartialOrd, Ord, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "datasize", derive(DataSize))]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[cfg_attr(feature = "json-schema", schemars(rename = "AccountAssociatedKeys"))]
#[serde(deny_unknown_fields)]
#[rustfmt::skip]
pub struct AssociatedKeys(
    #[serde(with = "BTreeMapToArray::<AccountHash, Weight, Labels>")]
    BTreeMap<AccountHash, Weight>,
);

impl AssociatedKeys {
    /// Constructs a new AssociatedKeys.
    pub fn new(key: AccountHash, weight: Weight) -> AssociatedKeys {
        let mut bt: BTreeMap<AccountHash, Weight> = BTreeMap::new();
        bt.insert(key, weight);
        AssociatedKeys(bt)
    }

    /// Adds a new AssociatedKey to the set.
    ///
    /// Returns true if added successfully, false otherwise.
    pub fn add_key(&mut self, key: AccountHash, weight: Weight) -> Result<(), AddKeyFailure> {
        match self.0.entry(key) {
            Entry::Vacant(entry) => {
                entry.insert(weight);
            }
            Entry::Occupied(_) => return Err(AddKeyFailure::DuplicateKey),
        }
        Ok(())
    }

    /// Removes key from the associated keys set.
    /// Returns true if value was found in the set prior to the removal, false
    /// otherwise.
    pub fn remove_key(&mut self, key: &AccountHash) -> Result<(), RemoveKeyFailure> {
        self.0
            .remove(key)
            .map(|_| ())
            .ok_or(RemoveKeyFailure::MissingKey)
    }

    /// Adds new AssociatedKey to the set.
    /// Returns true if added successfully, false otherwise.
    pub fn update_key(&mut self, key: AccountHash, weight: Weight) -> Result<(), UpdateKeyFailure> {
        match self.0.entry(key) {
            Entry::Vacant(_) => {
                return Err(UpdateKeyFailure::MissingKey);
            }
            Entry::Occupied(mut entry) => {
                *entry.get_mut() = weight;
            }
        }
        Ok(())
    }

    /// Returns the weight of an account hash.
    pub fn get(&self, key: &AccountHash) -> Option<&Weight> {
        self.0.get(key)
    }

    /// Returns `true` if a given key exists.
    pub fn contains_key(&self, key: &AccountHash) -> bool {
        self.0.contains_key(key)
    }

    /// Returns an iterator over the account hash and the weights.
    pub fn iter(&self) -> impl Iterator<Item = (&AccountHash, &Weight)> {
        self.0.iter()
    }

    /// Returns the count of the associated keys.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the associated keys are empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Helper method that calculates weight for keys that comes from any
    /// source.
    ///
    /// This method is not concerned about uniqueness of the passed iterable.
    /// Uniqueness is determined based on the input collection properties,
    /// which is either BTreeSet (in [`AssociatedKeys::calculate_keys_weight`])
    /// or BTreeMap (in [`AssociatedKeys::total_keys_weight`]).
    fn calculate_any_keys_weight<'a>(&self, keys: impl Iterator<Item = &'a AccountHash>) -> Weight {
        let total = keys
            .filter_map(|key| self.0.get(key))
            .fold(0u8, |acc, w| acc.saturating_add(w.value()));

        Weight::new(total)
    }

    /// Calculates total weight of authorization keys provided by an argument
    pub fn calculate_keys_weight(&self, authorization_keys: &BTreeSet<AccountHash>) -> Weight {
        self.calculate_any_keys_weight(authorization_keys.iter())
    }

    /// Calculates total weight of all authorization keys
    pub fn total_keys_weight(&self) -> Weight {
        self.calculate_any_keys_weight(self.0.keys())
    }

    /// Calculates total weight of all authorization keys excluding a given key
    pub fn total_keys_weight_excluding(&self, account_hash: AccountHash) -> Weight {
        self.calculate_any_keys_weight(self.0.keys().filter(|&&element| element != account_hash))
    }
}

impl From<BTreeMap<AccountHash, Weight>> for AssociatedKeys {
    fn from(associated_keys: BTreeMap<AccountHash, Weight>) -> Self {
        Self(associated_keys)
    }
}

impl From<AssociatedKeys> for BTreeMap<AccountHash, Weight> {
    fn from(associated_keys: AssociatedKeys) -> Self {
        associated_keys.0
    }
}

impl ToBytes for AssociatedKeys {
    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        self.0.to_bytes()
    }

    fn serialized_length(&self) -> usize {
        self.0.serialized_length()
    }

    fn write_bytes(&self, writer: &mut Vec<u8>) -> Result<(), bytesrepr::Error> {
        self.0.write_bytes(writer)
    }
}

impl FromBytes for AssociatedKeys {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        let (associated_keys, rem) = FromBytes::from_bytes(bytes)?;
        Ok((AssociatedKeys(associated_keys), rem))
    }
}

struct Labels;

impl KeyValueLabels for Labels {
    const KEY: &'static str = "account_hash";
    const VALUE: &'static str = "weight";
}

#[cfg(feature = "json-schema")]
impl KeyValueJsonSchema for Labels {
    const JSON_SCHEMA_KV_NAME: Option<&'static str> = Some("AssociatedKey");
    const JSON_SCHEMA_KV_DESCRIPTION: Option<&'static str> = Some("A weighted public key.");
    const JSON_SCHEMA_KEY_DESCRIPTION: Option<&'static str> =
        Some("The account hash of the public key.");
    const JSON_SCHEMA_VALUE_DESCRIPTION: Option<&'static str> =
        Some("The weight assigned to the public key.");
}

#[doc(hidden)]
#[cfg(any(feature = "testing", feature = "gens", test))]
pub mod gens {
    use proptest::prelude::*;

    use crate::gens::{account_hash_arb, account_weight_arb};

    use super::AssociatedKeys;

    pub fn account_associated_keys_arb() -> impl Strategy<Value = AssociatedKeys> {
        proptest::collection::btree_map(account_hash_arb(), account_weight_arb(), 10).prop_map(
            |keys| {
                let mut associated_keys = AssociatedKeys::default();
                keys.into_iter().for_each(|(k, v)| {
                    associated_keys.add_key(k, v).unwrap();
                });
                associated_keys
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, iter::FromIterator};

    use crate::{
        account::{AccountHash, Weight, ACCOUNT_HASH_LENGTH},
        bytesrepr,
    };

    use super::*;

    #[test]
    fn associated_keys_add() {
        let mut keys =
            AssociatedKeys::new(AccountHash::new([0u8; ACCOUNT_HASH_LENGTH]), Weight::new(1));
        let new_pk = AccountHash::new([1u8; ACCOUNT_HASH_LENGTH]);
        let new_pk_weight = Weight::new(2);
        assert!(keys.add_key(new_pk, new_pk_weight).is_ok());
        assert_eq!(keys.get(&new_pk), Some(&new_pk_weight))
    }

    #[test]
    fn associated_keys_add_duplicate() {
        let pk = AccountHash::new([0u8; ACCOUNT_HASH_LENGTH]);
        let weight = Weight::new(1);
        let mut keys = AssociatedKeys::new(pk, weight);
        assert_eq!(
            keys.add_key(pk, Weight::new(10)),
            Err(AddKeyFailure::DuplicateKey)
        );
        assert_eq!(keys.get(&pk), Some(&weight));
    }

    #[test]
    fn associated_keys_remove() {
        let pk = AccountHash::new([0u8; ACCOUNT_HASH_LENGTH]);
        let weight = Weight::new(1);
        let mut keys = AssociatedKeys::new(pk, weight);
        assert!(keys.remove_key(&pk).is_ok());
        assert!(keys
            .remove_key(&AccountHash::new([1u8; ACCOUNT_HASH_LENGTH]))
            .is_err());
    }

    #[test]
    fn associated_keys_update() {
        let pk1 = AccountHash::new([0u8; ACCOUNT_HASH_LENGTH]);
        let pk2 = AccountHash::new([1u8; ACCOUNT_HASH_LENGTH]);
        let weight = Weight::new(1);
        let mut keys = AssociatedKeys::new(pk1, weight);
        assert!(matches!(
            keys.update_key(pk2, Weight::new(2))
                .expect_err("should get error"),
            UpdateKeyFailure::MissingKey
        ));
        keys.add_key(pk2, Weight::new(1)).unwrap();
        assert_eq!(keys.get(&pk2), Some(&Weight::new(1)));
        keys.update_key(pk2, Weight::new(2)).unwrap();
        assert_eq!(keys.get(&pk2), Some(&Weight::new(2)));
    }

    #[test]
    fn associated_keys_calculate_keys_once() {
        let key_1 = AccountHash::new([0; 32]);
        let key_2 = AccountHash::new([1; 32]);
        let key_3 = AccountHash::new([2; 32]);
        let mut keys = AssociatedKeys::default();

        keys.add_key(key_2, Weight::new(2))
            .expect("should add key_1");
        keys.add_key(key_1, Weight::new(1))
            .expect("should add key_1");
        keys.add_key(key_3, Weight::new(3))
            .expect("should add key_1");

        assert_eq!(
            keys.calculate_keys_weight(&BTreeSet::from_iter(vec![
                key_1, key_2, key_3, key_1, key_2, key_3,
            ])),
            Weight::new(1 + 2 + 3)
        );
    }

    #[test]
    fn associated_keys_total_weight() {
        let associated_keys = {
            let mut res = AssociatedKeys::new(AccountHash::new([1u8; 32]), Weight::new(1));
            res.add_key(AccountHash::new([2u8; 32]), Weight::new(11))
                .expect("should add key 1");
            res.add_key(AccountHash::new([3u8; 32]), Weight::new(12))
                .expect("should add key 2");
            res.add_key(AccountHash::new([4u8; 32]), Weight::new(13))
                .expect("should add key 3");
            res
        };
        assert_eq!(
            associated_keys.total_keys_weight(),
            Weight::new(1 + 11 + 12 + 13)
        );
    }

    #[test]
    fn associated_keys_total_weight_excluding() {
        let identity_key = AccountHash::new([1u8; 32]);
        let identity_key_weight = Weight::new(1);

        let key_1 = AccountHash::new([2u8; 32]);
        let key_1_weight = Weight::new(11);

        let key_2 = AccountHash::new([3u8; 32]);
        let key_2_weight = Weight::new(12);

        let key_3 = AccountHash::new([4u8; 32]);
        let key_3_weight = Weight::new(13);

        let associated_keys = {
            let mut res = AssociatedKeys::new(identity_key, identity_key_weight);
            res.add_key(key_1, key_1_weight).expect("should add key 1");
            res.add_key(key_2, key_2_weight).expect("should add key 2");
            res.add_key(key_3, key_3_weight).expect("should add key 3");
            res
        };
        assert_eq!(
            associated_keys.total_keys_weight_excluding(key_2),
            Weight::new(identity_key_weight.value() + key_1_weight.value() + key_3_weight.value())
        );
    }

    #[test]
    fn overflowing_keys_weight() {
        let identity_key = AccountHash::new([1u8; 32]);
        let key_1 = AccountHash::new([2u8; 32]);
        let key_2 = AccountHash::new([3u8; 32]);
        let key_3 = AccountHash::new([4u8; 32]);

        let identity_key_weight = Weight::new(250);
        let weight_1 = Weight::new(1);
        let weight_2 = Weight::new(2);
        let weight_3 = Weight::new(3);

        let saturated_weight = Weight::new(u8::MAX);

        let associated_keys = {
            let mut res = AssociatedKeys::new(identity_key, identity_key_weight);

            res.add_key(key_1, weight_1).expect("should add key 1");
            res.add_key(key_2, weight_2).expect("should add key 2");
            res.add_key(key_3, weight_3).expect("should add key 3");
            res
        };

        assert_eq!(
            associated_keys.calculate_keys_weight(&BTreeSet::from_iter(vec![
                identity_key, // 250
                key_1,        // 251
                key_2,        // 253
                key_3,        // 256 - error
            ])),
            saturated_weight,
        );
    }

    #[test]
    fn serialization_roundtrip() {
        let mut keys = AssociatedKeys::default();
        keys.add_key(AccountHash::new([1; 32]), Weight::new(1))
            .unwrap();
        keys.add_key(AccountHash::new([2; 32]), Weight::new(2))
            .unwrap();
        keys.add_key(AccountHash::new([3; 32]), Weight::new(3))
            .unwrap();
        bytesrepr::test_serialization_roundtrip(&keys);
    }
}
