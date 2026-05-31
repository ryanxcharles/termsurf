use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem::{align_of, size_of, MaybeUninit};

use super::size::{Offset, OffsetBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Error {
    OutOfMemory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Layout {
    pub(super) total_size: usize,
    pub(super) keys_start: usize,
    pub(super) vals_start: usize,
    pub(super) capacity: u32,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct OffsetHashMap<K, V> {
    metadata: Offset<Metadata>,
    _marker: PhantomData<fn() -> (K, V)>,
}

impl<K, V> OffsetHashMap<K, V>
where
    K: Copy + Eq + Hash,
    V: Copy + Default,
{
    pub(super) const BASE_ALIGN: usize =
        max3(align_of::<Header<K, V>>(), align_of::<K>(), align_of::<V>());

    pub(super) fn layout(capacity: u32) -> Layout {
        layout_for_capacity::<K, V>(capacity)
    }

    /// Initialize a new offset hash map in caller-owned backing storage.
    ///
    /// # Safety
    ///
    /// `buf` must point to valid writable memory for `layout.total_size`
    /// bytes, aligned to `BASE_ALIGN`. The memory must outlive all maps derived
    /// from the returned offset handle.
    pub(super) unsafe fn init(buf: OffsetBuf, layout: Layout) -> Self {
        assert_ne!(size_of::<K>(), 0, "zero-sized keys are unsupported");
        assert_ne!(layout.capacity, 0, "zero-capacity map is unsupported");
        assert_eq!((buf.start() as usize) % Self::BASE_ALIGN, 0);

        let metadata_start = size_of::<Header<K, V>>();
        let metadata_buf = buf.rebase(metadata_start);
        let metadata = buf.member::<Metadata>(metadata_start);
        let metadata_ptr = metadata.ptr_mut(buf);
        let header = header_from_metadata::<K, V>(metadata_ptr);
        unsafe {
            // Safety: caller provided backing storage for the full layout.
            (*header).capacity = layout.capacity;
            (*header).size = 0;
            (*header).keys = metadata_buf.member::<MaybeUninit<K>>(layout.keys_start);
            (*header).values = metadata_buf.member::<MaybeUninit<V>>(layout.vals_start);
            std::ptr::write_bytes(metadata_ptr, 0, layout.capacity as usize);
        }

        Self {
            metadata,
            _marker: PhantomData,
        }
    }

    pub(super) fn map<'a>(&self, backing: &'a mut [u8]) -> Map<'a, K, V> {
        assert!(backing.len() > self.metadata.offset() as usize);
        assert_eq!(backing.as_ptr() as usize % Self::BASE_ALIGN, 0);
        Map {
            metadata: self.metadata.ptr_mut(backing),
            _marker: PhantomData,
        }
    }

    pub(super) fn map_ref<'a>(&self, backing: &'a [u8]) -> MapRef<'a, K, V> {
        assert!(backing.len() > self.metadata.offset() as usize);
        assert_eq!(backing.as_ptr() as usize % Self::BASE_ALIGN, 0);
        MapRef {
            metadata: self.metadata.ptr(backing),
            _marker: PhantomData,
        }
    }
}

#[derive(Debug)]
pub(super) struct Map<'a, K, V> {
    metadata: *mut Metadata,
    _marker: PhantomData<&'a mut (K, V)>,
}

