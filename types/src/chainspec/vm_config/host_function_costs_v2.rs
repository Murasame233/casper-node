//! Support for host function gas cost tables.
use core::ops::Add;

#[cfg(feature = "datasize")]
use datasize::DataSize;
use derive_more::Add;
use num_traits::Zero;
use rand::{distributions::Standard, prelude::Distribution, Rng};
use serde::{Deserialize, Serialize};

use crate::{
    bytesrepr::{self, FromBytes, ToBytes, U32_SERIALIZED_LENGTH},
    Gas,
};

use super::HostFunction;

/// Representation of argument's cost.
pub type Cost = u32;

const COST_SERIALIZED_LENGTH: usize = U32_SERIALIZED_LENGTH;

/// An identifier that represents an unused argument.
const NOT_USED: Cost = 0;

/// An arbitrary default fixed cost for host functions that were not researched yet.
const DEFAULT_FIXED_COST: Cost = 200;

const DEFAULT_ADD_COST: u32 = 5_800;
const DEFAULT_ADD_ASSOCIATED_KEY_COST: u32 = 1_200_000;

const DEFAULT_CALL_COST: u32 = 300_000_000;

const DEFAULT_ENV_TRANSFERRED_VALUE_COST: u32 = 2_500_000_000;
const DEFAULT_ENV_BALANCE_COST: u32 = 3_000_000;
const DEFAULT_ENV_BLOCK_TIME_COST: u32 = 330;
const DEFAULT_GET_CALLER_COST: u32 = 380;
const DEFAULT_GET_KEY_COST: u32 = 2_000;
const DEFAULT_GET_KEY_NAME_SIZE_WEIGHT: u32 = 440;
const DEFAULT_GET_MAIN_PURSE_COST: u32 = 1_300;
const DEFAULT_GET_PHASE_COST: u32 = 710;
const DEFAULT_GET_SYSTEM_CONTRACT_COST: u32 = 1_100;
const DEFAULT_HAS_KEY_COST: u32 = 1_500;
const DEFAULT_HAS_KEY_NAME_SIZE_WEIGHT: u32 = 840;
const DEFAULT_IS_VALID_UREF_COST: u32 = 760;
const DEFAULT_LOAD_NAMED_KEYS_COST: u32 = 42_000;
const DEFAULT_NEW_UREF_COST: u32 = 17_000;
const DEFAULT_NEW_UREF_VALUE_SIZE_WEIGHT: u32 = 590;

const DEFAULT_PRINT_COST: u32 = 20_000;
const DEFAULT_PRINT_TEXT_SIZE_WEIGHT: u32 = 4_600;

const DEFAULT_PUT_KEY_COST: u32 = 100_000_000;
const DEFAULT_PUT_KEY_NAME_SIZE_WEIGHT: u32 = 120_000;

const DEFAULT_READ_HOST_BUFFER_COST: u32 = 3_500;
const DEFAULT_READ_HOST_BUFFER_DEST_SIZE_WEIGHT: u32 = 310;

const DEFAULT_READ_VALUE_COST: u32 = 60_000;
const DEFAULT_DICTIONARY_GET_COST: u32 = 5_500;
const DEFAULT_DICTIONARY_GET_KEY_SIZE_WEIGHT: u32 = 590;

const DEFAULT_REMOVE_ASSOCIATED_KEY_COST: u32 = 4_200;

const DEFAULT_REMOVE_KEY_COST: u32 = 61_000;
const DEFAULT_REMOVE_KEY_NAME_SIZE_WEIGHT: u32 = 3_200;

const DEFAULT_RET_COST: u32 = 23_000;
const DEFAULT_RET_VALUE_SIZE_WEIGHT: u32 = 420_000;

const DEFAULT_REVERT_COST: u32 = 500;
const DEFAULT_SET_ACTION_THRESHOLD_COST: u32 = 74_000;
const DEFAULT_TRANSFER_FROM_PURSE_TO_ACCOUNT_COST: u32 = 2_500_000_000;
const DEFAULT_TRANSFER_FROM_PURSE_TO_PURSE_COST: u32 = 82_000_000;
const DEFAULT_TRANSFER_COST: u32 = 2_500_000_000;
const DEFAULT_UPDATE_ASSOCIATED_KEY_COST: u32 = 4_200;

const DEFAULT_WRITE_COST: u32 = 14_000;
const DEFAULT_WRITE_VALUE_SIZE_WEIGHT: u32 = 980;

const DEFAULT_ARG_CHARGE: u32 = 120_000;

const DEFAULT_DICTIONARY_PUT_COST: u32 = 9_500;
const DEFAULT_DICTIONARY_PUT_KEY_BYTES_SIZE_WEIGHT: u32 = 1_800;
const DEFAULT_DICTIONARY_PUT_VALUE_SIZE_WEIGHT: u32 = 520;

const DEFAULT_COPY_INPUT_COST: u32 = 0;
const DEFAULT_COPY_INPUT_VALUE_SIZE_WEIGHT: u32 = 0;

const DEFAULT_CREATE_COST: u32 = 0;
const DEFAULT_CREATE_CODE_SIZE_WEIGHT: u32 = 0;
const DEFAULT_CREATE_ENTRYPOINT_SIZE_WEIGHT: u32 = 0;
const DEFAULT_CREATE_INPUT_SIZE_WEIGHT: u32 = 0;
const DEFAULT_CREATE_SEED_SIZE_WEIGHT: u32 = 0;

/// Default cost for a new dictionary.
pub const DEFAULT_NEW_DICTIONARY_COST: u32 = DEFAULT_NEW_UREF_COST;

/// Host function cost unit for a new dictionary.
#[allow(unused)]
pub const DEFAULT_HOST_FUNCTION_NEW_DICTIONARY: HostFunction<[Cost; 1]> =
    HostFunction::new(DEFAULT_NEW_DICTIONARY_COST, [NOT_USED]);
const DEFAULT_BLAKE2B_COST: u32 = 1_200_000;

/// Default value that the cost of calling `casper_emit_message` increases by for every new message
/// emitted within an execution.
pub const DEFAULT_COST_INCREASE_PER_MESSAGE_EMITTED: u32 = 50;

const DEFAULT_MESSAGE_TOPIC_NAME_SIZE_WEIGHT: u32 = 30_000;
const DEFAULT_MESSAGE_PAYLOAD_SIZE_WEIGHT: u32 = 120_000;

const DEFAULT_CONTRACT_VERSION_ARG_SIZE_WEIGHT: u32 = 120_000;

const DEFAULT_GENERIC_HASH_COST: u32 = 1_200_000;

const DEFAULT_GENERIC_HASH_INPUT_COST: u32 = 120_000;

