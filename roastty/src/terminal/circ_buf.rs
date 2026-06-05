//! A fixed-capacity circular buffer (port of the core of upstream `datastruct/circ_buf`).

/// Returned by `append` when the buffer is full (upstream returns `error.OutOfMemory`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Full;

/// Traversal direction for a `CircBuf` iterator (upstream `Iterator.Direction`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Forward,
    Reverse,
}

/// A read-only forward/reverse iterator over a `CircBuf` (upstream `CircBuf.Iterator`).
pub(crate) struct Iter<'a, T: Copy> {
    buf: &'a CircBuf<T>,
    idx: usize,
    direction: Direction,
}

impl<'a, T: Copy> Iter<'a, T> {
    /// The next element, or `None` once the iterator is past the end (upstream `next`).
    pub(crate) fn next(&mut self) -> Option<&'a T> {
        if self.idx >= self.buf.len() {
            return None;
        }
        let tail_idx = match self.direction {
            Direction::Forward => self.idx,
            Direction::Reverse => self.buf.len() - self.idx - 1,
        };
        let storage_idx = (self.buf.tail + tail_idx) % self.buf.capacity();
        self.idx += 1;
        Some(&self.buf.storage[storage_idx])
    }

    /// Move the logical index by a signed amount, saturating at the bounds (upstream
    /// `seekBy`).
    pub(crate) fn seek_by(&mut self, amount: isize) {
        if amount > 0 {
            self.idx = self.idx.saturating_add(amount as usize);
        } else {
            self.idx = self.idx.saturating_sub(amount.unsigned_abs());
        }
    }

    /// Reset back to the first element (upstream `reset`).
    pub(crate) fn reset(&mut self) {
        self.idx = 0;
    }
}

/// A fixed-capacity ring buffer of `T` (upstream `datastruct.CircBuf`). `head` is the next
/// write index, `tail` the oldest; `full` disambiguates `head == tail`.
pub(crate) struct CircBuf<T: Copy> {
    storage: Vec<T>,
    head: usize,
    tail: usize,
    full: bool,
    default: T,
}

impl<T: Copy> CircBuf<T> {
    /// Allocate a ring of `size` elements, filled with `default` (upstream `init`).
    pub(crate) fn new(size: usize, default: T) -> Self {
        Self {
            storage: vec![default; size],
            head: 0,
            tail: 0,
            full: size == 0,
            default,
        }
    }

    /// Append a value; `Err(Full)` if the buffer is full (upstream `append`).
    pub(crate) fn append(&mut self, v: T) -> Result<(), Full> {
        if self.full {
            return Err(Full);
        }
        self.append_assume_capacity(v);
        Ok(())
    }

    /// Append a value, assuming there is capacity (upstream `appendAssumeCapacity`).
    pub(crate) fn append_assume_capacity(&mut self, v: T) {
        assert!(!self.full, "append to a full CircBuf");
        self.storage[self.head] = v;
        self.head += 1;
        if self.head >= self.storage.len() {
            self.head = 0;
        }
        self.full = self.head == self.tail;
    }