impl<K, V> Map<'_, K, V>
where
    K: Copy + Eq + Hash,
    V: Copy + Default,
{
    pub(super) fn count(&self) -> u32 {
        self.header().size
    }

    pub(super) fn capacity(&self) -> u32 {
        self.header().capacity
    }

    pub(super) fn ensure_total_capacity(&self, new_size: u32) -> Result<(), Error> {
        if new_size > self.count() {
            self.grow_if_needed(new_size - self.count())?;
        }
        Ok(())
    }

    pub(super) fn ensure_unused_capacity(&self, additional_size: u32) -> Result<(), Error> {
        self.ensure_total_capacity(
            self.count()
                .checked_add(additional_size)
                .ok_or(Error::OutOfMemory)?,
        )
    }

    pub(super) fn clear_retaining_capacity(&mut self) {
        unsafe {
            // Safety: metadata points to `capacity` initialized metadata bytes.
            std::ptr::write_bytes(self.metadata, 0, self.capacity() as usize);
        }
        self.header_mut().size = 0;
    }

    pub(super) fn contains(&self, key: K) -> bool {
        self.index(key).is_some()
    }

    pub(super) fn get(&self, key: K) -> Option<V> {
        let idx = self.index(key)?;
        Some(unsafe {
            // Safety: `idx` is used, so the value slot is initialized.
            self.values().add(idx).read().assume_init()
        })
    }

    pub(super) fn get_mut(&mut self, key: K) -> Option<&mut V> {
        let idx = self.index(key)?;
        Some(unsafe {
            // Safety: `idx` is used, so the value slot is initialized. `&mut
            // self` guarantees exclusive map access.
            self.values_mut().add(idx).cast::<V>().as_mut().unwrap()
        })
    }

    pub(super) fn get_entry(&mut self, key: K) -> Option<Entry<'_, K, V>> {
        let idx = self.index(key)?;
        Some(self.entry_at(idx))
    }

    pub(super) fn put_no_clobber(&mut self, key: K, value: V) -> Result<(), Error> {
        assert!(!self.contains(key));
        self.grow_if_needed(1)?;
        self.put_assume_capacity_no_clobber(key, value);
        Ok(())
    }

    pub(super) fn put_assume_capacity(&mut self, key: K, value: V) {
        let result = self.get_or_put_assume_capacity(key);
        *result.value = value;
    }

    pub(super) fn put_assume_capacity_no_clobber(&mut self, key: K, value: V) {
        assert!(!self.contains(key));

        let hash = hash_key(&key);
        let mask = self.capacity() as usize - 1;
        let mut idx = hash as usize & mask;

        loop {
            if !self.metadata_at(idx).is_used() {
                unsafe {
                    // Safety: this slot is not used, so writing initializes it.
                    self.keys_mut().add(idx).write(MaybeUninit::new(key));
                    self.values_mut().add(idx).write(MaybeUninit::new(value));
                }
                self.metadata_at_mut(idx).fill(fingerprint(hash));
                self.header_mut().size += 1;
                return;
            }
            idx = (idx + 1) & mask;
        }
    }

    pub(super) fn fetch_put_assume_capacity(&mut self, key: K, value: V) -> Option<(K, V)> {
        let result = self.get_or_put_assume_capacity(key);
        let old = if result.found_existing {
            Some((*result.key, *result.value))
        } else {
            None
        };
        *result.value = value;
        old
    }

    pub(super) fn get_or_put(&mut self, key: K) -> Result<GetOrPutResult<'_, K, V>, Error> {
        self.grow_if_needed(1)
            .or_else(|err| if self.contains(key) { Ok(()) } else { Err(err) })?;
        Ok(self.get_or_put_assume_capacity(key))
    }

    pub(super) fn get_or_put_value(&mut self, key: K, value: V) -> Result<Entry<'_, K, V>, Error> {
        let found_existing;
        {
            let result = self.get_or_put(key)?;
            found_existing = result.found_existing;
            if !found_existing {
                *result.value = value;
            }
        }
        debug_assert!(found_existing || self.contains(key));
        Ok(self
            .get_entry(key)
            .expect("entry must exist after get_or_put"))
    }

    pub(super) fn get_or_put_assume_capacity(&mut self, key: K) -> GetOrPutResult<'_, K, V> {
        let hash = hash_key(&key);
        let mask = self.capacity() as usize - 1;
        let fp = fingerprint(hash);
        let mut limit = self.capacity();
        let mut idx = hash as usize & mask;
        let mut first_tombstone_idx = self.capacity() as usize;

        while !self.metadata_at(idx).is_free() && limit != 0 {
            let metadata = self.metadata_at(idx);
            if metadata.is_used() && metadata.fingerprint() == fp {
                let test_key = unsafe {
                    // Safety: metadata says this key slot is initialized.
                    self.keys().add(idx).read().assume_init()
                };
                if test_key == key {
                    return self.get_or_put_result_at(idx, true);
                }
            } else if first_tombstone_idx == self.capacity() as usize && metadata.is_tombstone() {
                first_tombstone_idx = idx;
            }

            limit -= 1;
            idx = (idx + 1) & mask;
        }

        if first_tombstone_idx < self.capacity() as usize {
            idx = first_tombstone_idx;
        }

        let default_value = V::default();
        unsafe {
            // Safety: this slot is free/tombstone, so writing initializes it.
            self.keys_mut().add(idx).write(MaybeUninit::new(key));
            self.values_mut()
                .add(idx)
                .write(MaybeUninit::new(default_value));
        }
        self.metadata_at_mut(idx).fill(fp);
        self.header_mut().size += 1;
        self.get_or_put_result_at(idx, false)
    }

    pub(super) fn remove(&mut self, key: K) -> bool {
        let Some(idx) = self.index(key) else {
            return false;
        };
        self.remove_by_index(idx);
        true
    }

    pub(super) fn remove_by_ptr(&mut self, key: *const K) {
        assert_ne!(size_of::<K>(), 0, "zero-sized keys are unsupported");
        let keys_start = self.keys() as *const K as usize;
        let key_addr = key as usize;
        assert!(key_addr >= keys_start);
        assert_eq!((key_addr - keys_start) % size_of::<K>(), 0);
        let idx = (key_addr - keys_start) / size_of::<K>();
        assert!(idx < self.capacity() as usize);
        self.remove_by_index(idx);
    }

    pub(super) fn fetch_remove(&mut self, key: K) -> Option<(K, V)> {
        let idx = self.index(key)?;
        let result = unsafe {
            // Safety: `idx` is used, so both slots are initialized.
            (
                self.keys().add(idx).read().assume_init(),
                self.values().add(idx).read().assume_init(),
            )
        };
        self.remove_by_index(idx);
        Some(result)
    }

    pub(super) fn iter(&self) -> Iter<'_, '_, K, V> {
        Iter {
            map: self,
            index: 0,
        }
    }

    fn index(&self, key: K) -> Option<usize> {
        if self.count() == 0 {
            return None;
        }

        let hash = hash_key(&key);
        let mask = self.capacity() as usize - 1;
        let fp = fingerprint(hash);
        let mut limit = self.capacity();
        let mut idx = hash as usize & mask;

        while !self.metadata_at(idx).is_free() && limit != 0 {
            let metadata = self.metadata_at(idx);
            if metadata.is_used() && metadata.fingerprint() == fp {
                let test_key = unsafe {
                    // Safety: metadata says this key slot is initialized.
                    self.keys().add(idx).read().assume_init()
                };
                if test_key == key {
                    return Some(idx);
                }
            }

            limit -= 1;
            idx = (idx + 1) & mask;
        }

        None
    }

    fn get_or_put_result_at(
        &mut self,
        idx: usize,
        found_existing: bool,
    ) -> GetOrPutResult<'_, K, V> {
        unsafe {
            // Safety: caller selected a slot that was just initialized or
            // already marked used.
            GetOrPutResult {
                key: self.keys_mut().add(idx).cast::<K>().as_mut().unwrap(),
                value: self.values_mut().add(idx).cast::<V>().as_mut().unwrap(),
                found_existing,
            }
        }
    }

    fn entry_at(&mut self, idx: usize) -> Entry<'_, K, V> {
        unsafe {
            // Safety: caller only asks for entries at used indices.
            Entry {
                key: self.keys_mut().add(idx).cast::<K>().as_mut().unwrap(),
                value: self.values_mut().add(idx).cast::<V>().as_mut().unwrap(),
            }
        }
    }

    fn remove_by_index(&mut self, idx: usize) {
        assert!(self.metadata_at(idx).is_used());
        self.metadata_at_mut(idx).remove();
        self.header_mut().size -= 1;
    }

    fn grow_if_needed(&self, new_count: u32) -> Result<(), Error> {
        let available = self.capacity() - self.count();
        if new_count > available {
            Err(Error::OutOfMemory)
        } else {
            Ok(())
        }
    }

    fn header(&self) -> &Header<K, V> {
        unsafe {
            // Safety: metadata points immediately after the initialized header.
            &*header_from_metadata::<K, V>(self.metadata)
        }
    }

    fn header_mut(&mut self) -> &mut Header<K, V> {
        unsafe {
            // Safety: `&mut self` guarantees exclusive access to the header.
            &mut *header_from_metadata::<K, V>(self.metadata)
        }
    }

    fn metadata_at(&self, idx: usize) -> Metadata {
        assert!(idx < self.capacity() as usize);
        unsafe {
            // Safety: idx is within the metadata array.
            *self.metadata.add(idx)
        }
    }

    fn metadata_at_mut(&mut self, idx: usize) -> &mut Metadata {
        assert!(idx < self.capacity() as usize);
        unsafe {
            // Safety: idx is within the metadata array and `&mut self`
            // guarantees exclusive access.
            &mut *self.metadata.add(idx)
        }
    }

    fn keys(&self) -> *const MaybeUninit<K> {
        self.header().keys.ptr(self.metadata)
    }

    fn keys_mut(&mut self) -> *mut MaybeUninit<K> {
        self.header().keys.ptr_mut(self.metadata)
    }

    fn values(&self) -> *const MaybeUninit<V> {
        self.header().values.ptr(self.metadata)
    }

    fn values_mut(&mut self) -> *mut MaybeUninit<V> {
        self.header().values.ptr_mut(self.metadata)
    }
}