const DEFAULT_RECOVER_SECP256K1_COST: u32 = 1_300_000;
const DEFAULT_RECOVER_SECP256K1_SIZE_WEIGHT: u32 = 120_000;

const DEFAULT_VERIFY_SIGNATURE_COST: u32 = 1_300_000;
const DEFAULT_VERIFY_SIGNATURE_SIZE_WEIGHT: u32 = 120_000;

/// Definition of a host function cost table.
#[derive(Add, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "datasize", derive(DataSize))]
#[serde(deny_unknown_fields)]
pub struct HostFunctionCostsV2 {
    // /// Cost increase for successive calls to `casper_emit_message` within an execution.
    // pub cost_increase_per_message: u32,
    /// Cost of calling the `read` host function.
    pub read: HostFunction<[Cost; 6]>,
    // /// Cost of calling the `dictionary_get` host function.
    // pub dictionary_get: HostFunction<[Cost; 3]>,
    /// Cost of calling the `write` host function.
    pub write: HostFunction<[Cost; 5]>,
    /// Cost of calling the `copy_input` host function.
    pub copy_input: HostFunction<[Cost; 2]>,
    // /// Cost of calling the `dictionary_put` host function.
    // pub dictionary_put: HostFunction<[Cost; 4]>,
    // /// Cost of calling the `add` host function.
    // pub add: HostFunction<[Cost; 4]>,
    // /// Cost of calling the `new_uref` host function.
    // pub new_uref: HostFunction<[Cost; 3]>,
    // /// Cost of calling the `load_named_keys` host function.
    // pub load_named_keys: HostFunction<[Cost; 2]>,
    /// Cost of calling the `ret` host function.
    pub ret: HostFunction<[Cost; 2]>,
    pub create: HostFunction<[Cost; 10]>,
    // /// Cost of calling the `get_key` host function.
    // pub get_key: HostFunction<[Cost; 5]>,
    // /// Cost of calling the `has_key` host function.
    // pub has_key: HostFunction<[Cost; 2]>,
    // /// Cost of calling the `put_key` host function.
    // pub put_key: HostFunction<[Cost; 4]>,
    // /// Cost of calling the `remove_key` host function.
    // pub remove_key: HostFunction<[Cost; 2]>,
    // /// Cost of calling the `revert` host function.
    // pub revert: HostFunction<[Cost; 1]>,
    // /// Cost of calling the `is_valid_uref` host function.
    // pub is_valid_uref: HostFunction<[Cost; 2]>,
    // /// Cost of calling the `add_associated_key` host function.
    // pub add_associated_key: HostFunction<[Cost; 3]>,
    // /// Cost of calling the `remove_associated_key` host function.
    // pub remove_associated_key: HostFunction<[Cost; 2]>,
    // /// Cost of calling the `update_associated_key` host function.
    // pub update_associated_key: HostFunction<[Cost; 3]>,
    // /// Cost of calling the `set_action_threshold` host function.
    // pub set_action_threshold: HostFunction<[Cost; 2]>,
    /// Cost of calling the `env_caller` host function.
    pub env_caller: HostFunction<[Cost; 3]>,
    /// Cost of calling the `env_block_time` host function.
    pub env_block_time: HostFunction<[Cost; 0]>,
    /// Cost of calling the `env_transferred_value` host function.
    pub env_transferred_value: HostFunction<[Cost; 1]>,
    /// Cost of calling the `transfer` host function.
    pub transfer: HostFunction<[Cost; 3]>,
    // /// Cost of calling the `transfer_from_purse_to_account` host function.
    // pub transfer_from_purse_to_account: HostFunction<[Cost; 9]>,
    // /// Cost of calling the `transfer_from_purse_to_purse` host function.
    // pub transfer_from_purse_to_purse: HostFunction<[Cost; 8]>,
    /// Cost of calling the `env_balance` host function.
    pub env_balance: HostFunction<[Cost; 4]>,
    // /// Cost of calling the `get_phase` host function.
    // pub get_phase: HostFunction<[Cost; 1]>,
    // /// Cost of calling the `get_system_contract` host function.
    // pub get_system_contract: HostFunction<[Cost; 3]>,
    // /// Cost of calling the `get_main_purse` host function.
    // pub get_main_purse: HostFunction<[Cost; 1]>,
    // /// Cost of calling the `read_host_buffer` host function.
    // pub read_host_buffer: HostFunction<[Cost; 3]>,
    // /// Cost of calling the `create_contract_package_at_hash` host function.
    // pub create_contract_package_at_hash: HostFunction<[Cost; 2]>,
    // /// Cost of calling the `create_contract_user_group` host function.
    // pub create_contract_user_group: HostFunction<[Cost; 8]>,
    /// Cost of calling the `upgrade` host function.
    pub upgrade: HostFunction<[Cost; 6]>,
    // /// Cost of calling the `upgrade_with_message_topics` host function.
    // pub upgrade_with_message_topics: HostFunction<[Cost; 11]>,
    // /// Cost of calling the `add_package_version` host function.
    // pub add_package_version: HostFunction<[Cost; 11]>,
    // /// Cost of calling the `disable_contract_version` host function.
    // pub disable_contract_version: HostFunction<[Cost; 4]>,
    /// Cost of calling the `call` host function.
    pub call: HostFunction<[Cost; 9]>,
    // /// Cost of calling the `call_versioned_contract` host function.
    // pub call_versioned_contract: HostFunction<[Cost; 9]>,
    // /// Cost of calling the `get_named_arg_size` host function.
    // pub get_named_arg_size: HostFunction<[Cost; 3]>,
    // /// Cost of calling the `get_named_arg` host function.
    // pub get_named_arg: HostFunction<[Cost; 4]>,
    // /// Cost of calling the `remove_contract_user_group` host function.
    // pub remove_contract_user_group: HostFunction<[Cost; 4]>,
    // /// Cost of calling the `provision_contract_user_group_uref` host function.
    // pub provision_contract_user_group_uref: HostFunction<[Cost; 5]>,
    // /// Cost of calling the `remove_contract_user_group_urefs` host function.
    // pub remove_contract_user_group_urefs: HostFunction<[Cost; 6]>,
    // /// Cost of calling the `print` host function.
    pub print: HostFunction<[Cost; 2]>,
    // /// Cost of calling the `blake2b` host function.
    // pub blake2b: HostFunction<[Cost; 4]>,
    // /// Cost of calling the `next address` host function.
    // pub random_bytes: HostFunction<[Cost; 2]>,
    // /// Cost of calling the `enable_contract_version` host function.
    // pub enable_contract_version: HostFunction<[Cost; 4]>,
    // /// Cost of calling the `casper_manage_message_topic` host function.
    // pub manage_message_topic: HostFunction<[Cost; 4]>,
    // /// Cost of calling the `casper_emit_message` host function.
    // pub emit_message: HostFunction<[Cost; 4]>,
    // /// Cost of calling the `get_block_info` host function.
    // pub get_block_info: HostFunction<[Cost; 2]>,
    // /// Cost of calling the `generic_hash` host function.
    // pub generic_hash: HostFunction<[Cost; 5]>,
    // /// Cost of calling the 'recover_secp256k1' host function.
    // pub recover_secp256k1: HostFunction<[Cost; 6]>,
    // /// Cost of calling the 'recover_secp256k1' host function.
    // pub verify_signature: HostFunction<[Cost; 6]>,
}