    /// Reset to empty (upstream `clear`).
    pub(crate) fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.full = false;
    }

    /// Whether the buffer holds no elements (upstream `empty`).
    pub(crate) fn is_empty(&self) -> bool {
        !self.full && self.head == self.tail
    }

    /// The total allocated capacity (upstream `capacity`).
    pub(crate) fn capacity(&self) -> usize {
        self.storage.len()
    }

    /// The number of used elements (upstream `len`).
    pub(crate) fn len(&self) -> usize {
        if self.full {
            return self.storage.len();
        }
        if self.head >= self.tail {
            self.head - self.tail
        } else {
            self.storage.len() - (self.tail - self.head)
        }
    }

    /// Delete the oldest `n` values, resetting their slots to `default` (upstream
    /// `deleteOldest`). Deletes everything if `n` exceeds the used length.
    pub(crate) fn delete_oldest(&mut self, n: usize) {
        assert!(n <= self.storage.len());
        if n == 0 {
            return;
        }
        let count = n.min(self.len());
        let cap = self.storage.len();
        for i in 0..count {
            let idx = (self.tail + i) % cap;
            self.storage[idx] = self.default;
        }
        self.tail = (self.tail + count) % cap;
        self.full = false;
    }

    /// The oldest value, or `None` if there are no elements (upstream `first`).
    pub(crate) fn first(&self) -> Option<&T> {
        // Guard on `len() == 0`, not `is_empty()`: a zero-capacity buffer has `full == true`
        // (so `is_empty()` is false) yet `len()` is 0, and upstream's iterator returns `null`.
        if self.len() == 0 {
            return None;
        }
        Some(&self.storage[self.tail])
    }

    /// The newest value, or `None` if there are no elements (upstream `last`).
    pub(crate) fn last(&self) -> Option<&T> {
        if self.len() == 0 {
            return None;
        }
        let cap = self.storage.len();
        Some(&self.storage[(self.head + cap - 1) % cap])
    }

    /// Iterate over the logical elements, oldest-first (`Forward`) or newest-first
    /// (`Reverse`) (upstream `iterator`).
    pub(crate) fn iterator(&self, direction: Direction) -> Iter<'_, T> {
        Iter {
            buf: self,
            idx: 0,
            direction,
        }
    }

    /// The (up to two) contiguous storage spans covering the logical range `[offset,
    /// offset+slice_len)` (upstream `getPtrSlice`). If the range extends past the current
    /// length, the space is claimed by advancing `head`. The second span is empty when the
    /// range does not wrap.
    pub(crate) fn get_ptr_slice(
        &mut self,
        offset: usize,
        slice_len: usize,
    ) -> (&mut [T], &mut [T]) {
        if slice_len == 0 {
            // Two disjoint empty spans (deriving them from a split avoids two `&mut []`
            // literals, which would be two mutable borrows of the same promoted empty array).
            let (a, b) = self.storage.split_at_mut(0);
            return (a, &mut b[..0]);
        }
        assert!(offset + slice_len <= self.capacity());
        let end_offset = offset + slice_len;
        let cur_len = self.len();
        if end_offset > cur_len {
            self.advance(end_offset - cur_len);
        }
        let start_idx = self.storage_offset(offset);
        let end_idx = self.storage_offset(end_offset - 1);
        if end_idx >= start_idx {
            // Non-wrap: one span `storage[start_idx..=end_idx]`, empty second. Split at
            // `end_idx + 1` so the second (empty) span is a disjoint sub-slice.
            let (front, back) = self.storage.split_at_mut(end_idx + 1);
            (&mut front[start_idx..], &mut back[..0])
        } else {
            // Wrap: span0 = storage[start_idx..], span1 = storage[0..=end_idx]; disjoint
            // because end_idx < start_idx.
            let (left, right) = self.storage.split_at_mut(start_idx);
            (right, &mut left[..=end_idx])
        }
    }

    /// Advance `head` by `amount`, claiming free space (upstream `advance`).
    fn advance(&mut self, amount: usize) {
        assert!(amount <= self.storage.len() - self.len());
        self.head += amount;
        if self.head >= self.storage.len() {
            self.head -= self.storage.len();
        }
        if self.full {
            self.tail = self.head;
        }
        self.full = self.head == self.tail;
    }

    /// Map a logical offset (from the oldest) to a storage index (upstream `storageOffset`).
    fn storage_offset(&self, offset: usize) -> usize {
        assert!(offset < self.storage.len());
        let fits = self.tail + offset;
        if fits < self.storage.len() {
            fits
        } else {
            fits - self.storage.len()
        }
    }

    /// Append a slice, assuming there is capacity (upstream `appendSliceAssumeCapacity`).
    pub(crate) fn append_slice_assume_capacity(&mut self, slice: &[T]) {
        let len = self.len();
        let (span0, span1) = self.get_ptr_slice(len, slice.len());
        let first = span0.len();
        span0.copy_from_slice(&slice[..first]);
        span1.copy_from_slice(&slice[first..]);
    }

    /// Ensure there is room to append `amount` more items, growing if needed (upstream
    /// `ensureUnusedCapacity`).
    pub(crate) fn ensure_unused_capacity(&mut self, amount: usize) {
        let new_cap = self.len() + amount;
        if new_cap <= self.capacity() {
            return;
        }
        self.resize(new_cap);
    }

    /// Resize the buffer to `size` (larger or smaller). New slots (when growing) are `default`
    /// (upstream `resize`).
    pub(crate) fn resize(&mut self, size: usize) {
        // Rotate the data to be zero-aligned so the reallocation's new space is contiguous.
        self.rotate_to_zero();

        let prev_len = self.len();
        let prev_cap = self.storage.len();
        // `Vec::resize` both grows (filling new slots with `default`) and shrinks (truncating)
        // — the equivalent of `realloc` + the grow-time `@memset` to `default`.
        self.storage.resize(size, self.default);

        if size > prev_cap && self.full {
            // We grew a full buffer: the data now occupies `[0, prev_len)`, free space after.
            self.head = prev_len;
            self.full = false;
        }
    }

    /// Rotate the data so the oldest element is at index 0 (upstream `rotateToZero`).
    fn rotate_to_zero(&mut self) {
        if self.tail == 0 {
            return;
        }
        self.storage.rotate_left(self.tail);
        self.head = self.len() % self.storage.len();
        self.tail = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_until_full() {
        let mut buf = CircBuf::new(3, 0u8);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.capacity(), 3);

        assert_eq!(buf.append(1), Ok(()));
        assert_eq!(buf.append(2), Ok(()));
        assert_eq!(buf.append(3), Ok(()));

        assert!(!buf.is_empty());
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.append(4), Err(Full));

        assert_eq!(buf.first(), Some(&1));
        assert_eq!(buf.last(), Some(&3));
    }

    #[test]
    fn delete_oldest_and_wrap() {
        let mut buf = CircBuf::new(3, 0u8);
        buf.append_assume_capacity(1);
        buf.append_assume_capacity(2);
        buf.append_assume_capacity(3);

        // Drop the oldest; the tail advances.
        buf.delete_oldest(1);
        assert_eq!(buf.len(), 2);
        assert_eq!(buf.first(), Some(&2));
        assert_eq!(buf.last(), Some(&3));

        // Appending now wraps the head to index 0.
        assert_eq!(buf.append(4), Ok(()));
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.first(), Some(&2));
        assert_eq!(buf.last(), Some(&4));

        // Deleting more than the length empties the buffer.
        buf.delete_oldest(3);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.first(), None);
        assert_eq!(buf.last(), None);
    }

    #[test]
    fn clear_resets_and_reuses() {
        let mut buf = CircBuf::new(2, 0u8);
        buf.append_assume_capacity(9);
        buf.append_assume_capacity(8);

        buf.clear();
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);

        assert_eq!(buf.append(7), Ok(()));
        assert_eq!(buf.first(), Some(&7));
    }

    /// Collect a whole iterator's elements (by value) for assertions.
    fn collect(mut it: Iter<'_, u8>) -> Vec<u8> {
        let mut out = Vec::new();
        while let Some(v) = it.next() {
            out.push(*v);
        }
        out
    }

    #[test]
    fn iterates_forward_and_reverse() {
        let mut buf = CircBuf::new(3, 0u8);
        buf.append_assume_capacity(1);
        buf.append_assume_capacity(2);
        buf.append_assume_capacity(3);

        assert_eq!(collect(buf.iterator(Direction::Forward)), vec![1, 2, 3]);
        assert_eq!(collect(buf.iterator(Direction::Reverse)), vec![3, 2, 1]);
    }

    #[test]
    fn iterates_across_the_wrap() {
        let mut buf = CircBuf::new(3, 0u8);
        buf.append_assume_capacity(1);
        buf.append_assume_capacity(2);
        buf.append_assume_capacity(3);
        buf.delete_oldest(1); // tail advances
        buf.append_assume_capacity(4); // head wraps to 0

        assert_eq!(collect(buf.iterator(Direction::Forward)), vec![2, 3, 4]);
        assert_eq!(collect(buf.iterator(Direction::Reverse)), vec![4, 3, 2]);
    }

    #[test]
    fn seek_by_and_reset() {
        let mut buf = CircBuf::new(3, 0u8);
        buf.append_assume_capacity(1);
        buf.append_assume_capacity(2);
        buf.append_assume_capacity(3);

        let mut it = buf.iterator(Direction::Forward);
        it.seek_by(1); // skip the first element
        assert_eq!(it.next(), Some(&2));

        // A large negative seek saturates to index 0.
        it.seek_by(-100);
        assert_eq!(it.next(), Some(&1));

        // reset returns to the start.
        let _ = it.next(); // advance away from 0
        it.reset();
        assert_eq!(it.next(), Some(&1));
    }

    #[test]
    fn iterating_empty_yields_none() {
        let buf = CircBuf::new(3, 0u8);
        let mut it = buf.iterator(Direction::Forward);
        assert_eq!(it.next(), None);
    }

    #[test]
    fn get_ptr_slice_non_wrap() {
        let mut buf = CircBuf::new(5, 0u8);
        buf.append_assume_capacity(1);
        buf.append_assume_capacity(2);
        buf.append_assume_capacity(3);

        // Reading the existing 3 elements does not advance (end_offset == len).
        let len_before = buf.len();
        let (span0, span1) = buf.get_ptr_slice(0, 3);
        assert_eq!(span0, &[1, 2, 3]);
        assert!(span1.is_empty());
        assert_eq!(buf.len(), len_before);
    }

    #[test]
    fn get_ptr_slice_wrap() {
        let mut buf = CircBuf::new(3, 0u8);
        buf.append_assume_capacity(1);
        buf.append_assume_capacity(2);
        buf.append_assume_capacity(3);
        buf.delete_oldest(1); // tail -> 1
        buf.append_assume_capacity(4); // head wraps; storage is [4, 2, 3]

        let (span0, span1) = buf.get_ptr_slice(0, 3);
        assert_eq!(span0, &[2, 3]); // storage[1..]
        assert_eq!(span1, &[4]); // storage[0..=0]
    }

    #[test]
    fn get_ptr_slice_claims_space() {
        let mut buf = CircBuf::new(4, 0u8);
        assert_eq!(buf.len(), 0);

        // Requesting a range past the (zero) length advances head to claim it.
        {
            let (span0, span1) = buf.get_ptr_slice(0, 4);
            assert!(span1.is_empty());
            span0.copy_from_slice(&[10, 20, 30, 40]);
        }
        assert_eq!(buf.len(), 4);
        assert_eq!(
            collect(buf.iterator(Direction::Forward)),
            vec![10, 20, 30, 40],
        );
    }

    #[test]
    fn append_slice_non_wrap() {
        let mut buf = CircBuf::new(5, 0u8);
        buf.append_assume_capacity(1);
        buf.append_slice_assume_capacity(&[2, 3, 4]);
        assert_eq!(buf.len(), 4);
        assert_eq!(collect(buf.iterator(Direction::Forward)), vec![1, 2, 3, 4]);
    }

    #[test]
    fn append_slice_across_wrap() {
        // Leave one element (3) near the end of the storage, then append a slice that wraps.
        let mut buf = CircBuf::new(4, 0u8);
        buf.append_assume_capacity(1);
        buf.append_assume_capacity(2);
        buf.append_assume_capacity(3);
        buf.delete_oldest(2); // drops 1, 2; tail -> 2, leaving [_, _, 3, _]

        buf.append_slice_assume_capacity(&[4, 5, 6]); // wraps around the end
        assert_eq!(buf.len(), 4);
        assert_eq!(collect(buf.iterator(Direction::Forward)), vec![3, 4, 5, 6]);
    }

    #[test]
    fn append_slice_empty_is_noop() {
        let mut buf = CircBuf::new(3, 0u8);
        buf.append_assume_capacity(7);
        buf.append_slice_assume_capacity(&[]);
        assert_eq!(buf.len(), 1);
        assert_eq!(buf.first(), Some(&7));
    }

    #[test]
    fn resize_grows_full_buffer() {
        let mut buf = CircBuf::new(3, 0u8);
        buf.append_assume_capacity(1);
        buf.append_assume_capacity(2);
        buf.append_assume_capacity(3); // full

        buf.resize(5);
        assert_eq!(buf.capacity(), 5);
        assert_eq!(buf.len(), 3);
        assert_eq!(collect(buf.iterator(Direction::Forward)), vec![1, 2, 3]);

        // The grown capacity is appendable.
        assert_eq!(buf.append(4), Ok(()));
        assert_eq!(buf.append(5), Ok(()));
        assert_eq!(
            collect(buf.iterator(Direction::Forward)),
            vec![1, 2, 3, 4, 5],
        );
    }

    #[test]
    fn resize_grows_wrapped_buffer() {
        let mut buf = CircBuf::new(4, 0u8);
        buf.append_assume_capacity(1);
        buf.append_assume_capacity(2);
        buf.append_assume_capacity(3);
        buf.delete_oldest(1); // tail != 0, data [2, 3]

        buf.resize(6); // rotates to zero, then grows
        assert_eq!(buf.capacity(), 6);
        assert_eq!(buf.len(), 2);
        assert_eq!(collect(buf.iterator(Direction::Forward)), vec![2, 3]);

        assert_eq!(buf.append(4), Ok(()));
        assert_eq!(collect(buf.iterator(Direction::Forward)), vec![2, 3, 4]);
    }

    #[test]
    fn ensure_unused_capacity_grows_or_noops() {
        let mut buf = CircBuf::new(3, 0u8);
        buf.append_assume_capacity(1);
        buf.append_assume_capacity(2);
        buf.append_assume_capacity(3); // full

        buf.ensure_unused_capacity(2);
        assert!(buf.capacity() >= 5);
        assert_eq!(collect(buf.iterator(Direction::Forward)), vec![1, 2, 3]);
        assert_eq!(buf.append(4), Ok(()));
        assert_eq!(buf.append(5), Ok(()));

        // Already enough room: a no-op (capacity unchanged).
        let cap = buf.capacity();
        buf.ensure_unused_capacity(0);
        assert_eq!(buf.capacity(), cap);
    }

    #[test]
    fn resize_grows_from_zero_capacity() {
        let mut buf = CircBuf::new(0, 0u8);
        buf.resize(3);
        assert_eq!(buf.capacity(), 3);
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());

        buf.append_assume_capacity(1);
        buf.append_assume_capacity(2);
        buf.append_assume_capacity(3);
        assert_eq!(collect(buf.iterator(Direction::Forward)), vec![1, 2, 3]);
    }

    #[test]
    fn resize_shrinks_full_buffer() {
        let mut buf = CircBuf::new(4, 0u8);
        buf.append_assume_capacity(1);
        buf.append_assume_capacity(2);
        buf.append_assume_capacity(3);
        buf.append_assume_capacity(4); // full

        buf.resize(3); // the "smaller" path
        assert_eq!(buf.capacity(), 3);
        assert_eq!(buf.len(), 3);
        assert!(buf.full); // a full buffer shrunk to its length stays full (upstream)
        assert!(!buf.is_empty());
        assert_eq!(collect(buf.iterator(Direction::Forward)), vec![1, 2, 3]);
    }

    #[test]
    fn zero_capacity_is_full_and_empty_ended() {
        let mut buf = CircBuf::new(0, 0u8);
        assert_eq!(buf.capacity(), 0);
        assert_eq!(buf.len(), 0);
        // A zero-capacity buffer is `full` (so not appendable) but reports no ends.
        assert_eq!(buf.append(1), Err(Full));
        assert_eq!(buf.first(), None);
        assert_eq!(buf.last(), None);
    }
}
