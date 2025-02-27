use std::iter;

use itertools::Itertools;
use thiserror::Error;

use crate::components::consensus::{
    highway_core::{highway::SignedWireUnit, state::Params},
    traits::Context,
    utils::{ValidatorIndex, Validators},
};

/// An error due to invalid evidence.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum EvidenceError {
    #[error("The sequence numbers in the equivocating units are different.")]
    EquivocationDifferentSeqNumbers,
    #[error("The creators in the equivocating units are different.")]
    EquivocationDifferentCreators,
    #[error("The units were created for a different instance ID.")]
    EquivocationInstanceId,
    #[error("The two units are equal.")]
    EquivocationSameUnit,
    #[error("The endorsements don't match the unit hashes.")]
    EndorsementWrongHash,
    #[error("The creators of the conflicting endorsements are different.")]
    EndorsementDifferentCreators,
    #[error("The swimlane is not a contiguous sequence of units.")]
    EndorsementInvalidSwimlane,
    #[error("Includes more units than allowed.")]
    EndorsementTooManyUnits,
    #[error("The perpetrator is not a validator.")]
    UnknownPerpetrator,
    #[error("The signature is invalid.")]
    Signature,
}

#[allow(clippy::arithmetic_side_effects)]
pub mod relaxed {
    // This module exists solely to exempt the `EnumDiscriminants` macro generated code from the
    // module-wide `clippy::arithmetic_side_effects` lint.

    use datasize::DataSize;
    use serde::{Deserialize, Serialize};
    use strum::EnumDiscriminants;

    use crate::components::consensus::{
        highway_core::{endorsement::SignedEndorsement, highway::SignedWireUnit},
        traits::Context,
    };

    /// Evidence that a validator is faulty.
    #[derive(
        Clone, DataSize, Debug, Eq, PartialEq, Serialize, Deserialize, Hash, EnumDiscriminants,
    )]
    #[serde(bound(
        serialize = "C::Hash: Serialize",
        deserialize = "C::Hash: Deserialize<'de>",
    ))]
    #[strum_discriminants(derive(strum::EnumIter))]
    pub enum Evidence<C>
    where
        C: Context,
    {
        /// The validator produced two units with the same sequence number.
        Equivocation(SignedWireUnit<C>, SignedWireUnit<C>),
        /// The validator endorsed two conflicting units.
        Endorsements {
            /// The endorsement for `unit1`.
            endorsement1: SignedEndorsement<C>,
            /// The unit with the lower (or equal) sequence number.
            unit1: SignedWireUnit<C>,
            /// The endorsement for `unit2`, by the same creator as endorsement1.
            endorsement2: SignedEndorsement<C>,
            /// The unit with the higher (or equal) sequence number, on a conflicting fork of the
            /// same creator as `unit1`.
            unit2: SignedWireUnit<C>,
            /// The predecessors of `unit2`, back to the same sequence number as `unit1`, in
            /// reverse chronological order.
            swimlane2: Vec<SignedWireUnit<C>>,
        },
    }
}
pub use relaxed::{Evidence, EvidenceDiscriminants};

impl<C: Context> Evidence<C> {
    /// Returns the ID of the faulty validator.
    pub fn perpetrator(&self) -> ValidatorIndex {
        match self {
            Evidence::Equivocation(unit1, _) => unit1.wire_unit().creator,
            Evidence::Endorsements { endorsement1, .. } => endorsement1.validator_idx(),
        }
    }