impl Zero for HostFunctionCostsV2 {
    fn zero() -> Self {
        Self {
            read: HostFunction::zero(),
            // dictionary_get: HostFunction::zero(),
            write: HostFunction::zero(),
            copy_input: HostFunction::zero(),
            // dictionary_put: HostFunction::zero(),
            // add: HostFunction::zero(),
            // new_uref: HostFunction::zero(),
            // load_named_keys: HostFunction::zero(),
            ret: HostFunction::zero(),
            create: HostFunction::zero(),
            // get_key: HostFunction::zero(),
            // has_key: HostFunction::zero(),
            // put_key: HostFunction::zero(),
            // remove_key: HostFunction::zero(),
            // revert: HostFunction::zero(),
            // is_valid_uref: HostFunction::zero(),
            // add_associated_key: HostFunction::zero(),
            // remove_associated_key: HostFunction::zero(),
            // update_associated_key: HostFunction::zero(),
            // set_action_threshold: HostFunction::zero(),
            env_caller: HostFunction::zero(),
            env_block_time: HostFunction::zero(),
            env_transferred_value: HostFunction::zero(),
            transfer: HostFunction::zero(),
            // transfer_from_purse_to_account: HostFunction::zero(),
            // transfer_from_purse_to_purse: HostFunction::zero(),
            env_balance: HostFunction::zero(),
            // get_phase: HostFunction::zero(),
            // get_system_contract: HostFunction::zero(),
            // get_main_purse: HostFunction::zero(),
            // read_host_buffer: HostFunction::zero(),
            // create_contract_package_at_hash: HostFunction::zero(),
            // create_contract_user_group: HostFunction::zero(),
            // upgrade_with_message_topics: HostFunction::zero(),
            upgrade: HostFunction::zero(),
            // add_package_version: HostFunction::zero(),
            // disable_contract_version: HostFunction::zero(),
            call: HostFunction::zero(),
            // call_versioned_contract: HostFunction::zero(),
            // get_named_arg_size: HostFunction::zero(),
            // get_named_arg: HostFunction::zero(),
            // remove_contract_user_group: HostFunction::zero(),
            // provision_contract_user_group_uref: HostFunction::zero(),
            // remove_contract_user_group_urefs: HostFunction::zero(),
            print: HostFunction::zero(),
            // blake2b: HostFunction::zero(),
            // random_bytes: HostFunction::zero(),
            // enable_contract_version: HostFunction::zero(),
            // manage_message_topic: HostFunction::zero(),
            // emit_message: HostFunction::zero(),
            // cost_increase_per_message: Zero::zero(),
            // get_block_info: HostFunction::zero(),
            // generic_hash: HostFunction::zero(),
            // recover_secp256k1: HostFunction::zero(),
            // verify_signature: HostFunction::zero(),
        }
    }

    fn is_zero(&self) -> bool {
        let HostFunctionCostsV2 {
            // cost_increase_per_message,
            read,
            // dictionary_get,
            write,
            copy_input,
            // dictionary_put,
            // add,
            // new_uref,
            // load_named_keys,
            ret,
            create,
            // get_key,
            // has_key,
            // put_key,
            // remove_key,
            // revert,
            // is_valid_uref,
            // add_associated_key,
            // remove_associated_key,
            // update_associated_key,
            // set_action_threshold,
            env_caller,
            env_block_time,
            env_transferred_value,
            transfer,
            // transfer_from_purse_to_account,
            // transfer_from_purse_to_purse,
            env_balance,
            // get_phase,
            // get_system_contract,
            // get_main_purse,
            // read_host_buffer,
            // create_contract_package_at_hash,
            // create_contract_user_group,
            // upgrade_with_message_topics,
            upgrade,
            // add_package_version,
            // disable_contract_version,
            call,
            // call_versioned_contract,
            // get_named_arg_size,
            // get_named_arg,
            // remove_contract_user_group,
            // provision_contract_user_group_uref,
            // remove_contract_user_group_urefs,
            print,
            // blake2b,
            // random_bytes,
            // enable_contract_version,
            // manage_message_topic,
            // emit_message,
            // get_block_info,
            // generic_hash,
            // recover_secp256k1,
            // verify_signature,
        } = self;
        read.is_zero()
            // && dictionary_get.is_zero()
            && write.is_zero()
            && copy_input.is_zero()
            && ret.is_zero()
            && create.is_zero()
            // && dictionary_put.is_zero()
            // && add.is_zero()
            // && new_uref.is_zero()
            // && load_named_keys.is_zero()
            // && ret.is_zero()
            // && get_key.is_zero()
            // && has_key.is_zero()
            // && put_key.is_zero()
            // && remove_key.is_zero()
            // && revert.is_zero()
            // && is_valid_uref.is_zero()
            // && add_associated_key.is_zero()
            // && remove_associated_key.is_zero()
            // && update_associated_key.is_zero()
            // && set_action_threshold.is_zero()
            && env_caller.is_zero()
            && env_block_time.is_zero()
            && env_transferred_value.is_zero()
            && transfer.is_zero()
            // && transfer_from_purse_to_account.is_zero()
            // && transfer_from_purse_to_purse.is_zero()
            && env_balance.is_zero()
            // && get_phase.is_zero()
            // && get_system_contract.is_zero()
            // && get_main_purse.is_zero()
            // && read_host_buffer.is_zero()
            // && create_contract_package_at_hash.is_zero()
            // && create_contract_user_group.is_zero()
            && upgrade.is_zero()
            // && disable_contract_version.is_zero()
            && call.is_zero()
            // && call_versioned_contract.is_zero()
            // && get_named_arg_size.is_zero()
            // && get_named_arg.is_zero()
            // && remove_contract_user_group.is_zero()
            // && provision_contract_user_group_uref.is_zero()
            // && remove_contract_user_group_urefs.is_zero()
            && print.is_zero()
            // && blake2b.is_zero()
            // && random_bytes.is_zero()
            // && enable_contract_version.is_zero()
            // && manage_message_topic.is_zero()
            // && emit_message.is_zero()
            // && cost_increase_per_message.is_zero()
            // && add_package_version.is_zero()
            // && get_block_info.is_zero()
            // && upgrade_with_message_topics.is_zero()
            // && generic_hash.is_zero()
            // && recover_secp256k1.is_zero()
            // && verify_signature.is_zero()
    }
}