#[derive(Debug)]
pub(super) struct MapRef<'a, K, V> {
    metadata: *const Metadata,
    _marker: PhantomData<&'a (K, V)>,
}

impl<K, V> MapRef<'_, K, V>
where
    K: Copy + Eq + Hash,
    V: Copy + Default,
{
    pub(super) fn count(&self) -> u32 {
        self.header().size
    }

    pub(super) fn capacity(&self) -> u32 {
        self.header().capacity
    }

    pub(super) fn contains(&self, key: K) -> bool {
        self.index(key).is_some()
    }

    pub(super) fn get(&self, key: K) -> Option<V> {
        let idx = self.index(key)?;
        Some(unsafe {
            // Safety: `idx` is used, so the value slot is initialized.
            self.values().add(idx).read().assume_init()
        })
    }

    pub(super) fn iter(&self) -> IterRef<'_, '_, K, V> {
        IterRef {
            map: self,
            index: 0,
        }
    }

    fn index(&self, key: K) -> Option<usize> {
        if self.count() == 0 {
            return None;
        }

        let hash = hash_key(&key);
        let mask = self.capacity() as usize - 1;
        let fp = fingerprint(hash);
        let mut limit = self.capacity();
        let mut idx = hash as usize & mask;

        while !self.metadata_at(idx).is_free() && limit != 0 {
            let metadata = self.metadata_at(idx);
            if metadata.is_used() && metadata.fingerprint() == fp {
                let test_key = unsafe {
                    // Safety: metadata says this key slot is initialized.
                    self.keys().add(idx).read().assume_init()
                };
                if test_key == key {
                    return Some(idx);
                }
            }

            limit -= 1;
            idx = (idx + 1) & mask;
        }

        None
    }

    fn header(&self) -> &Header<K, V> {
        unsafe {
            // Safety: metadata points immediately after the initialized header.
            &*header_from_metadata::<K, V>(self.metadata.cast_mut())
        }
    }

    fn metadata_at(&self, idx: usize) -> Metadata {
        assert!(idx < self.capacity() as usize);
        unsafe {
            // Safety: idx is within the metadata array.
            *self.metadata.add(idx)
        }
    }

    fn keys(&self) -> *const MaybeUninit<K> {
        self.header().keys.ptr(self.metadata)
    }

    fn values(&self) -> *const MaybeUninit<V> {
        self.header().values.ptr(self.metadata)
    }
}

