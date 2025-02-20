pub(crate) mod detail;
/// Provides native mint processing.
mod mint_native;
/// Provides runtime logic for mint processing.
pub mod runtime_provider;
/// Provides storage logic for mint processing.
pub mod storage_provider;
/// Provides system logic for mint processing.
pub mod system_provider;

use num_rational::Ratio;
use num_traits::CheckedMul;

use casper_types::{
    account::AccountHash,
    system::{
        mint::{Error, ROUND_SEIGNIORAGE_RATE_KEY, TOTAL_SUPPLY_KEY},
        Caller,
    },
    Key, PublicKey, URef, U512,
};

use crate::system::mint::{
    runtime_provider::RuntimeProvider, storage_provider::StorageProvider,
    system_provider::SystemProvider,
};

/// Mint trait.
pub trait Mint: RuntimeProvider + StorageProvider + SystemProvider {
    /// Mint new token with given `initial_balance` balance. Returns new purse on success, otherwise
    /// an error.
    fn mint(&mut self, initial_balance: U512) -> Result<URef, Error> {
        let caller = self.get_caller();
        let is_empty_purse = initial_balance.is_zero();
        if !is_empty_purse && caller != PublicKey::System.to_account_hash() {
            return Err(Error::InvalidNonEmptyPurseCreation);
        }

        let purse_uref: URef = self.new_uref(())?;
        self.write_balance(purse_uref, initial_balance)?;

        if !is_empty_purse {
            // get total supply uref if exists, otherwise error
            let total_supply_uref = match self.get_key(TOTAL_SUPPLY_KEY) {
                None => {
                    // total supply URef should exist due to genesis
                    return Err(Error::TotalSupplyNotFound);
                }
                Some(Key::URef(uref)) => uref,
                Some(_) => return Err(Error::MissingKey),
            };
            // increase total supply
            self.add(total_supply_uref, initial_balance)?;
        }

        Ok(purse_uref)
    }

    /// Burns native tokens.
    fn burn(&mut self, purse: URef, amount: U512) -> Result<(), Error> {
        if !purse.is_writeable() {
            return Err(Error::InvalidAccessRights);
        }
        if !self.is_valid_uref(&purse) {
            return Err(Error::ForgedReference);
        }

        let source_available_balance: U512 = match self.balance(purse)? {
            Some(source_balance) => source_balance,
            None => return Err(Error::PurseNotFound),
        };

        let new_balance = source_available_balance
            .checked_sub(amount)
            .unwrap_or_else(U512::zero);
        // change balance
        self.write_balance(purse, new_balance)?;
        // reduce total supply AFTER changing balance in case changing balance errors
        let burned_amount = source_available_balance.saturating_sub(new_balance);
        detail::reduce_total_supply_unsafe(self, burned_amount)
    }

    /// Reduce total supply by `amount`. Returns unit on success, otherwise
    /// an error.
    fn reduce_total_supply(&mut self, amount: U512) -> Result<(), Error> {
        // only system may reduce total supply
        let caller = self.get_caller();
        if caller != PublicKey::System.to_account_hash() {
            return Err(Error::InvalidTotalSupplyReductionAttempt);
        }

        detail::reduce_total_supply_unsafe(self, amount)
    }

    /// Read balance of given `purse`.
    fn balance(&mut self, purse: URef) -> Result<Option<U512>, Error> {
        match self.available_balance(purse)? {
            some @ Some(_) => Ok(some),
            None => Err(Error::PurseNotFound),
        }
    }