impl Default for HostFunctionCostsV2 {
    fn default() -> Self {
        Self {
            read: HostFunction::new(
                DEFAULT_READ_VALUE_COST,
                [
                    NOT_USED,
                    DEFAULT_ARG_CHARGE,
                    NOT_USED,
                    NOT_USED,
                    NOT_USED,
                    NOT_USED,
                ],
            ),
            // dictionary_get: HostFunction::new(
            //     DEFAULT_DICTIONARY_GET_COST,
            //     [NOT_USED, DEFAULT_DICTIONARY_GET_KEY_SIZE_WEIGHT, NOT_USED],
            // ),
            write: HostFunction::new(
                DEFAULT_WRITE_COST,
                [
                    NOT_USED,
                    NOT_USED,
                    NOT_USED,
                    DEFAULT_WRITE_VALUE_SIZE_WEIGHT,
                    NOT_USED,
                ],
            ),
            copy_input: HostFunction::new(
                DEFAULT_COPY_INPUT_COST,
                [
                    NOT_USED,
                    DEFAULT_COPY_INPUT_VALUE_SIZE_WEIGHT
                ],
            ),
            // dictionary_put: HostFunction::new(
            //     DEFAULT_DICTIONARY_PUT_COST,
            //     [
            //         NOT_USED,
            //         DEFAULT_DICTIONARY_PUT_KEY_BYTES_SIZE_WEIGHT,
            //         NOT_USED,
            //         DEFAULT_DICTIONARY_PUT_VALUE_SIZE_WEIGHT,
            //     ],
            // ),
            // add: HostFunction::fixed(DEFAULT_ADD_COST),
            // new_uref: HostFunction::new(
            //     DEFAULT_NEW_UREF_COST,
            //     [NOT_USED, NOT_USED, DEFAULT_NEW_UREF_VALUE_SIZE_WEIGHT],
            // ),
            // load_named_keys: HostFunction::fixed(DEFAULT_LOAD_NAMED_KEYS_COST),
            ret: HostFunction::new(DEFAULT_RET_COST, [NOT_USED, DEFAULT_RET_VALUE_SIZE_WEIGHT]),
            create: HostFunction::new(DEFAULT_CREATE_COST, [
                NOT_USED,
                DEFAULT_CREATE_CODE_SIZE_WEIGHT,
                NOT_USED,
                NOT_USED,
                DEFAULT_CREATE_ENTRYPOINT_SIZE_WEIGHT,
                NOT_USED,
                DEFAULT_CREATE_INPUT_SIZE_WEIGHT,
                NOT_USED,
                DEFAULT_CREATE_SEED_SIZE_WEIGHT,
                NOT_USED
            ]),
            // get_key: HostFunction::new(
            //     DEFAULT_GET_KEY_COST,
            //     [
            //         NOT_USED,
            //         DEFAULT_GET_KEY_NAME_SIZE_WEIGHT,
            //         NOT_USED,
            //         NOT_USED,
            //         NOT_USED,
            //     ],
            // ),
            // has_key: HostFunction::new(
            //     DEFAULT_HAS_KEY_COST,
            //     [NOT_USED, DEFAULT_HAS_KEY_NAME_SIZE_WEIGHT],
            // ),
            // put_key: HostFunction::new(
            //     DEFAULT_PUT_KEY_COST,
            //     [
            //         NOT_USED,
            //         DEFAULT_PUT_KEY_NAME_SIZE_WEIGHT,
            //         NOT_USED,
            //         DEFAULT_ARG_CHARGE,
            //     ],
            // ),
            // remove_key: HostFunction::new(
            //     DEFAULT_REMOVE_KEY_COST,
            //     [NOT_USED, DEFAULT_REMOVE_KEY_NAME_SIZE_WEIGHT],
            // ),
            // revert: HostFunction::fixed(DEFAULT_REVERT_COST),
            // is_valid_uref: HostFunction::fixed(DEFAULT_IS_VALID_UREF_COST),
            // add_associated_key: HostFunction::fixed(DEFAULT_ADD_ASSOCIATED_KEY_COST),
            // remove_associated_key: HostFunction::fixed(DEFAULT_REMOVE_ASSOCIATED_KEY_COST),
            // update_associated_key: HostFunction::fixed(DEFAULT_UPDATE_ASSOCIATED_KEY_COST),
            // set_action_threshold: HostFunction::fixed(DEFAULT_SET_ACTION_THRESHOLD_COST),
            env_caller: HostFunction::new(DEFAULT_GET_CALLER_COST, [
                NOT_USED,
                NOT_USED,
                NOT_USED,
            ]),
            env_block_time: HostFunction::fixed(DEFAULT_ENV_BLOCK_TIME_COST),
            env_transferred_value: HostFunction::fixed(DEFAULT_ENV_TRANSFERRED_VALUE_COST),
            transfer: HostFunction::new(DEFAULT_TRANSFER_COST, [
                NOT_USED,
                NOT_USED,
                NOT_USED,
            ]),
            // transfer_from_purse_to_account: HostFunction::fixed(
            //     DEFAULT_TRANSFER_FROM_PURSE_TO_ACCOUNT_COST,
            // ),
            // transfer_from_purse_to_purse: HostFunction::fixed(
            //     DEFAULT_TRANSFER_FROM_PURSE_TO_PURSE_COST,
            // ),
            env_balance: HostFunction::fixed(DEFAULT_ENV_BALANCE_COST),
            // get_phase: HostFunction::fixed(DEFAULT_GET_PHASE_COST),
            // get_system_contract: HostFunction::fixed(DEFAULT_GET_SYSTEM_CONTRACT_COST),
            // get_main_purse: HostFunction::fixed(DEFAULT_GET_MAIN_PURSE_COST),
            // read_host_buffer: HostFunction::new(
            //     DEFAULT_READ_HOST_BUFFER_COST,
            //     [
            //         NOT_USED,
            //         DEFAULT_READ_HOST_BUFFER_DEST_SIZE_WEIGHT,
            //         NOT_USED,
            //     ],
            // ),
            // create_contract_package_at_hash: HostFunction::default(),
            // create_contract_user_group: HostFunction::default(),
            // add_package_version: HostFunction::new(
            //     DEFAULT_FIXED_COST,
            //     [
            //         NOT_USED,
            //         NOT_USED,
            //         NOT_USED,
            //         NOT_USED,
            //         DEFAULT_CONTRACT_VERSION_ARG_SIZE_WEIGHT,
            //         NOT_USED,
            //         NOT_USED,
            //         NOT_USED,
            //         DEFAULT_MESSAGE_TOPIC_NAME_SIZE_WEIGHT,
            //         NOT_USED,
            //         NOT_USED,
            //     ],
            // ),
            upgrade: HostFunction::new(
                DEFAULT_FIXED_COST,
                [
                    NOT_USED,
                    NOT_USED,
                    NOT_USED,
                    NOT_USED,
                    NOT_USED,
                    NOT_USED,
                ],
            ),
            // upgrade_with_message_topics: HostFunction::new(
            //     DEFAULT_FIXED_COST,
            //     [
            //         NOT_USED,
            //         NOT_USED,
            //         NOT_USED,
            //         NOT_USED,
            //         DEFAULT_CONTRACT_VERSION_ARG_SIZE_WEIGHT,
            //         NOT_USED,
            //         NOT_USED,
            //         NOT_USED,
            //         DEFAULT_MESSAGE_TOPIC_NAME_SIZE_WEIGHT,
            //         NOT_USED,
            //         NOT_USED,
            //     ],
            // ),
            // disable_contract_version: HostFunction::default(),
            call: HostFunction::new(
                DEFAULT_CALL_COST,
                [
                    NOT_USED,
                    NOT_USED,
                    NOT_USED,
                    NOT_USED,
                    NOT_USED,
                    NOT_USED,
                    NOT_USED,
                    NOT_USED,
                    NOT_USED
                ],
            ),
            // call_versioned_contract: HostFunction::new(
            //     DEFAULT_CALL_CONTRACT_COST,
            //     [
            //         NOT_USED,
            //         NOT_USED,
            //         NOT_USED,
            //         NOT_USED,
            //         NOT_USED,
            //         DEFAULT_ARG_CHARGE,
            //         NOT_USED,
            //         DEFAULT_ARG_CHARGE,
            //         NOT_USED,
            //     ],
            // ),
            // get_named_arg_size: HostFunction::default(),
            // get_named_arg: HostFunction::new(
            //     200,
            //     [NOT_USED, DEFAULT_ARG_CHARGE, NOT_USED, DEFAULT_ARG_CHARGE],
            // ),
            // remove_contract_user_group: HostFunction::default(),
            // provision_contract_user_group_uref: HostFunction::default(),
            // remove_contract_user_group_urefs: HostFunction::new(
            //     200,
            //     [
            //         NOT_USED,
            //         NOT_USED,
            //         NOT_USED,
            //         NOT_USED,
            //         NOT_USED,
            //         DEFAULT_ARG_CHARGE,
            //     ],
            // ),
            print: HostFunction::new(
                DEFAULT_PRINT_COST,
                [NOT_USED, DEFAULT_PRINT_TEXT_SIZE_WEIGHT],
            ),
            // blake2b: HostFunction::new(
            //     DEFAULT_BLAKE2B_COST,
            //     [NOT_USED, DEFAULT_ARG_CHARGE, NOT_USED, NOT_USED],
            // ),
            // random_bytes: HostFunction::default(),
            // enable_contract_version: HostFunction::default(),
            // manage_message_topic: HostFunction::new(
            //     DEFAULT_FIXED_COST,
            //     [
            //         NOT_USED,
            //         DEFAULT_MESSAGE_TOPIC_NAME_SIZE_WEIGHT,
            //         NOT_USED,
            //         NOT_USED,
            //     ],
            // ),
            // emit_message: HostFunction::new(
            //     DEFAULT_FIXED_COST,
            //     [
            //         NOT_USED,
            //         DEFAULT_MESSAGE_TOPIC_NAME_SIZE_WEIGHT,
            //         NOT_USED,
            //         DEFAULT_MESSAGE_PAYLOAD_SIZE_WEIGHT,
            //     ],
            // ),
            // generic_hash: HostFunction::new(
            //     DEFAULT_GENERIC_HASH_COST,
            //     [
            //         NOT_USED,
            //         DEFAULT_GENERIC_HASH_INPUT_COST,
            //         NOT_USED,
            //         NOT_USED,
            //         NOT_USED,
            //     ],
            // ),
            // cost_increase_per_message: DEFAULT_COST_INCREASE_PER_MESSAGE_EMITTED,
            // get_block_info: HostFunction::new(DEFAULT_env_block_time_COST, [NOT_USED, NOT_USED]),
            // recover_secp256k1: HostFunction::new(
            //     DEFAULT_RECOVER_SECP256K1_COST,
            //     [
            //         NOT_USED,
            //         DEFAULT_RECOVER_SECP256K1_SIZE_WEIGHT,
            //         NOT_USED,
            //         NOT_USED,
            //         NOT_USED,
            //         NOT_USED,
            //     ],
            // ),
            // verify_signature: HostFunction::new(
            //     DEFAULT_VERIFY_SIGNATURE_COST,
            //     [
            //         NOT_USED,
            //         DEFAULT_VERIFY_SIGNATURE_SIZE_WEIGHT,
            //         NOT_USED,
            //         NOT_USED,
            //         NOT_USED,
            //         NOT_USED,
            //     ],
            // ),
        }
    }
}