pub(super) struct Entry<'a, K, V> {
    pub(super) key: &'a mut K,
    pub(super) value: &'a mut V,
}

pub(super) struct GetOrPutResult<'a, K, V> {
    pub(super) key: &'a mut K,
    pub(super) value: &'a mut V,
    pub(super) found_existing: bool,
}

pub(super) struct Iter<'a, 'b, K, V> {
    map: &'a Map<'b, K, V>,
    index: u32,
}

impl<K, V> Iterator for Iter<'_, '_, K, V>
where
    K: Copy + Eq + Hash,
    V: Copy + Default,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.map.capacity() {
            let idx = self.index as usize;
            self.index += 1;
            if self.map.metadata_at(idx).is_used() {
                return Some(unsafe {
                    // Safety: metadata says both slots are initialized.
                    (
                        self.map.keys().add(idx).read().assume_init(),
                        self.map.values().add(idx).read().assume_init(),
                    )
                });
            }
        }
        None
    }
}

pub(super) struct IterRef<'a, 'b, K, V> {
    map: &'a MapRef<'b, K, V>,
    index: u32,
}

impl<K, V> Iterator for IterRef<'_, '_, K, V>
where
    K: Copy + Eq + Hash,
    V: Copy + Default,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.map.capacity() {
            let idx = self.index as usize;
            self.index += 1;
            if self.map.metadata_at(idx).is_used() {
                return Some(unsafe {
                    // Safety: metadata says both slots are initialized.
                    (
                        self.map.keys().add(idx).read().assume_init(),
                        self.map.values().add(idx).read().assume_init(),
                    )
                });
            }
        }
        None
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Header<K, V> {
    values: Offset<MaybeUninit<V>>,
    keys: Offset<MaybeUninit<K>>,
    capacity: u32,
    size: u32,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Metadata(u8);

impl Metadata {
    const FREE: u8 = 0;
    const TOMBSTONE: u8 = 1;
    const USED_BIT: u8 = 0b1000_0000;
    const FINGERPRINT_MASK: u8 = 0b0111_1111;

    const fn is_used(self) -> bool {
        self.0 & Self::USED_BIT != 0
    }

    const fn is_tombstone(self) -> bool {
        self.0 == Self::TOMBSTONE
    }

    const fn is_free(self) -> bool {
        self.0 == Self::FREE
    }

    const fn fingerprint(self) -> u8 {
        self.0 & Self::FINGERPRINT_MASK
    }

    fn fill(&mut self, fingerprint: u8) {
        assert!(fingerprint <= Self::FINGERPRINT_MASK);
        self.0 = Self::USED_BIT | fingerprint;
    }

    fn remove(&mut self) {
        self.0 = Self::TOMBSTONE;
    }
}

pub(super) fn layout_for_capacity<K, V>(capacity: u32) -> Layout {
    assert_ne!(size_of::<K>(), 0, "zero-sized keys are unsupported");
    assert!(capacity == 0 || capacity.is_power_of_two());

    let cap = capacity as usize;
    let meta_start = size_of::<Header<K, V>>();
    let meta_end = meta_start + cap * size_of::<Metadata>();
    let keys_start = align_forward(meta_end, align_of::<K>());
    let keys_end = keys_start + cap * size_of::<K>();
    let vals_start = align_forward(keys_end, align_of::<V>());
    let vals_end = vals_start + cap * size_of::<V>();
    let total_size = align_forward(
        vals_end,
        max3(align_of::<Header<K, V>>(), align_of::<K>(), align_of::<V>()),
    );

    Layout {
        total_size,
        keys_start: keys_start - meta_start,
        vals_start: vals_start - meta_start,
        capacity,
    }
}

fn header_from_metadata<K, V>(metadata: *mut Metadata) -> *mut Header<K, V> {
    unsafe {
        // Safety: callers only pass metadata pointers created by `init`, where
        // the header is laid out immediately before metadata.
        metadata.cast::<u8>().sub(size_of::<Header<K, V>>()).cast()
    }
}

fn hash_key<K: Hash>(key: &K) -> u64 {
    let mut hasher = DeterministicHasher::default();
    key.hash(&mut hasher);
    hasher.finish()
}

fn fingerprint(hash: u64) -> u8 {
    (hash >> 57) as u8
}

struct DeterministicHasher(u64);

impl Default for DeterministicHasher {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for DeterministicHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= *byte as u64;
            self.0 = self.0.wrapping_mul(0x1000_0000_01b3);
        }
    }

    fn write_u8(&mut self, i: u8) {
        self.write(&i.to_ne_bytes());
    }

    fn write_u16(&mut self, i: u16) {
        self.write(&i.to_ne_bytes());
    }

    fn write_u32(&mut self, i: u32) {
        self.write(&i.to_ne_bytes());
    }

    fn write_u64(&mut self, i: u64) {
        self.write(&i.to_ne_bytes());
    }

    fn write_usize(&mut self, i: usize) {
        self.write(&i.to_ne_bytes());
    }

    fn write_i8(&mut self, i: i8) {
        self.write(&i.to_ne_bytes());
    }

    fn write_i16(&mut self, i: i16) {
        self.write(&i.to_ne_bytes());
    }

    fn write_i32(&mut self, i: i32) {
        self.write(&i.to_ne_bytes());
    }

    fn write_i64(&mut self, i: i64) {
        self.write(&i.to_ne_bytes());
    }

    fn write_isize(&mut self, i: isize) {
        self.write(&i.to_ne_bytes());
    }
}

