//! Lazy hash map

use std::fmt::Display;
use std::marker::PhantomData;

use borsh::{BorshDeserialize, BorshSerialize};

use super::super::Result;
use crate::ledger::storage_api::{self, StorageRead, StorageWrite};
use crate::types::storage;

/// Subkey corresponding to the data elements of the LazyMap
pub const DATA_SUBKEY: &str = "data";

/// LazyMap ! fill in !
pub struct LazyMap<K, V> {
    key: storage::Key,
    phantom_k: PhantomData<K>,
    phantom_v: PhantomData<V>,
}

impl<K, V> LazyMap<K, V>
where
    K: BorshDeserialize + BorshSerialize + Display,
    V: BorshDeserialize + BorshSerialize,
{
    /// Create or use an existing map with the given storage `key`.
    pub fn new(key: storage::Key) -> Self {
        Self {
            key,
            phantom_k: PhantomData,
            phantom_v: PhantomData,
        }
    }

    /// Inserts a key-value pair into the map.
    ///
    /// The full storage key identifies the key in the pair, while the value is
    /// held within the storage key.
    ///
    /// If the map did not have this key present, `None` is returned.
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned. Unlike in `std::collection::HashMap`, the key is also
    /// updated; this matters for types that can be `==` without being
    /// identical.
    pub fn insert<S>(
        &self,
        storage: &mut S,
        key: K,
        val: V,
    ) -> Result<Option<V>>
    where
        S: StorageWrite + StorageRead,
    {
        let previous = self.get(storage, &key)?;

        let data_key = self.get_data_key(&key);
        Self::write_key_val(storage, &data_key, val)?;

        Ok(previous)
    }

    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map.
    pub fn remove<S>(&self, storage: &mut S, key: &K) -> Result<Option<V>>
    where
        S: StorageWrite + StorageRead,
    {
        let value = self.get(storage, key)?;

        let data_key = self.get_data_key(key);
        storage.delete(&data_key)?;

        Ok(value)
    }

    /// Returns the value corresponding to the key, if any.
    pub fn get(
        &self,
        storage: &impl StorageRead,
        key: &K,
    ) -> Result<Option<V>> {
        let data_key = self.get_data_key(key);
        Self::read_key_val(storage, &data_key)
    }

    /// An iterator visiting all key-value elements. The iterator element type
    /// is `Result<(K, V)>`, because iterator's call to `next` may fail with
    /// e.g. out of gas or data decoding error.
    ///
    /// Note that this function shouldn't be used in transactions and VPs code
    /// on unbounded sets to avoid gas usage increasing with the length of the
    /// map.
    pub fn iter<'a>(
        &self,
        storage: &'a impl StorageRead,
    ) -> Result<impl Iterator<Item = Result<V>> + 'a> {
        let iter = storage.iter_prefix(&self.get_data_prefix())?;
        let iter = itertools::unfold(iter, |iter| {
            match storage.iter_next(iter) {
                Ok(Some((_key, value))) => {
                    match V::try_from_slice(&value[..]) {
                        Ok(decoded_value) => Some(Ok(decoded_value)),
                        Err(err) => Some(Err(storage_api::Error::new(err))),
                    }
                }
                Ok(None) => None,
                Err(err) => {
                    // Propagate errors into Iterator's Item
                    Some(Err(err))
                }
            }
        });
        Ok(iter)
    }

    /// Reads a value from storage
    fn read_key_val(
        storage: &impl StorageRead,
        storage_key: &storage::Key,
    ) -> Result<Option<V>> {
        let res = storage.read(storage_key)?;
        Ok(res)
    }

    /// Write a value into storage
    fn write_key_val(
        storage: &mut impl StorageWrite,
        storage_key: &storage::Key,
        val: V,
    ) -> Result<()> {
        storage.write(storage_key, val)
    }

    /// Get the prefix of set's elements storage
    fn get_data_prefix(&self) -> storage::Key {
        self.key.push(&DATA_SUBKEY.to_owned()).unwrap()
    }

    /// Get the sub-key of a given element
    fn get_data_key(&self, key: &K) -> storage::Key {
        self.get_data_prefix().push(&key.to_string()).unwrap()
    }
}