impl ToBytes for HostFunctionCostsV2 {
    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        let mut ret = bytesrepr::unchecked_allocate_buffer(self);
        ret.append(&mut self.read.to_bytes()?);
        // ret.append(&mut self.dictionary_get.to_bytes()?);
        ret.append(&mut self.write.to_bytes()?);
        ret.append(&mut self.copy_input.to_bytes()?);
        // ret.append(&mut self.dictionary_put.to_bytes()?);
        // ret.append(&mut self.add.to_bytes()?);
        // ret.append(&mut self.new_uref.to_bytes()?);
        // ret.append(&mut self.load_named_keys.to_bytes()?);
        ret.append(&mut self.ret.to_bytes()?);
        ret.append(&mut self.create.to_bytes()?);
        // ret.append(&mut self.get_key.to_bytes()?);
        // ret.append(&mut self.has_key.to_bytes()?);
        // ret.append(&mut self.put_key.to_bytes()?);
        // ret.append(&mut self.remove_key.to_bytes()?);
        // ret.append(&mut self.revert.to_bytes()?);
        // ret.append(&mut self.is_valid_uref.to_bytes()?);
        // ret.append(&mut self.add_associated_key.to_bytes()?);
        // ret.append(&mut self.remove_associated_key.to_bytes()?);
        // ret.append(&mut self.update_associated_key.to_bytes()?);
        // ret.append(&mut self.set_action_threshold.to_bytes()?);
        ret.append(&mut self.env_caller.to_bytes()?);
        ret.append(&mut self.env_block_time.to_bytes()?);
        ret.append(&mut self.env_transferred_value.to_bytes()?);
        ret.append(&mut self.transfer.to_bytes()?);
        // ret.append(&mut self.transfer_from_purse_to_account.to_bytes()?);
        // ret.append(&mut self.transfer_from_purse_to_purse.to_bytes()?);
        ret.append(&mut self.env_balance.to_bytes()?);
        // ret.append(&mut self.get_phase.to_bytes()?);
        // ret.append(&mut self.get_system_contract.to_bytes()?);
        // ret.append(&mut self.get_main_purse.to_bytes()?);
        // ret.append(&mut self.read_host_buffer.to_bytes()?);
        // ret.append(&mut self.create_contract_package_at_hash.to_bytes()?);
        // ret.append(&mut self.create_contract_user_group.to_bytes()?);
        // ret.append(&mut self.upgrade_with_message_topics.to_bytes()?);
        ret.append(&mut self.upgrade.to_bytes()?);
        // ret.append(&mut self.add_package_version.to_bytes()?);
        // ret.append(&mut self.disable_contract_version.to_bytes()?);
        ret.append(&mut self.call.to_bytes()?);
        // ret.append(&mut self.call_versioned_contract.to_bytes()?);
        // ret.append(&mut self.get_named_arg_size.to_bytes()?);
        // ret.append(&mut self.get_named_arg.to_bytes()?);
        // ret.append(&mut self.remove_contract_user_group.to_bytes()?);
        // ret.append(&mut self.provision_contract_user_group_uref.to_bytes()?);
        // ret.append(&mut self.remove_contract_user_group_urefs.to_bytes()?);
        ret.append(&mut self.print.to_bytes()?);
        // ret.append(&mut self.blake2b.to_bytes()?);
        // ret.append(&mut self.random_bytes.to_bytes()?);
        // ret.append(&mut self.enable_contract_version.to_bytes()?);
        // ret.append(&mut self.manage_message_topic.to_bytes()?);
        // ret.append(&mut self.emit_message.to_bytes()?);
        // ret.append(&mut self.cost_increase_per_message.to_bytes()?);
        // ret.append(&mut self.get_block_info.to_bytes()?);
        // ret.append(&mut self.generic_hash.to_bytes()?);
        // ret.append(&mut self.recover_secp256k1.to_bytes()?);
        // ret.append(&mut self.verify_signature.to_bytes()?);
        Ok(ret)
    }

    fn serialized_length(&self) -> usize {
        self.read.serialized_length()
            // + self.dictionary_get.serialized_length()
            + self.write.serialized_length()
            + self.copy_input.serialized_length()
            // + self.dictionary_put.serialized_length()
            // + self.add.serialized_length()
            // + self.new_uref.serialized_length()
            // + self.load_named_keys.serialized_length()
            + self.ret.serialized_length()
            + self.create.serialized_length()
            // + self.get_key.serialized_length()
            // + self.has_key.serialized_length()
            // + self.put_key.serialized_length()
            // + self.remove_key.serialized_length()
            // + self.revert.serialized_length()
            // + self.is_valid_uref.serialized_length()
            // + self.add_associated_key.serialized_length()
            // + self.remove_associated_key.serialized_length()
            // + self.update_associated_key.serialized_length()
            // + self.set_action_threshold.serialized_length()
            + self.env_caller.serialized_length()
            + self.env_block_time.serialized_length()
            + self.env_transferred_value.serialized_length()
            + self.transfer.serialized_length()
            // + self.transfer_from_purse_to_account.serialized_length()
            // + self.transfer_from_purse_to_purse.serialized_length()
            + self.env_balance.serialized_length()
            // + self.get_phase.serialized_length()
            // + self.get_system_contract.serialized_length()
            // + self.get_main_purse.serialized_length()
            // + self.read_host_buffer.serialized_length()
            // + self.create_contract_package_at_hash.serialized_length()
            // + self.create_contract_user_group.serialized_length()
            // + self
            //     .upgrade_with_message_topics
            //     .serialized_length()
            + self.upgrade.serialized_length()
            // + self.add_package_version.serialized_length()
            // + self.disable_contract_version.serialized_length()
            + self.call.serialized_length()
            // + self.call_versioned_contract.serialized_length()
            // + self.get_named_arg_size.serialized_length()
            // + self.get_named_arg.serialized_length()
            // + self.remove_contract_user_group.serialized_length()
            // + self.provision_contract_user_group_uref.serialized_length()
            // + self.remove_contract_user_group_urefs.serialized_length()
            + self.print.serialized_length()
            // + self.blake2b.serialized_length()
            // + self.random_bytes.serialized_length()
            // + self.enable_contract_version.serialized_length()
            // + self.manage_message_topic.serialized_length()
            // + self.emit_message.serialized_length()
            // + self.cost_increase_per_message.serialized_length()
            // + self.get_block_info.serialized_length()
            // + self.generic_hash.serialized_length()
            // + self.recover_secp256k1.serialized_length()
            // + self.verify_signature.serialized_length()
    }
}