const fn max3(a: usize, b: usize, c: usize) -> usize {
    let ab = if a > b { a } else { b };
    if ab > c {
        ab
    } else {
        c
    }
}

fn align_forward(value: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (value + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Backing<K, V> {
        bytes: Vec<u8>,
        base_offset: usize,
        map: OffsetHashMap<K, V>,
        layout: Layout,
    }

    impl<K, V> Backing<K, V> {
        fn backing_mut(&mut self) -> &mut [u8] {
            let start = self.base_offset;
            let end = start + self.layout.total_size;
            &mut self.bytes[start..end]
        }
    }

    impl<K, V> Backing<K, V>
    where
        K: Copy + Eq + Hash,
        V: Copy + Default,
    {
        fn map(&mut self) -> Map<'_, K, V> {
            let map = self.map;
            map.map(self.backing_mut())
        }
    }

    fn backing<K, V>(capacity: u32) -> Backing<K, V>
    where
        K: Copy + Eq + Hash,
        V: Copy + Default,
    {
        let layout = OffsetHashMap::<K, V>::layout(capacity);
        let mut bytes = vec![0; layout.total_size + OffsetHashMap::<K, V>::BASE_ALIGN];
        let align_offset = bytes
            .as_ptr()
            .align_offset(OffsetHashMap::<K, V>::BASE_ALIGN);
        let base = unsafe {
            // Safety: align_offset is within the overallocated buffer.
            bytes.as_mut_ptr().add(align_offset)
        };
        let map = unsafe {
            // Safety: buffer is aligned and sized according to layout.
            OffsetHashMap::<K, V>::init(OffsetBuf::init(base), layout)
        };
        Backing {
            bytes,
            base_offset: align_offset,
            map,
            layout,
        }
    }

    #[test]
    fn layout_values() {
        assert_eq!(size_of::<Metadata>(), 1);
        assert_eq!(align_of::<Metadata>(), 1);
        assert_eq!(size_of::<Header<Offset<u8>, u16>>(), 16);
        assert_eq!(align_of::<Header<Offset<u8>, u16>>(), 4);

        assert_eq!(
            OffsetHashMap::<u32, u32>::layout(16),
            Layout {
                total_size: 160,
                keys_start: 16,
                vals_start: 80,
                capacity: 16,
            }
        );
    }

    #[test]
    fn large_layout_does_not_overflow() {
        let large_cap = 1 << 30;
        let layout = OffsetHashMap::<u64, u64>::layout(large_cap);
        let min_expected = large_cap as usize * (size_of::<u64>() + size_of::<u64>());

        assert!(layout.total_size >= min_expected);
        assert!(layout.keys_start > 0);
        assert!(layout.vals_start > layout.keys_start);
    }

    #[test]
    #[should_panic(expected = "zero-sized keys are unsupported")]
    fn zero_sized_keys_are_rejected() {
        let _ = OffsetHashMap::<(), u32>::layout(8);
    }

    #[test]
    fn basic_usage() {
        let mut backing = backing::<u32, u32>(16);
        let mut map = backing.map();

        let mut total = 0;
        for i in 0..5 {
            map.put_no_clobber(i, i).unwrap();
            total += i;
        }

        let iter_total: u32 = map.iter().map(|(key, _)| key).sum();
        assert_eq!(iter_total, total);

        for i in 0..5 {
            assert_eq!(map.get(i), Some(i));
        }
    }

    #[test]
    fn ensure_total_capacity() {
        let mut backing = backing::<i32, i32>(32);
        let mut map = backing.map();

        let initial_capacity = map.capacity();
        assert!(initial_capacity >= 20);
        for i in 0..20 {
            assert!(map.fetch_put_assume_capacity(i, i + 10).is_none());
        }
        assert_eq!(initial_capacity, map.capacity());
    }

    #[test]
    fn ensure_unused_capacity_with_tombstones() {
        let mut backing = backing::<i32, i32>(32);
        let mut map = backing.map();

        for i in 0..100 {
            map.ensure_unused_capacity(1).unwrap();
            map.put_assume_capacity(i, i);
            assert!(map.remove(i));
        }
    }

    #[test]
    fn clear_retaining_capacity() {
        let mut backing = backing::<u32, u32>(16);
        let mut map = backing.map();

        map.clear_retaining_capacity();
        map.put_no_clobber(1, 1).unwrap();
        assert_eq!(map.get(1), Some(1));
        assert_eq!(map.count(), 1);

        map.clear_retaining_capacity();
        map.put_assume_capacity(1, 1);
        assert_eq!(map.get(1), Some(1));
        assert_eq!(map.count(), 1);
        assert_eq!(map.capacity(), 16);

        map.clear_retaining_capacity();
        map.clear_retaining_capacity();
        assert_eq!(map.count(), 0);
        assert!(!map.contains(1));
    }

    #[test]
    fn ensure_total_capacity_with_existing_elements() {
        let mut backing = backing::<u32, u32>(8);
        let mut map = backing.map();

        map.put_no_clobber(0, 0).unwrap();
        assert_eq!(map.count(), 1);
        assert_eq!(map.capacity(), 8);
        assert_eq!(map.ensure_total_capacity(65), Err(Error::OutOfMemory));
        assert_eq!(map.count(), 1);
        assert_eq!(map.capacity(), 8);
    }

    #[test]
    fn remove() {
        let mut backing = backing::<u32, u32>(16);
        let mut map = backing.map();

        for i in 0..10 {
            map.put_no_clobber(i, i + 10).unwrap();
        }
        assert!(map.remove(3));
        assert_eq!(map.get(3), None);
        assert_eq!(map.count(), 9);
        assert!(!map.remove(3));
        for i in [0, 1, 2, 4, 5, 6, 7, 8, 9] {
            assert_eq!(map.get(i), Some(i + 10));
        }
    }

    #[test]
    fn reverse_removes() {
        let mut backing = backing::<u32, u32>(32);
        let mut map = backing.map();

        for i in 0..20 {
            map.put_no_clobber(i, i).unwrap();
        }
        for i in (0..20).rev() {
            assert!(map.remove(i));
        }
        assert_eq!(map.count(), 0);
    }

    #[test]
    fn multiple_removes_on_same_metadata() {
        let mut backing = backing::<u32, u32>(8);
        let mut map = backing.map();

        map.put_no_clobber(0, 0).unwrap();
        assert!(map.remove(0));
        assert!(!map.remove(0));
        map.put_no_clobber(8, 8).unwrap();
        assert_eq!(map.get(8), Some(8));
    }

    #[test]
    fn put_and_remove_loop_in_random_order() {
        let mut backing = backing::<u32, u32>(64);
        let mut map = backing.map();
        let order = [
            13, 4, 29, 7, 41, 2, 31, 19, 23, 5, 37, 11, 17, 3, 43, 47, 53, 59, 61, 1,
        ];

        for key in order {
            map.put_no_clobber(key, key + 1).unwrap();
        }
        for key in order.into_iter().rev() {
            assert_eq!(map.fetch_remove(key), Some((key, key + 1)));
        }
        assert_eq!(map.count(), 0);
    }

    #[test]
    fn put() {
        let mut backing = backing::<u32, u32>(16);
        let mut map = backing.map();

        for i in 0..10 {
            map.put_no_clobber(i, i).unwrap();
        }
        for i in 0..10 {
            map.put_assume_capacity(i, i + 100);
        }
        for i in 0..10 {
            assert_eq!(map.get(i), Some(i + 100));
        }
    }

    #[test]
    fn put_full_load() {
        let mut backing = backing::<u32, u32>(8);
        let mut map = backing.map();

        for i in 0..8 {
            map.put_no_clobber(i, i).unwrap();
        }
        assert_eq!(map.count(), 8);
        assert_eq!(map.put_no_clobber(99, 99), Err(Error::OutOfMemory));
        for i in 0..8 {
            assert_eq!(map.get(i), Some(i));
        }
    }

    #[test]
    fn put_assume_capacity() {
        let mut backing = backing::<u32, u32>(16);
        let mut map = backing.map();

        for i in 0..8 {
            map.put_assume_capacity_no_clobber(i, i);
        }
        for i in 0..8 {
            map.put_assume_capacity(i, 1);
        }
        for i in 0..8 {
            assert_eq!(map.get(i), Some(1));
        }
    }

    #[test]
    fn repeat_put_assume_capacity_remove() {
        let mut backing = backing::<u32, u32>(64);
        let mut map = backing.map();
        let limit = 32;

        for i in 0..limit {
            map.put_assume_capacity_no_clobber(i, i);
        }
        for _ in 0..8 {
            for i in 0..limit {
                assert!(map.remove(i));
            }
            for i in 0..limit {
                map.put_assume_capacity_no_clobber(i, i);
            }
            for i in 0..limit {
                map.put_assume_capacity(i, i + 1);
            }
        }
        for i in 0..limit {
            assert_eq!(map.get(i), Some(i + 1));
        }
    }

    #[test]
    fn get_or_put() {
        let mut backing = backing::<u32, u32>(16);
        let mut map = backing.map();

        {
            let result = map.get_or_put(5).unwrap();
            assert!(!result.found_existing);
            *result.value = 55;
        }
        {
            let result = map.get_or_put(5).unwrap();
            assert!(result.found_existing);
            assert_eq!(*result.key, 5);
            assert_eq!(*result.value, 55);
        }
        {
            let entry = map.get_or_put_value(6, 66).unwrap();
            assert_eq!(*entry.key, 6);
            assert_eq!(*entry.value, 66);
        }
    }

    #[test]
    fn panicking_default_does_not_mark_slot_used() {
        #[derive(Clone, Copy)]
        struct PanicDefault;

        impl Default for PanicDefault {
            fn default() -> Self {
                panic!("default failed");
            }
        }

        let mut backing = backing::<u32, PanicDefault>(16);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut map = backing.map();
            let _ = map.get_or_put_assume_capacity(5);
        }));

        assert!(result.is_err());

        let map = backing.map();
        assert_eq!(map.count(), 0);
        assert!(!map.contains(5));
        assert_eq!(map.iter().count(), 0);
    }

    #[test]
    fn ensure_unused_capacity() {
        let mut backing = backing::<u32, u32>(32);
        let map = backing.map();

        map.ensure_unused_capacity(32).unwrap();
        assert_eq!(map.ensure_unused_capacity(33), Err(Error::OutOfMemory));
    }

    #[test]
    fn remove_by_ptr() {
        let mut backing = backing::<u32, u32>(16);
        let mut map = backing.map();

        for i in 0..10 {
            map.put_no_clobber(i, i).unwrap();
        }
        let key_ptr = {
            let entry = map.get_entry(5).unwrap();
            entry.key as *const u32
        };
        map.remove_by_ptr(key_ptr);
        assert_eq!(map.get(5), None);
        assert_eq!(map.count(), 9);
    }

    #[test]
    fn repeat_fetch_remove() {
        let mut backing = backing::<u32, ()>(8);
        let mut map = backing.map();

        for i in 0..4 {
            map.put_assume_capacity(i, ());
        }
        for _ in 0..4 {
            assert_eq!(map.fetch_remove(3), Some((3, ())));
            map.put_assume_capacity(3, ());
        }
    }

    #[test]
    fn offset_map_remake() {
        let mut backing = backing::<u32, u32>(16);
        {
            let mut map = backing.map();
            map.put_no_clobber(5, 5).unwrap();
        }
        {
            let map = backing.map();
            assert_eq!(map.get(5), Some(5));
        }
    }

    #[test]
    fn offset_map_rebase_after_copy() {
        let mut backing = backing::<u32, u32>(16);
        let layout = backing.layout;
        {
            let mut map = backing.map();
            map.put_no_clobber(5, 50).unwrap();
            map.put_no_clobber(7, 70).unwrap();
        }

        let base = backing.base_offset;
        let bytes = backing.bytes[base..base + layout.total_size].to_vec();
        let mut copied = vec![0; layout.total_size + OffsetHashMap::<u32, u32>::BASE_ALIGN];
        let copied_offset = copied
            .as_ptr()
            .align_offset(OffsetHashMap::<u32, u32>::BASE_ALIGN);
        copied[copied_offset..copied_offset + layout.total_size].copy_from_slice(&bytes);

        let copied_backing = &mut copied[copied_offset..copied_offset + layout.total_size];
        let mut map = backing.map.map(copied_backing);
        assert_eq!(map.get(5), Some(50));
        assert_eq!(map.get(7), Some(70));
        assert!(map.remove(5));
        assert_eq!(map.get(5), None);
    }
}
