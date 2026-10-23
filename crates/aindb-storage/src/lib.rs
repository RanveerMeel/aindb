//! Byte-oriented storage backend trait (SPEC §10, ADR-0001). Backends know
//! nothing about objects or events. The one hard requirement is that
//! [`Backend::commit`] applies a whole [`Batch`] atomically and durably.
//!
//! [`MemoryBackend`] is the reference implementation used by tests; the
//! durable backend (redb, proposed) arrives in M3.

use std::collections::BTreeMap;
use std::ops::Bound;
use std::sync::{Arc, RwLock};

pub type Key = Vec<u8>;
pub type Bytes = Vec<u8>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Batch {
    pub puts: Vec<(Key, Bytes)>,
    pub deletes: Vec<Key>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StorageError {
    Io(String),
    Corrupt(String),
}

/// A consistent, read-only view of the store at one point in time.
pub trait Snapshot: Send + Sync {
    fn get(&self, key: &[u8]) -> Result<Option<Bytes>, StorageError>;
    /// Ordered range `[start, end)`, at most `limit` entries.
    fn range(
        &self,
        start: &[u8],
        end: &[u8],
        limit: usize,
    ) -> Result<Vec<(Key, Bytes)>, StorageError>;
}

pub trait Backend: Send + Sync {
    fn snapshot(&self) -> Result<Box<dyn Snapshot>, StorageError>;
    /// Apply all puts and deletes atomically; durable on return.
    fn commit(&self, batch: Batch) -> Result<(), StorageError>;
}

/// In-memory backend. Snapshots are copy-on-write clones of the map.
#[derive(Default)]
pub struct MemoryBackend {
    data: RwLock<Arc<BTreeMap<Key, Bytes>>>,
}

struct MemorySnapshot(Arc<BTreeMap<Key, Bytes>>);

impl Snapshot for MemorySnapshot {
    fn get(&self, key: &[u8]) -> Result<Option<Bytes>, StorageError> {
        Ok(self.0.get(key).cloned())
    }

    fn range(
        &self,
        start: &[u8],
        end: &[u8],
        limit: usize,
    ) -> Result<Vec<(Key, Bytes)>, StorageError> {
        Ok(self
            .0
            .range::<[u8], _>((Bound::Included(start), Bound::Excluded(end)))
            .take(limit)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect())
    }
}

impl Backend for MemoryBackend {
    fn snapshot(&self) -> Result<Box<dyn Snapshot>, StorageError> {
        let guard = self
            .data
            .read()
            .map_err(|e| StorageError::Io(e.to_string()))?;
        Ok(Box::new(MemorySnapshot(Arc::clone(&guard))))
    }

    fn commit(&self, batch: Batch) -> Result<(), StorageError> {
        let mut guard = self
            .data
            .write()
            .map_err(|e| StorageError::Io(e.to_string()))?;
        let mut next = (**guard).clone();
        for k in batch.deletes {
            next.remove(&k);
        }
        for (k, v) in batch.puts {
            next.insert(k, v);
        }
        *guard = Arc::new(next);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_is_isolated_from_later_commits() {
        let db = MemoryBackend::default();
        db.commit(Batch {
            puts: vec![(b"a".to_vec(), b"1".to_vec())],
            deletes: vec![],
        })
        .unwrap();
        let snap = db.snapshot().unwrap();
        db.commit(Batch {
            puts: vec![
                (b"a".to_vec(), b"2".to_vec()),
                (b"b".to_vec(), b"3".to_vec()),
            ],
            deletes: vec![],
        })
        .unwrap();
        assert_eq!(snap.get(b"a").unwrap(), Some(b"1".to_vec()));
        assert_eq!(snap.get(b"b").unwrap(), None);
        let now = db.snapshot().unwrap();
        assert_eq!(now.range(b"a", b"c", 10).unwrap().len(), 2);
    }
}
