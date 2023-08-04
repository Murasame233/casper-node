use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use bytes::Bytes;
use vm::{
    backend::{Context, WasmInstance},
    storage::{self, Storage},
    ExecuteRequest, VM,
};

// use super::*;
const TEST_CONTRACT_WASM: &[u8] = include_bytes!("../test-contract.wasm");

#[derive(Default, Debug, Clone)]
struct MockStorage {
    // journal: Arc<Vec<JournalEntry>>,
    db: Arc<RwLock<HashMap<Bytes, Bytes>>>,
}

impl Storage for MockStorage {
    fn write(&self, key: &[u8], value: &[u8]) -> Result<(), storage::Error> {
        let key_bytes = Bytes::copy_from_slice(key);
        let value_bytes = Bytes::copy_from_slice(value);
        // self.journal.push(JournalEntry::Write(key_bytes.clone(), value_bytes.clone()));
        self.db.write().unwrap().insert(key_bytes, value_bytes);
        Ok(())
    }

    fn read(&self, key: &[u8]) -> Result<Option<Bytes>, storage::Error> {
        // let key_bytes = Bytes::copy_from_slice(key);
        // self.journal.push(JournalEntry::Read(key_bytes.clone()));
        match self.db.read().unwrap().get(key) {
            Some(value) => Ok(Some(value.clone())),
            None => Ok(None),
        }
    }
}

#[test]
fn smoke() {
    let mut vm = VM::new();
    let execute_request = ExecuteRequest {
        wasm_bytes: Bytes::from_static(TEST_CONTRACT_WASM),
    };

    let storage = MockStorage::default();

    let mock_context = Context { storage };

    let retrieved_context = {
        let mut instance = vm
            .prepare(execute_request, mock_context)
            .expect("should prepare");


        let args = &[b"hello".as_slice(), b"world".as_slice()];

        let (result, gas_summary) = instance.call_export("call", args);
        dbg!(&result, gas_summary);

        instance.teardown()
    };

    // dbg!(&res);
    dbg!(&retrieved_context.storage);
    // retrieved_context.storage
}
