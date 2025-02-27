//! Support for storage costs.
#[cfg(feature = "datasize")]
use datasize::DataSize;
use derive_more::Add;
use num_traits::Zero;
#[cfg(any(feature = "testing", test))]
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use serde::{Deserialize, Serialize};

use crate::{
    bytesrepr::{self, FromBytes, ToBytes},
    Gas, U512,
};

/// Default gas cost per byte stored.
pub const DEFAULT_GAS_PER_BYTE_COST: u32 = 1_117_587;

/// Represents a cost table for storage costs.
#[derive(Add, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "datasize", derive(DataSize))]
#[serde(deny_unknown_fields)]
pub struct StorageCosts {
    /// Gas charged per byte stored in the global state.
    gas_per_byte: u32,
}

impl StorageCosts {
    /// Creates new `StorageCosts`.
    pub const fn new(gas_per_byte: u32) -> Self {
        Self { gas_per_byte }
    }

    /// Returns amount of gas per byte stored.
    pub fn gas_per_byte(&self) -> u32 {
        self.gas_per_byte
    }

    /// Calculates gas cost for storing `bytes`.
    pub fn calculate_gas_cost(&self, bytes: usize) -> Gas {
        let value = U512::from(self.gas_per_byte) * U512::from(bytes);
        Gas::new(value)
    }
}

impl Default for StorageCosts {
    fn default() -> Self {
        Self {
            gas_per_byte: DEFAULT_GAS_PER_BYTE_COST,
        }
    }
}

#[cfg(any(feature = "testing", test))]
impl Distribution<StorageCosts> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> StorageCosts {
        StorageCosts {
            gas_per_byte: rng.gen(),
        }
    }
}

impl ToBytes for StorageCosts {
    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        let mut ret = bytesrepr::unchecked_allocate_buffer(self);

        ret.append(&mut self.gas_per_byte.to_bytes()?);

        Ok(ret)
    }

    fn serialized_length(&self) -> usize {
        self.gas_per_byte.serialized_length()
    }
}

impl FromBytes for StorageCosts {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        let (gas_per_byte, rem) = FromBytes::from_bytes(bytes)?;

        Ok((StorageCosts { gas_per_byte }, rem))
    }
}

impl Zero for StorageCosts {
    fn zero() -> Self {
        StorageCosts { gas_per_byte: 0 }
    }

    fn is_zero(&self) -> bool {
        self.gas_per_byte.is_zero()
    }
}

#[cfg(test)]
pub mod tests {
    use crate::U512;

    use super::*;
    use proptest::prelude::*;

    const SMALL_WEIGHT: usize = 123456789;
    const LARGE_WEIGHT: usize = usize::MAX;

    #[test]
    fn should_calculate_gas_cost() {
        let storage_costs = StorageCosts::default();

        let cost = storage_costs.calculate_gas_cost(SMALL_WEIGHT);

        let expected_cost = U512::from(DEFAULT_GAS_PER_BYTE_COST) * U512::from(SMALL_WEIGHT);
        assert_eq!(cost, Gas::new(expected_cost));
    }

    #[test]
    fn should_calculate_big_gas_cost() {
        let storage_costs = StorageCosts::default();

        let cost = storage_costs.calculate_gas_cost(LARGE_WEIGHT);

        let expected_cost = U512::from(DEFAULT_GAS_PER_BYTE_COST) * U512::from(LARGE_WEIGHT);
        assert_eq!(cost, Gas::new(expected_cost));
    }

    proptest! {
        #[test]
        fn bytesrepr_roundtrip(storage_costs in super::gens::storage_costs_arb()) {
            bytesrepr::test_serialization_roundtrip(&storage_costs);
        }
    }
}

#[doc(hidden)]
#[cfg(test)]
pub mod gens {
    use crate::gens::example_u32_arb;

    use super::StorageCosts;
    use proptest::prelude::*;

    pub(super) fn storage_costs_arb() -> impl Strategy<Value = StorageCosts> {
        example_u32_arb().prop_map(StorageCosts::new)
    }
}