impl FromBytes for HostFunctionCostsV2 {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        let (read, rem) = FromBytes::from_bytes(bytes)?;
        // let (dictionary_get, rem) = FromBytes::from_bytes(rem)?;
        let (write, rem) = FromBytes::from_bytes(rem)?;
        let (copy_input, rem) = FromBytes::from_bytes(rem)?;
        // let (dictionary_put, rem) = FromBytes::from_bytes(rem)?;
        // let (add, rem) = FromBytes::from_bytes(rem)?;
        // let (new_uref, rem) = FromBytes::from_bytes(rem)?;
        // let (load_named_keys, rem) = FromBytes::from_bytes(rem)?;
        let (ret, rem) = FromBytes::from_bytes(rem)?;
        let (create, rem) = FromBytes::from_bytes(rem)?;
        // let (get_key, rem) = FromBytes::from_bytes(rem)?;
        // let (has_key, rem) = FromBytes::from_bytes(rem)?;
        // let (put_key, rem) = FromBytes::from_bytes(rem)?;
        // let (remove_key, rem) = FromBytes::from_bytes(rem)?;
        // let (revert, rem) = FromBytes::from_bytes(rem)?;
        // let (is_valid_uref, rem) = FromBytes::from_bytes(rem)?;
        // let (add_associated_key, rem) = FromBytes::from_bytes(rem)?;
        // let (remove_associated_key, rem) = FromBytes::from_bytes(rem)?;
        // let (update_associated_key, rem) = FromBytes::from_bytes(rem)?;
        // let (set_action_threshold, rem) = FromBytes::from_bytes(rem)?;
        let (env_caller, rem) = FromBytes::from_bytes(rem)?;
        let (env_block_time, rem) = FromBytes::from_bytes(rem)?;
        let (env_transferred_value, rem) = FromBytes::from_bytes(rem)?;
        let (transfer, rem) = FromBytes::from_bytes(rem)?;
        // let (transfer_from_purse_to_account, rem) = FromBytes::from_bytes(rem)?;
        // let (transfer_from_purse_to_purse, rem) = FromBytes::from_bytes(rem)?;
        let (env_balance, rem) = FromBytes::from_bytes(rem)?;
        // let (get_phase, rem) = FromBytes::from_bytes(rem)?;
        // let (get_system_contract, rem) = FromBytes::from_bytes(rem)?;
        // let (get_main_purse, rem) = FromBytes::from_bytes(rem)?;
        // let (read_host_buffer, rem) = FromBytes::from_bytes(rem)?;
        // let (create_contract_package_at_hash, rem) = FromBytes::from_bytes(rem)?;
        // let (create_contract_user_group, rem) = FromBytes::from_bytes(rem)?;
        // let (upgrade_with_message_topics, rem) = FromBytes::from_bytes(rem)?;
        let (upgrade, rem) = FromBytes::from_bytes(rem)?;
        // let (add_package_version, rem) = FromBytes::from_bytes(rem)?;
        // let (disable_contract_version, rem) = FromBytes::from_bytes(rem)?;
        let (call, rem) = FromBytes::from_bytes(rem)?;
        // let (call_versioned_contract, rem) = FromBytes::from_bytes(rem)?;
        // let (get_named_arg_size, rem) = FromBytes::from_bytes(rem)?;
        // let (get_named_arg, rem) = FromBytes::from_bytes(rem)?;
        // let (remove_contract_user_group, rem) = FromBytes::from_bytes(rem)?;
        // let (provision_contract_user_group_uref, rem) = FromBytes::from_bytes(rem)?;
        // let (remove_contract_user_group_urefs, rem) = FromBytes::from_bytes(rem)?;
        let (print, rem) = FromBytes::from_bytes(rem)?;
        // let (blake2b, rem) = FromBytes::from_bytes(rem)?;
        // let (random_bytes, rem) = FromBytes::from_bytes(rem)?;
        // let (enable_contract_version, rem) = FromBytes::from_bytes(rem)?;
        // let (manage_message_topic, rem) = FromBytes::from_bytes(rem)?;
        // let (emit_message, rem) = FromBytes::from_bytes(rem)?;
        // let (cost_increase_per_message, rem) = FromBytes::from_bytes(rem)?;
        // let (get_block_info, rem) = FromBytes::from_bytes(rem)?;
        // let (generic_hash, rem) = FromBytes::from_bytes(rem)?;
        // let (recover_secp256k1, rem) = FromBytes::from_bytes(rem)?;
        // let (verify_signature, rem) = FromBytes::from_bytes(rem)?;
        Ok((
            HostFunctionCostsV2 {
                read,
                // dictionary_get,
                write,
                copy_input,
                // dictionary_put,
                // add,
                // new_uref,
                // load_named_keys,
                ret,
                create,
                // get_key,
                // has_key,
                // put_key,
                // remove_key,
                // revert,
                // is_valid_uref,
                // add_associated_key,
                // remove_associated_key,
                // update_associated_key,
                // set_action_threshold,
                env_caller,
                env_block_time,
                env_transferred_value,
                transfer,
                // transfer_from_purse_to_account,
                // transfer_from_purse_to_purse,
                env_balance,
                // get_phase,
                // get_system_contract,
                // get_main_purse,
                // read_host_buffer,
                // create_contract_package_at_hash,
                // create_contract_user_group,
                // upgrade_with_message_topics,
                upgrade,
                // add_package_version,
                // disable_contract_version,
                call,
                // call_versioned_contract,
                // get_named_arg_size,
                // get_named_arg,
                // remove_contract_user_group,
                // provision_contract_user_group_uref,
                // remove_contract_user_group_urefs,
                print,
                // blake2b,
                // random_bytes,
                // enable_contract_version,
                // manage_message_topic,
                // emit_message,
                // cost_increase_per_message,
                // get_block_info,
                // generic_hash,
                // recover_secp256k1,
                // verify_signature,
            },
            rem,
        ))
    }
}

