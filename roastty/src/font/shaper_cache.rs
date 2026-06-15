//! Cache shaped cells by text-run hash.
//!
//! Faithful port of upstream `font/shaper/Cache.zig`: shaped cells are copied
//! into cache-owned storage and keyed by [`TextRun::hash`] plus a shaping-option
//! namespace. The fixed-bucket LRU table stores slot indexes so the cached cell
//! vectors can stay owned by this module.

use crate::font::run::TextRun;
use crate::font::shape;
use crate::terminal::cache_table::{CacheContext, CacheTable};

struct RunHashContext;

impl CacheContext<u64> for RunHashContext {
    fn hash(&self, key: &u64) -> u64 {
        *key
    }

    fn eql(&self, a: &u64, b: &u64) -> bool {
        a == b
    }
}

type CellCacheTable = CacheTable<u64, usize, RunHashContext, 256, 8>;

/// A shaped-run cache keyed by [`TextRun::hash`] and shaping-option namespace.
pub(crate) struct ShaperCache {
    map: CellCacheTable,
    slots: Vec<Option<Vec<shape::Cell>>>,
    free: Vec<usize>,
}

impl ShaperCache {
    pub(crate) fn new() -> Self {
        Self {
            map: CellCacheTable::new(RunHashContext),
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    /// Get the shaped cells for `run`, bumping the cache entry to most-recent.
    pub(crate) fn get(&mut self, run: TextRun) -> Option<&[shape::Cell]> {
        self.get_with_namespace(run, 0)
    }

    /// Get shaped cells for `run` under a shaping-option namespace.
    pub(crate) fn get_with_namespace(
        &mut self,
        run: TextRun,
        namespace: u64,
    ) -> Option<&[shape::Cell]> {
        let slot = self.map.get(cache_key(run, namespace))?;
        self.slots.get(slot)?.as_deref()
    }

    /// Insert a cache-owned copy of `cells` for `run`.
    ///
    /// Unlike upstream's raw `CacheTable.put`, this replaces an existing key
    /// instead of appending a duplicate. The renderer's intended use is
    /// miss-then-put, so duplicate keys are not semantically meaningful.
    pub(crate) fn put(&mut self, run: TextRun, cells: &[shape::Cell]) {
        self.put_with_namespace(run, 0, cells)
    }

    /// Insert a cache-owned copy of `cells` for `run` under a shaping-option
    /// namespace.
    pub(crate) fn put_with_namespace(
        &mut self,
        run: TextRun,
        namespace: u64,
        cells: &[shape::Cell],
    ) {
        let key = cache_key(run, namespace);
        if let Some(slot) = self.map.get(key) {
            self.slots[slot] = Some(cells.to_vec());
            return;
        }

        let slot = self.alloc_slot();
        self.slots[slot] = Some(cells.to_vec());
        if let Some((_hash, evicted_slot)) = self.map.put(key, slot) {
            if evicted_slot != slot && self.slots[evicted_slot].take().is_some() {
                self.free.push(evicted_slot);
            }
        }
    }

    /// Remove all cached entries.
    pub(crate) fn clear(&mut self) {
        self.map.clear();
        self.slots.clear();
        self.free.clear();
    }

    fn alloc_slot(&mut self) -> usize {
        if let Some(slot) = self.free.pop() {
            slot
        } else {
            let slot = self.slots.len();
            self.slots.push(None);
            slot
        }
    }

    #[cfg(test)]
    pub(crate) fn slot_count(&self) -> usize {
        self.slots.len()
    }
}

fn cache_key(run: TextRun, namespace: u64) -> u64 {
    if namespace == 0 {
        return run.hash;
    }
    let mut hash = 0xcbf29ce484222325u64;
    for part in [run.hash, namespace] {
        for byte in part.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    if hash == run.hash {
        hash ^ namespace
    } else {
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::{collection::Index, Style};

    fn run(hash: u64) -> TextRun {
        TextRun {
            hash,
            offset: 0,
            cells: 1,
            font_index: Index::new(Style::Regular, 0),
        }
    }

    fn cell(glyph_index: u32) -> shape::Cell {
        shape::Cell {
            x: glyph_index as u16,
            x_offset: 0,
            y_offset: 0,
            glyph_index,
        }
    }

    #[test]
    fn miss_then_hit_returns_cells() {
        let mut cache = ShaperCache::new();
        let run = run(1);
        assert!(cache.get(run).is_none());

        let cells = [cell(10), cell(11)];
        cache.put(run, &cells);
        assert_eq!(cache.get(run), Some(cells.as_slice()));
    }

    #[test]
    fn cached_cells_are_owned_copy() {
        let mut cache = ShaperCache::new();
        let run = run(2);
        let mut cells = vec![cell(20)];
        cache.put(run, &cells);

        cells[0] = cell(99);
        assert_eq!(cache.get(run), Some(&[cell(20)][..]));
    }

    #[test]
    fn put_same_run_replaces_value() {
        let mut cache = ShaperCache::new();
        let run = run(3);

        cache.put(run, &[cell(30)]);
        cache.put(run, &[cell(31), cell(32)]);

        assert_eq!(cache.get(run), Some(&[cell(31), cell(32)][..]));
        assert_eq!(
            cache.slot_count(),
            1,
            "same-key replacement reuses the slot"
        );
    }

    #[test]
    fn shaper_cache_feature_namespace_separates_same_run() {
        let mut cache = ShaperCache::new();
        let run = run(5);

        cache.put_with_namespace(run, 1, &[cell(50)]);
        cache.put_with_namespace(run, 2, &[cell(60)]);

        assert_eq!(cache.get_with_namespace(run, 1), Some(&[cell(50)][..]));
        assert_eq!(cache.get_with_namespace(run, 2), Some(&[cell(60)][..]));
        assert!(cache.get(run).is_none(), "default namespace is separate");
        assert_eq!(cache.slot_count(), 2);
    }

    #[test]
    fn same_bucket_insert_evicts_lru() {
        let mut cache = ShaperCache::new();
        for i in 0..8 {
            let hash = i * 256;
            cache.put(run(hash), &[cell(i as u32)]);
        }

        cache.put(run(8 * 256), &[cell(8)]);

        assert!(
            cache.get(run(0)).is_none(),
            "oldest same-bucket run evicted"
        );
        for i in 1..=8 {
            let hash = i * 256;
            assert_eq!(cache.get(run(hash)), Some(&[cell(i as u32)][..]));
        }
    }

    #[test]
    fn get_hit_bumps_to_most_recent() {
        let mut cache = ShaperCache::new();
        for i in 0..8 {
            let hash = i * 256;
            cache.put(run(hash), &[cell(i as u32)]);
        }

        assert_eq!(cache.get(run(0)), Some(&[cell(0)][..]));
        cache.put(run(8 * 256), &[cell(8)]);

        assert_eq!(cache.get(run(0)), Some(&[cell(0)][..]));
        assert!(
            cache.get(run(256)).is_none(),
            "the non-bumped oldest run was evicted"
        );
    }

    #[test]
    fn repeated_evictions_reuse_slots() {
        let mut cache = ShaperCache::new();
        for i in 0..64 {
            let hash = i * 256;
            cache.put(run(hash), &[cell(i as u32)]);
        }

        assert_eq!(
            cache.slot_count(),
            9,
            "one same-bucket insertion beyond capacity allocates the replacement slot; later evictions reuse freed slots"
        );
    }

    #[test]
    fn clear_removes_entries() {
        let mut cache = ShaperCache::new();
        cache.put(run(4), &[cell(40)]);
        assert!(cache.get(run(4)).is_some());

        cache.clear();
        assert!(cache.get(run(4)).is_none());
        assert_eq!(cache.slot_count(), 0);
    }
}