    /// Validates the evidence and returns `Ok(())` if it is valid.
    /// "Validation" can mean different things for different type of evidence.
    ///
    /// - For an equivocation, it checks whether the creators, sequence numbers and instance IDs of
    ///   the two units are the same.
    pub fn validate(
        &self,
        validators: &Validators<C::ValidatorId>,
        instance_id: &C::InstanceId,
        params: &Params,
    ) -> Result<(), EvidenceError> {
        match self {
            Evidence::Equivocation(unit1, unit2) => {
                Self::validate_equivocation(unit1, unit2, instance_id, validators)
            }
            Evidence::Endorsements {
                endorsement1,
                unit1,
                endorsement2,
                unit2,
                swimlane2,
            } => {
                if swimlane2.len() as u64 > params.endorsement_evidence_limit() {
                    return Err(EvidenceError::EndorsementTooManyUnits);
                }
                let v_id = validators
                    .id(endorsement1.validator_idx())
                    .ok_or(EvidenceError::UnknownPerpetrator)?;
                if *endorsement1.unit() != unit1.hash() || *endorsement2.unit() != unit2.hash() {
                    return Err(EvidenceError::EndorsementWrongHash);
                }
                if endorsement1.validator_idx() != endorsement2.validator_idx() {
                    return Err(EvidenceError::EndorsementDifferentCreators);
                }
                for (unit, pred) in iter::once(unit2).chain(swimlane2).tuple_windows() {
                    if unit.wire_unit().previous() != Some(&pred.hash()) {
                        return Err(EvidenceError::EndorsementInvalidSwimlane);
                    }
                }
                Self::validate_equivocation(
                    unit1,
                    swimlane2.last().unwrap_or(unit2),
                    instance_id,
                    validators,
                )?;
                if !C::verify_signature(&endorsement1.hash(), v_id, endorsement1.signature())
                    || !C::verify_signature(&endorsement2.hash(), v_id, endorsement2.signature())
                {
                    return Err(EvidenceError::Signature);
                }
                Ok(())
            }
        }
    }

    fn validate_equivocation(
        unit1: &SignedWireUnit<C>,
        unit2: &SignedWireUnit<C>,
        instance_id: &C::InstanceId,
        validators: &Validators<C::ValidatorId>,
    ) -> Result<(), EvidenceError> {
        let wunit1 = unit1.wire_unit();
        let wunit2 = unit2.wire_unit();
        let v_id = validators
            .id(wunit1.creator)
            .ok_or(EvidenceError::UnknownPerpetrator)?;
        if wunit1.creator != wunit2.creator {
            return Err(EvidenceError::EquivocationDifferentCreators);
        }
        if wunit1.seq_number != wunit2.seq_number {
            return Err(EvidenceError::EquivocationDifferentSeqNumbers);
        }
        if wunit1.instance_id != *instance_id || wunit2.instance_id != *instance_id {
            return Err(EvidenceError::EquivocationInstanceId);
        }
        if unit1 == unit2 {
            return Err(EvidenceError::EquivocationSameUnit);
        }
        if !C::verify_signature(&unit1.hash(), v_id, &unit1.signature)
            || !C::verify_signature(&unit2.hash(), v_id, &unit2.signature)
        {
            return Err(EvidenceError::Signature);
        }
        Ok(())
    }
}

mod specimen_support {

    use crate::{
        components::consensus::ClContext,
        utils::specimen::{
            estimator_max_rounds_per_era, largest_variant, vec_of_largest_specimen, Cache,
            LargestSpecimen, SizeEstimator,
        },
    };

    use super::{Evidence, EvidenceDiscriminants};

    impl LargestSpecimen for Evidence<ClContext> {
        fn largest_specimen<E: SizeEstimator>(estimator: &E, cache: &mut Cache) -> Self {
            largest_variant::<Self, EvidenceDiscriminants, _, _>(estimator, |variant| match variant
            {
                EvidenceDiscriminants::Equivocation => Evidence::Equivocation(
                    LargestSpecimen::largest_specimen(estimator, cache),
                    LargestSpecimen::largest_specimen(estimator, cache),
                ),
                EvidenceDiscriminants::Endorsements => {
                    if estimator.parameter_bool("endorsements_enabled") {
                        Evidence::Endorsements {
                            endorsement1: LargestSpecimen::largest_specimen(estimator, cache),
                            unit1: LargestSpecimen::largest_specimen(estimator, cache),
                            endorsement2: LargestSpecimen::largest_specimen(estimator, cache),
                            unit2: LargestSpecimen::largest_specimen(estimator, cache),
                            swimlane2: vec_of_largest_specimen(
                                estimator,
                                estimator_max_rounds_per_era(estimator),
                                cache,
                            ),
                        }
                    } else {
                        Evidence::Equivocation(
                            LargestSpecimen::largest_specimen(estimator, cache),
                            LargestSpecimen::largest_specimen(estimator, cache),
                        )
                    }
                }
            })
        }
    }
}