#[cfg(any(feature = "testing", test))]
impl Distribution<HostFunctionCostsV2> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> HostFunctionCostsV2 {
        HostFunctionCostsV2 {
            read: rng.gen(),
            // dictionary_get: rng.gen(),
            write: rng.gen(),
            copy_input: rng.gen(),
            // dictionary_put: rng.gen(),
            // add: rng.gen(),
            // new_uref: rng.gen(),
            // load_named_keys: rng.gen(),
            ret: rng.gen(),
            create: rng.gen(),
            // get_key: rng.gen(),
            // has_key: rng.gen(),
            // put_key: rng.gen(),
            // remove_key: rng.gen(),
            // revert: rng.gen(),
            // is_valid_uref: rng.gen(),
            // add_associated_key: rng.gen(),
            // remove_associated_key: rng.gen(),
            // update_associated_key: rng.gen(),
            // set_action_threshold: rng.gen(),
            env_caller: rng.gen(),
            env_block_time: rng.gen(),
            env_transferred_value: rng.gen(),
            transfer: rng.gen(),
            // transfer_from_purse_to_account: rng.gen(),
            // transfer_from_purse_to_purse: rng.gen(),
            env_balance: rng.gen(),
            // get_phase: rng.gen(),
            // get_system_contract: rng.gen(),
            // get_main_purse: rng.gen(),
            // read_host_buffer: rng.gen(),
            // create_contract_package_at_hash: rng.gen(),
            // create_contract_user_group: rng.gen(),
            // upgrade_with_message_topics: rng.gen(),
            upgrade: rng.gen(),
            // add_package_version: rng.gen(),
            // disable_contract_version: rng.gen(),
            call: rng.gen(),
            // call_versioned_contract: rng.gen(),
            // get_named_arg_size: rng.gen(),
            // get_named_arg: rng.gen(),
            // remove_contract_user_group: rng.gen(),
            // provision_contract_user_group_uref: rng.gen(),
            // remove_contract_user_group_urefs: rng.gen(),
            print: rng.gen(),
            // blake2b: rng.gen(),
            // random_bytes: rng.gen(),
            // enable_contract_version: rng.gen(),
            // manage_message_topic: rng.gen(),
            // emit_message: rng.gen(),
            // cost_increase_per_message: rng.gen(),
            // get_block_info: rng.gen(),
            // generic_hash: rng.gen(),
            // recover_secp256k1: rng.gen(),
            // verify_signature: rng.gen(),
        }
    }
}

#[doc(hidden)]
#[cfg(any(feature = "gens", test))]
pub mod gens {
    use proptest::{num, prelude::*};

    use crate::{HostFunction, HostFunctionCost, HostFunctionCostsV2};