    /// Transfers `amount` of tokens from `source` purse to a `target` purse.
    fn transfer(
        &mut self,
        maybe_to: Option<AccountHash>,
        source: URef,
        target: URef,
        amount: U512,
        id: Option<u64>,
    ) -> Result<(), Error> {
        if !self.allow_unrestricted_transfers() {
            let registry = self
                .get_system_entity_registry()
                .map_err(|_| Error::UnableToGetSystemRegistry)?;
            let immediate_caller = self.get_immediate_caller();
            match immediate_caller {
                Some(Caller::Entity { entity_addr, .. })
                    if registry.exists(&entity_addr.value()) =>
                {
                    // System contract calling a mint is fine (i.e. standard payment calling mint's
                    // transfer)
                }

                Some(Caller::Initiator { account_hash: _ })
                    if self.is_called_from_standard_payment() =>
                {
                    // Standard payment acts as a session without separate stack frame and calls
                    // into mint's transfer.
                }

                Some(Caller::Initiator { account_hash })
                    if account_hash == PublicKey::System.to_account_hash() =>
                {
                    // System calls a session code.
                }

                Some(Caller::Initiator { account_hash }) => {
                    // For example: a session using transfer host functions, or calling the mint's
                    // entrypoint directly
                    let is_source_admin = self.is_administrator(&account_hash);
                    match maybe_to {
                        Some(to) => {
                            let maybe_account = self.runtime_footprint_by_account_hash(to);

                            match maybe_account {
                                Ok(Some(runtime_footprint)) => {
                                    // This can happen when user tries to transfer funds by
                                    // calling mint
                                    // directly but tries to specify wrong account hash.
                                    let addr = if let Some(uref) = runtime_footprint.main_purse() {
                                        uref.addr()
                                    } else {
                                        return Err(Error::InvalidContext);
                                    };

                                    if addr != target.addr() {
                                        return Err(Error::DisabledUnrestrictedTransfers);
                                    }
                                    let is_target_system_account =
                                        to == PublicKey::System.to_account_hash();
                                    let is_target_administrator = self.is_administrator(&to);
                                    if !(is_source_admin
                                        || is_target_system_account
                                        || is_target_administrator)
                                    {
                                        return Err(Error::DisabledUnrestrictedTransfers);
                                    }
                                }
                                Ok(None) => {
                                    // `to` is specified, but no new account is persisted
                                    // yet. Only
                                    // administrators can do that and it is also validated
                                    // at the host function level.
                                    if !is_source_admin {
                                        return Err(Error::DisabledUnrestrictedTransfers);
                                    }
                                }
                                Err(_) => {
                                    return Err(Error::Storage);
                                }
                            }
                        }
                        None => {
                            if !is_source_admin {
                                return Err(Error::DisabledUnrestrictedTransfers);
                            }
                        }
                    }
                }

                Some(Caller::Entity {
                    package_hash: _,
                    entity_addr: _,
                }) => {
                    if self.get_caller() != PublicKey::System.to_account_hash()
                        && !self.is_administrator(&self.get_caller())
                    {
                        return Err(Error::DisabledUnrestrictedTransfers);
                    }
                }

                Some(Caller::SmartContract {
                    contract_package_hash: _,
                    contract_hash: _,
                }) => {
                    if self.get_caller() != PublicKey::System.to_account_hash()
                        && !self.is_administrator(&self.get_caller())
                    {
                        return Err(Error::DisabledUnrestrictedTransfers);
                    }
                }

                None => {
                    // There's always an immediate caller, but we should return something.
                    return Err(Error::DisabledUnrestrictedTransfers);
                }
            }
        }

        if !source.is_writeable() || !target.is_addable() {
            // TODO: I don't think we should enforce is addable on the target
            // Unlike other uses of URefs (such as a counter), in this context the value represents
            // a deposit of token. Generally, deposit of a desirable resource is permissive.
            return Err(Error::InvalidAccessRights);
        }
        let source_available_balance: U512 = match self.available_balance(source)? {
            Some(source_balance) => source_balance,
            None => return Err(Error::SourceNotFound),
        };
        if amount > source_available_balance {
            // NOTE: we use AVAILABLE balance to check sufficient funds
            return Err(Error::InsufficientFunds);
        }
        let source_total_balance = self.total_balance(source)?;
        if source_available_balance > source_total_balance {
            panic!("available balance can never be greater than total balance");
        }
        if self.available_balance(target)?.is_none() {
            return Err(Error::DestNotFound);
        }
        let addr = match self.get_main_purse() {
            None => return Err(Error::InvalidURef),
            Some(uref) => uref.addr(),
        };
        if self.get_caller() != PublicKey::System.to_account_hash() && addr == source.addr() {
            if amount > self.get_approved_spending_limit() {
                return Err(Error::UnapprovedSpendingAmount);
            }
            self.sub_approved_spending_limit(amount);
        }

        // NOTE: we use TOTAL balance to determine new balance
        let new_balance = source_total_balance.saturating_sub(amount);
        self.write_balance(source, new_balance)?;
        self.add_balance(target, amount)?;
        self.record_transfer(maybe_to, source, target, amount, id)?;
        Ok(())
    }

    /// Retrieves the base round reward.
    fn read_base_round_reward(&mut self) -> Result<U512, Error> {
        let total_supply_uref = match self.get_key(TOTAL_SUPPLY_KEY) {
            Some(Key::URef(uref)) => uref,
            Some(_) => return Err(Error::MissingKey),
            None => return Err(Error::MissingKey),
        };
        let total_supply: U512 = self
            .read(total_supply_uref)?
            .ok_or(Error::TotalSupplyNotFound)?;

        let round_seigniorage_rate_uref = match self.get_key(ROUND_SEIGNIORAGE_RATE_KEY) {
            Some(Key::URef(uref)) => uref,
            Some(_) => return Err(Error::MissingKey),
            None => return Err(Error::MissingKey),
        };
        let round_seigniorage_rate: Ratio<U512> = self
            .read(round_seigniorage_rate_uref)?
            .ok_or(Error::TotalSupplyNotFound)?;

        round_seigniorage_rate
            .checked_mul(&Ratio::from(total_supply))
            .map(|ratio| ratio.to_integer())
            .ok_or(Error::ArithmeticOverflow)
    }

    /// Mint `amount` new token into `existing_purse`.
    /// Returns unit on success, otherwise an error.
    fn mint_into_existing_purse(
        &mut self,
        existing_purse: URef,
        amount: U512,
    ) -> Result<(), Error> {
        let caller = self.get_caller();
        if caller != PublicKey::System.to_account_hash() {
            return Err(Error::InvalidContext);
        }
        if amount.is_zero() {
            // treat as noop
            return Ok(());
        }
        if !self.purse_exists(existing_purse)? {
            return Err(Error::PurseNotFound);
        }
        self.add_balance(existing_purse, amount)?;
        // get total supply uref if exists, otherwise error.
        let total_supply_uref = match self.get_key(TOTAL_SUPPLY_KEY) {
            None => {
                // total supply URef should exist due to genesis
                // which obviously must have been called
                // before new rewards are minted at the end of an era
                return Err(Error::TotalSupplyNotFound);
            }
            Some(Key::URef(uref)) => uref,
            Some(_) => return Err(Error::MissingKey),
        };
        // increase total supply
        self.add(total_supply_uref, amount)?;
        Ok(())
    }

    /// Check if a purse exists.
    fn purse_exists(&mut self, uref: URef) -> Result<bool, Error>;
}
