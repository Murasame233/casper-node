use std::path::Path;

use clap::ArgMatches;
use lmdb::{self, Cursor, Environment, EnvironmentFlags, Transaction};

use casper_engine_test_support::LmdbWasmTestBuilder;
use casper_execution_engine::engine_state::engine_config::{
    DEFAULT_ENABLE_ENTITY, DEFAULT_PROTOCOL_VERSION,
};
use casper_storage::{
    data_access_layer::{
        SystemEntityRegistryPayload, SystemEntityRegistryRequest, SystemEntityRegistrySelector,
    },
    global_state::state::StateProvider,
};
use casper_types::{
    bytesrepr::FromBytes,
    system::{AUCTION, HANDLE_PAYMENT, MINT, STANDARD_PAYMENT},
    AddressableEntityHash, CLValue, Key, ProtocolVersion, StoredValue, SystemHashRegistry,
    KEY_HASH_LENGTH,
};

use crate::utils::{hash_from_str, print_entry};

const DATABASE_NAME: &str = "PROTOCOL_DATA_STORE";

pub(crate) fn generate_system_entity_registry(matches: &ArgMatches<'_>) {
    let data_dir = Path::new(matches.value_of("data_dir").unwrap_or("."));
    match matches.value_of("hash") {
        None => generate_system_entity_registry_using_protocol_data(data_dir),
        Some(hash) => generate_system_entity_registry_using_global_state(data_dir, hash),
    }
}

fn generate_system_entity_registry_using_protocol_data(data_dir: &Path) {
    let database_path = data_dir.join("data.lmdb");

    let env = Environment::new()
        .set_flags(EnvironmentFlags::READ_ONLY | EnvironmentFlags::NO_SUB_DIR)
        .set_max_dbs(2)
        .open(&database_path)
        .unwrap_or_else(|error| {
            panic!(
                "failed to initialize database environment at {}: {}",
                database_path.display(),
                error
            )
        });

    let protocol_data_db = env.open_db(Some(DATABASE_NAME)).unwrap_or_else(|error| {
        panic!("failed to open database named {}: {}", DATABASE_NAME, error)
    });

    let ro_transaction = env
        .begin_ro_txn()
        .unwrap_or_else(|error| panic!("failed to initialize read-only transaction: {}", error));
    let mut cursor = ro_transaction
        .open_ro_cursor(protocol_data_db)
        .unwrap_or_else(|error| panic!("failed to open a read-only cursor: {}", error));

    let serialized_protocol_data = match cursor.iter().next().map(Result::unwrap) {
        Some((_key, value)) => value,
        None => {
            println!("No protocol data found");
            return;
        }
    };

    // The last four 32-byte chunks of the serialized data are the contract hashes.
    let start_index = serialized_protocol_data
        .len()
        .saturating_sub(4 * KEY_HASH_LENGTH);
    let remainder = &serialized_protocol_data[start_index..];
    let (mint_hash, remainder) =
        AddressableEntityHash::from_bytes(remainder).unwrap_or_else(|error| {
            panic!(
                "failed to parse mint hash: {:?}\nraw_bytes: {:?}",
                error, serialized_protocol_data
            )
        });
    let (handle_payment_hash, remainder) = AddressableEntityHash::from_bytes(remainder)
        .unwrap_or_else(|error| {
            panic!(
                "failed to parse handle_payment hash: {:?}\nraw_bytes: {:?}",
                error, serialized_protocol_data
            )
        });
    let (standard_payment_hash, remainder) = AddressableEntityHash::from_bytes(remainder)
        .unwrap_or_else(|error| {
            panic!(
                "failed to parse standard_payment hash: {:?}\nraw_bytes: {:?}",
                error, serialized_protocol_data
            )
        });
    let (auction_hash, remainder) =
        AddressableEntityHash::from_bytes(remainder).unwrap_or_else(|error| {
            panic!(
                "failed to parse auction hash: {:?}\nraw_bytes: {:?}",
                error, serialized_protocol_data
            )
        });
    assert!(remainder.is_empty());

    let mut registry = SystemHashRegistry::new();
    registry.insert(MINT.to_string(), mint_hash.value());
    registry.insert(HANDLE_PAYMENT.to_string(), handle_payment_hash.value());
    registry.insert(STANDARD_PAYMENT.to_string(), standard_payment_hash.value());
    registry.insert(AUCTION.to_string(), auction_hash.value());

    print_entry(
        &Key::SystemEntityRegistry,
        &StoredValue::from(CLValue::from_t(registry).unwrap()),
    );
}

fn generate_system_entity_registry_using_global_state(data_dir: &Path, state_hash: &str) {
    let builder = LmdbWasmTestBuilder::open_raw(
        data_dir,
        Default::default(),
        DEFAULT_PROTOCOL_VERSION,
        hash_from_str(state_hash),
    );

    let registry_req = SystemEntityRegistryRequest::new(
        builder.get_post_state_hash(),
        ProtocolVersion::V2_0_0,
        SystemEntityRegistrySelector::All,
        DEFAULT_ENABLE_ENTITY,
    );

    let registry = match builder
        .data_access_layer()
        .system_entity_registry(registry_req)
        .as_registry_payload()
        .expect("should have payload")
    {
        SystemEntityRegistryPayload::All(registry) => registry,
        SystemEntityRegistryPayload::EntityKey(_) => {
            panic!("expected registry");
        }
    };

    // make sure expected entries exist
    let _ = *registry.get(MINT).expect("mint should exist");
    let _ = *registry.get(AUCTION).expect("auction should exist");
    let _ = *registry
        .get(HANDLE_PAYMENT)
        .expect("handle payment should exist");

    print_entry(
        &Key::SystemEntityRegistry,
        &StoredValue::from(CLValue::from_t(registry).unwrap()),
    );
}