    #[allow(unused)]
    pub fn host_function_cost_v2_arb<T: Copy + Arbitrary>() -> impl Strategy<Value = HostFunction<T>> {
        (any::<HostFunctionCost>(), any::<T>())
            .prop_map(|(cost, arguments)| HostFunction::new(cost, arguments))
    }

    prop_compose! {
        pub fn host_function_costs_v2_arb() (
            read in host_function_cost_v2_arb(),
            // dictionary_get in host_function_cost_v2_arb(),
            write in host_function_cost_v2_arb(),
            copy_input in host_function_cost_v2_arb(),
            // dictionary_put in host_function_cost_v2_arb(),
            // add in host_function_cost_v2_arb(),
            // new_uref in host_function_cost_v2_arb(),
            // load_named_keys in host_function_cost_v2_arb(),
            ret in host_function_cost_v2_arb(),
            create in host_function_cost_v2_arb(),
            // get_key in host_function_cost_v2_arb(),
            // has_key in host_function_cost_v2_arb(),
            // put_key in host_function_cost_v2_arb(),
            // remove_key in host_function_cost_v2_arb(),
            // revert in host_function_cost_v2_arb(),
            // is_valid_uref in host_function_cost_v2_arb(),
            // add_associated_key in host_function_cost_v2_arb(),
            // remove_associated_key in host_function_cost_v2_arb(),
            // update_associated_key in host_function_cost_v2_arb(),
            // set_action_threshold in host_function_cost_v2_arb(),
            env_caller in host_function_cost_v2_arb(),
            env_block_time in host_function_cost_v2_arb(),
            env_transferred_value in host_function_cost_v2_arb(),
            transfer in host_function_cost_v2_arb(),
            // transfer_from_purse_to_account in host_function_cost_v2_arb(),
            // transfer_from_purse_to_purse in host_function_cost_v2_arb(),
            env_balance in host_function_cost_v2_arb(),
            // get_phase in host_function_cost_v2_arb(),
            // get_system_contract in host_function_cost_v2_arb(),
            // get_main_purse in host_function_cost_v2_arb(),
            // read_host_buffer in host_function_cost_v2_arb(),
            // create_contract_package_at_hash in host_function_cost_v2_arb(),
            // create_contract_user_group in host_function_cost_v2_arb(),
            // upgrade_with_message_topics in host_function_cost_v2_arb(),
            upgrade in host_function_cost_v2_arb(),
            // add_package_version in host_function_cost_v2_arb(),
            // disable_contract_version in host_function_cost_v2_arb(),
            call in host_function_cost_v2_arb(),
            // call_versioned_contract in host_function_cost_v2_arb(),
            // get_named_arg_size in host_function_cost_v2_arb(),
            // get_named_arg in host_function_cost_v2_arb(),
            // remove_contract_user_group in host_function_cost_v2_arb(),
            // provision_contract_user_group_uref in host_function_cost_v2_arb(),
            // remove_contract_user_group_urefs in host_function_cost_v2_arb(),
            print in host_function_cost_v2_arb(),
            // blake2b in host_function_cost_v2_arb(),
            // random_bytes in host_function_cost_v2_arb(),
            // enable_contract_version in host_function_cost_v2_arb(),
            // manage_message_topic in host_function_cost_v2_arb(),
            // emit_message in host_function_cost_v2_arb(),
            // cost_increase_per_message in num::u32::ANY,
            // get_block_info in host_function_cost_v2_arb(),
            // generic_hash in host_function_cost_v2_arb(),
            // recover_secp256k1 in host_function_cost_v2_arb(),
            // verify_signature in host_function_cost_v2_arb(),
        ) -> HostFunctionCostsV2 {
            HostFunctionCostsV2 {
                read,
                // dictionary_get,
                write,
                copy_input,
                // dictionary_put,
                // add,
                // new_uref,
                // load_named_keys,
                ret,
                create,
                // get_key,
                // has_key,
                // put_key,
                // remove_key,
                // revert,
                // is_valid_uref,
                // add_associated_key,
                // remove_associated_key,
                // update_associated_key,
                // set_action_threshold,
                env_caller,
                env_block_time,
                env_transferred_value,
                transfer,
                // transfer_from_purse_to_account,
                // transfer_from_purse_to_purse,
                env_balance,
                // get_phase,
                // get_system_contract,
                // get_main_purse,
                // read_host_buffer,
                // create_contract_package_at_hash,
                // create_contract_user_group,
                // upgrade_with_message_topics,
                upgrade,
                // add_package_version,
                // disable_contract_version,
                call,
                // call_versioned_contract,
                // get_named_arg_size,
                // get_named_arg,
                // remove_contract_user_group,
                // provision_contract_user_group_uref,
                // remove_contract_user_group_urefs,
                print,
                // blake2b,
                // random_bytes,
                // enable_contract_version,
                // manage_message_topic,
                // emit_message,
                // cost_increase_per_message,
                // get_block_info,
                // generic_hash,
                // recover_secp256k1,
                // verify_signature,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::U512;

    use super::*;

    const COST: Cost = 42;
    const ARGUMENT_COSTS: [Cost; 3] = [123, 456, 789];
    const WEIGHTS: [Cost; 3] = [1000, 1100, 1200];

    #[test]
    fn calculate_gas_cost_for_host_function() {
        let host_function = HostFunction::new(COST, ARGUMENT_COSTS);
        let expected_cost = COST
            + (ARGUMENT_COSTS[0] * WEIGHTS[0])
            + (ARGUMENT_COSTS[1] * WEIGHTS[1])
            + (ARGUMENT_COSTS[2] * WEIGHTS[2]);
        assert_eq!(
            host_function.calculate_gas_cost(WEIGHTS),
            Some(Gas::new(expected_cost))
        );
    }

    #[test]
    fn calculate_gas_cost_would_overflow() {
        let large_value = Cost::MAX;

        let host_function = HostFunction::new(
            large_value,
            [large_value, large_value, large_value, large_value],
        );

        let lhs =
            host_function.calculate_gas_cost([large_value, large_value, large_value, large_value]);

        let large_value = U512::from(large_value);
        let rhs = large_value + (U512::from(4) * large_value * large_value);

        assert_eq!(lhs, Some(Gas::new(rhs)));
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use crate::bytesrepr;

    use super::*;

    type Signature = [Cost; 10];

    proptest! {
        #[test]
        fn test_host_function(host_function in gens::host_function_cost_v2_arb::<Signature>()) {
            bytesrepr::test_serialization_roundtrip(&host_function);
        }

        #[test]
        fn test_host_function_costs(host_function_costs in gens::host_function_costs_v2_arb()) {
            bytesrepr::test_serialization_roundtrip(&host_function_costs);
        }
    }
}
