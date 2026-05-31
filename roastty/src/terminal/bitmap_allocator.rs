use std::mem::{align_of, size_of};

use super::size::{BaseAddress, Offset, OffsetBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BitmapAllocError {
    OutOfMemory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Layout {
    pub(super) total_size: usize,
    pub(super) bitmap_count: usize,
    pub(super) bitmap_start: usize,
    pub(super) chunks_start: usize,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct BitmapAllocator<const CHUNK_SIZE: usize> {
    bitmap: Offset<u64>,
    bitmap_count: usize,
    chunks: Offset<u8>,
}

impl<const CHUNK_SIZE: usize> BitmapAllocator<CHUNK_SIZE> {
    pub(super) const BASE_ALIGN: usize = align_of::<u64>();
    pub(super) const BITMAP_BIT_SIZE: usize = u64::BITS as usize;

    /// Initialize an allocator map inside caller-provided backing storage.
    ///
    /// # Safety
    ///
    /// `buf` must point to valid backing storage for `layout.total_size` bytes,
    /// aligned to `BASE_ALIGN`, and must remain live while the allocator is
    /// used.
    pub(super) unsafe fn init(buf: OffsetBuf, layout: Layout) -> Self {
        assert!(CHUNK_SIZE.is_power_of_two());
        assert_eq!((buf.start() as usize) % Self::BASE_ALIGN, 0);

        let bitmap = buf.member::<u64>(layout.bitmap_start);
        let bitmap_ptr = bitmap.ptr_mut(buf);
        for i in 0..layout.bitmap_count {
            unsafe {
                // Safety: caller provided backing memory matching `layout`.
                *bitmap_ptr.add(i) = u64::MAX;
            }
        }

        Self {
            bitmap,
            bitmap_count: layout.bitmap_count,
            chunks: buf.member::<u8>(layout.chunks_start),
        }
    }

    pub(super) fn bytes_required<T>(n: usize) -> usize {
        align_forward(size_of::<T>() * n, CHUNK_SIZE)
    }

    /// Allocate `n` values from `base`.
    ///
    /// # Safety
    ///
    /// `base` must point to backing storage initialized with this allocator's
    /// layout. The backing storage must outlive the returned slice. The returned
    /// slice must be freed exactly once through this allocator and the same
    /// backing storage. `CHUNK_SIZE` must satisfy `T`'s alignment.
    pub(super) unsafe fn alloc<'a, T, B>(
        &mut self,
        base: B,
        n: usize,
    ) -> Result<&'a mut [T], BitmapAllocError>
    where
        B: BaseAddress + Copy,
    {
        assert_eq!(CHUNK_SIZE % align_of::<T>(), 0);
        assert!(n > 0);

        let byte_count = size_of::<T>()
            .checked_mul(n)
            .ok_or(BitmapAllocError::OutOfMemory)?;
        let chunk_count = div_ceil(byte_count, CHUNK_SIZE).ok_or(BitmapAllocError::OutOfMemory)?;

        let bitmap_ptr = self.bitmap.ptr_mut(base);
        let bitmaps = unsafe {
            // Safety: caller guarantees `base` points to initialized allocator
            // storage with at least `bitmap_count` bitmap words.
            std::slice::from_raw_parts_mut(bitmap_ptr, self.bitmap_count)
        };
        let idx = find_free_chunks(bitmaps, chunk_count).ok_or(BitmapAllocError::OutOfMemory)?;

        let chunks = self.chunks.ptr_mut(base);
        let ptr = unsafe {
            // Safety: `idx` came from the bitmap for this backing storage and
            // `CHUNK_SIZE` alignment was asserted above.
            chunks.add(idx * CHUNK_SIZE).cast::<T>()
        };
        assert_eq!(ptr as usize % align_of::<T>(), 0);
        Ok(unsafe {
            // Safety: caller upholds backing-storage lifetime and the bitmap
            // marks this span as exclusively allocated.
            std::slice::from_raw_parts_mut(ptr, n)
        })
    }

    /// Free a previously allocated slice.
    ///
    /// # Safety
    ///
    /// `slice` must have been returned by this allocator from the same `base`,
    /// must not already have been freed, and no live references may depend on it
    /// after this call.
    pub(super) unsafe fn free<T, B>(&mut self, base: B, slice: &mut [T])
    where
        B: BaseAddress + Copy,
    {
        let bytes_len = size_of::<T>() * slice.len();
        let aligned_len = align_forward(bytes_len, CHUNK_SIZE);
        let chunk_count = aligned_len / CHUNK_SIZE;

        let chunks = self.chunks.ptr(base);
        let slice_ptr = slice.as_mut_ptr().cast::<u8>();
        let chunk_idx = (slice_ptr as usize - chunks as usize) / CHUNK_SIZE;

        let bitmaps = unsafe {
            // Safety: caller guarantees `base` points to initialized allocator
            // storage with at least `bitmap_count` bitmap words.
            std::slice::from_raw_parts_mut(self.bitmap.ptr_mut(base), self.bitmap_count)
        };

        for i in chunk_idx..chunk_idx + chunk_count {
            let bitmap = i / Self::BITMAP_BIT_SIZE;
            let bit = i % Self::BITMAP_BIT_SIZE;
            bitmaps[bitmap] |= 1_u64 << bit;
        }
    }

    pub(super) fn capacity_bytes(&self) -> usize {
        self.bitmap_count * Self::BITMAP_BIT_SIZE * CHUNK_SIZE
    }

    /// Returns the number of bytes currently in use.
    ///
    /// # Safety
    ///
    /// `base` must point to the backing storage used to initialize this
    /// allocator.
    pub(super) unsafe fn used_bytes<B>(&self, base: B) -> usize
    where
        B: BaseAddress,
    {
        let bitmaps = unsafe {
            // Safety: caller provides allocator backing storage.
            std::slice::from_raw_parts(self.bitmap.ptr(base), self.bitmap_count)
        };
        let free_chunks: usize = bitmaps
            .iter()
            .map(|bitmap| bitmap.count_ones() as usize)
            .sum();
        let total_chunks = self.bitmap_count * Self::BITMAP_BIT_SIZE;
        (total_chunks - free_chunks) * CHUNK_SIZE
    }

    #[cfg(test)]
    unsafe fn is_allocated<T, B>(&self, base: B, slice: &[T]) -> bool
    where
        B: BaseAddress + Copy,
    {
        let bytes_len = size_of::<T>() * slice.len();
        let aligned_len = align_forward(bytes_len, CHUNK_SIZE);
        let chunk_count = aligned_len / CHUNK_SIZE;

        let chunks = self.chunks.ptr(base);
        let chunk_idx = (slice.as_ptr().cast::<u8>() as usize - chunks as usize) / CHUNK_SIZE;
        let bitmaps = unsafe {
            // Safety: caller provides allocator backing storage.
            std::slice::from_raw_parts(self.bitmap.ptr(base), self.bitmap_count)
        };

        for i in chunk_idx..chunk_idx + chunk_count {
            let bitmap = i / Self::BITMAP_BIT_SIZE;
            let bit = i % Self::BITMAP_BIT_SIZE;
            if bitmaps[bitmap] & (1_u64 << bit) != 0 {
                return false;
            }
        }

        true
    }

    pub(super) fn layout(cap: usize) -> Layout {
        assert!(CHUNK_SIZE.is_power_of_two());

        let aligned_cap = align_forward(cap, CHUNK_SIZE);
        let chunk_count = aligned_cap / CHUNK_SIZE;
        let aligned_chunk_count = align_forward(chunk_count, Self::BITMAP_BIT_SIZE);
        let bitmap_count = aligned_chunk_count / Self::BITMAP_BIT_SIZE;

        let bitmap_start = 0;
        let bitmap_end = size_of::<u64>() * bitmap_count;
        let chunks_start = align_forward(bitmap_end, align_of::<u8>());
        let chunks_end = chunks_start + (aligned_cap * CHUNK_SIZE);

        Layout {
            total_size: chunks_end,
            bitmap_count,
            bitmap_start,
            chunks_start,
        }
    }
}

fn find_free_chunks(bitmaps: &mut [u64], n: usize) -> Option<usize> {
    assert!(n > 0);

    let total_chunks = bitmaps.len() * u64::BITS as usize;
    if n > total_chunks {
        return None;
    }

    for start in 0..=total_chunks - n {
        if (start..start + n).all(|i| bit_is_free(bitmaps, i)) {
            for i in start..start + n {
                mark_used(bitmaps, i);
            }
            return Some(start);
        }
    }

    None
}

fn bit_is_free(bitmaps: &[u64], i: usize) -> bool {
    let bitmap = i / u64::BITS as usize;
    let bit = i % u64::BITS as usize;
    bitmaps[bitmap] & (1_u64 << bit) != 0
}

fn mark_used(bitmaps: &mut [u64], i: usize) {
    let bitmap = i / u64::BITS as usize;
    let bit = i % u64::BITS as usize;
    bitmaps[bitmap] &= !(1_u64 << bit);
}

fn div_ceil(n: usize, d: usize) -> Option<usize> {
    let adjusted = n.checked_add(d.checked_sub(1)?)?;
    Some(adjusted / d)
}

fn align_forward(value: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (value + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::marker::PhantomData;

    struct Backing {
        words: Vec<u64>,
        len: usize,
        _marker: PhantomData<[u8]>,
    }

    impl Backing {
        fn new(len: usize) -> Self {
            let word_count = align_forward(len, size_of::<u64>()) / size_of::<u64>();
            Self {
                words: vec![0; word_count],
                len,
                _marker: PhantomData,
            }
        }

        fn as_mut_ptr(&mut self) -> *mut u8 {
            self.words.as_mut_ptr().cast::<u8>()
        }

        fn as_ptr(&self) -> *const u8 {
            self.words.as_ptr().cast::<u8>()
        }

        fn len(&self) -> usize {
            self.len
        }
    }

    fn allocator<const CHUNK_SIZE: usize>(
        cap: usize,
    ) -> (Backing, BitmapAllocator<CHUNK_SIZE>, Layout) {
        let layout = BitmapAllocator::<CHUNK_SIZE>::layout(cap);
        let mut backing = Backing::new(layout.total_size);
        assert!(backing.len() >= layout.total_size);
        let bitmap = unsafe {
            BitmapAllocator::<CHUNK_SIZE>::init(OffsetBuf::init(backing.as_mut_ptr()), layout)
        };
        (backing, bitmap, layout)
    }

    fn bitmap_words<const N: usize>(bm: &BitmapAllocator<N>, backing: &Backing) -> Vec<u64> {
        unsafe { std::slice::from_raw_parts(bm.bitmap.ptr(backing.as_ptr()), bm.bitmap_count) }
            .to_vec()
    }

    #[test]
    fn find_free_chunks_single_found() {
        let mut bitmaps =
            [0b10000000_00000000_00000000_00000000_00000000_00000000_00001110_00000000];
        let idx = find_free_chunks(&mut bitmaps, 2).unwrap();
        assert_eq!(9, idx);
        assert_eq!(
            0b10000000_00000000_00000000_00000000_00000000_00000000_00001000_00000000,
            bitmaps[0]
        );
    }

    #[test]
    fn find_free_chunks_single_not_found() {
        let mut bitmaps =
            [0b10000111_00000000_00000000_00000000_00000000_00000000_00000000_00000000];
        assert_eq!(None, find_free_chunks(&mut bitmaps, 4));
    }

    #[test]
    fn find_free_chunks_multiple_found() {
        let mut bitmaps = [
            0b10000111_00000000_00000000_00000000_00000000_00000000_00000000_01110000,
            0b10000000_00111110_00000000_00000000_00000000_00000000_00111110_00000000,
        ];
        let idx = find_free_chunks(&mut bitmaps, 4).unwrap();
        assert_eq!(73, idx);
        assert_eq!(
            0b10000000_00111110_00000000_00000000_00000000_00000000_00100000_00000000,
            bitmaps[1]
        );
    }

    #[test]
    fn find_free_chunks_exactly_64_chunks() {
        let mut bitmaps = [u64::MAX];
        let idx = find_free_chunks(&mut bitmaps, 64).unwrap();
        assert_eq!(0, bitmaps[0]);
        assert_eq!(0, idx);
    }

    #[test]
    fn find_free_chunks_larger_than_64_chunks() {
        let mut bitmaps = [u64::MAX, u64::MAX];
        let idx = find_free_chunks(&mut bitmaps, 65).unwrap();
        assert_eq!(0, bitmaps[0]);
        assert_eq!(
            0b11111111_11111111_11111111_11111111_11111111_11111111_11111111_11111110,
            bitmaps[1]
        );
        assert_eq!(0, idx);
    }

    #[test]
    fn find_free_chunks_larger_than_64_chunks_not_at_beginning() {
        let mut bitmaps = [
            0b11111111_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
            u64::MAX,
            u64::MAX,
        ];
        let idx = find_free_chunks(&mut bitmaps, 65).unwrap();
        assert_eq!(0, bitmaps[0]);
        assert_eq!(
            0b11111110_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
            bitmaps[1]
        );
        assert_eq!(u64::MAX, bitmaps[2]);
        assert_eq!(56, idx);
    }

    #[test]
    fn find_free_chunks_larger_than_64_chunks_exact() {
        let mut bitmaps = [u64::MAX, u64::MAX];
        let idx = find_free_chunks(&mut bitmaps, 128).unwrap();
        assert_eq!(0, bitmaps[0]);
        assert_eq!(0, bitmaps[1]);
        assert_eq!(0, idx);
    }

    #[test]
    fn bitmap_allocator_layout() {
        type Alloc = BitmapAllocator<4>;
        let cap = 64 * 4;
        let layout = Alloc::layout(cap);

        assert_eq!(1, layout.bitmap_count);
        assert_eq!(0, layout.bitmap_start);
        assert_eq!(8, layout.chunks_start);
        assert_eq!(1032, layout.total_size);
    }

    #[test]
    fn bitmap_allocator_alloc_sequentially() {
        type Alloc = BitmapAllocator<4>;
        let (mut backing, mut bm, _) = allocator::<4>(64);

        let ptr_addr;
        let ptr3_addr;
        unsafe {
            let ptr = bm.alloc::<u8, _>(backing.as_mut_ptr(), 1).unwrap();
            ptr[0] = b'A';
            ptr_addr = ptr.as_ptr() as usize;

            let ptr2 = bm.alloc::<u8, _>(backing.as_mut_ptr(), 1).unwrap();
            assert_ne!(ptr_addr, ptr2.as_ptr() as usize);
            assert_eq!(ptr_addr + 4, ptr2.as_ptr() as usize);

            let ptr = std::slice::from_raw_parts_mut(ptr_addr as *mut u8, 1);
            bm.free(backing.as_mut_ptr(), ptr);
            let ptr3 = bm.alloc::<u8, _>(backing.as_mut_ptr(), 1).unwrap();
            ptr3_addr = ptr3.as_ptr() as usize;
        }

        assert_eq!(ptr_addr, ptr3_addr);
        assert_eq!(Alloc::BITMAP_BIT_SIZE, 64);
    }

    #[test]
    fn bitmap_allocator_alloc_non_byte() {
        let (mut backing, mut bm, _) = allocator::<4>(128);

        unsafe {
            let ptr = bm.alloc::<u32, _>(backing.as_mut_ptr(), 1).unwrap();
            ptr[0] = b'A' as u32;
            let ptr_addr = ptr.as_ptr() as usize;

            let ptr2 = bm.alloc::<u32, _>(backing.as_mut_ptr(), 1).unwrap();
            assert_ne!(ptr_addr, ptr2.as_ptr() as usize);
            assert_eq!(ptr_addr + 4, ptr2.as_ptr() as usize);

            let ptr = std::slice::from_raw_parts_mut(ptr_addr as *mut u32, 1);
            bm.free(backing.as_mut_ptr(), ptr);
            let ptr3 = bm.alloc::<u32, _>(backing.as_mut_ptr(), 1).unwrap();
            assert_eq!(ptr_addr, ptr3.as_ptr() as usize);
        }
    }

    #[test]
    fn bitmap_allocator_alloc_non_byte_multi_chunk() {
        let (mut backing, mut bm, _) = allocator::<16>(128);

        unsafe {
            let ptr = bm.alloc::<u32, _>(backing.as_mut_ptr(), 6).unwrap();
            assert_eq!(6, ptr.len());
            ptr.fill(b'A' as u32);
            let ptr_addr = ptr.as_ptr() as usize;

            let ptr2 = bm.alloc::<u32, _>(backing.as_mut_ptr(), 1).unwrap();
            assert_ne!(ptr_addr, ptr2.as_ptr() as usize);
            assert_eq!(
                ptr_addr + (size_of::<u32>() * 4 * 2),
                ptr2.as_ptr() as usize
            );

            let ptr = std::slice::from_raw_parts_mut(ptr_addr as *mut u32, 6);
            bm.free(backing.as_mut_ptr(), ptr);
            let ptr3 = bm.alloc::<u32, _>(backing.as_mut_ptr(), 1).unwrap();
            assert_eq!(ptr_addr, ptr3.as_ptr() as usize);
        }
    }

    #[test]
    fn bitmap_allocator_alloc_large() {
        let (mut backing, mut bm, _) = allocator::<2>(256);

        unsafe {
            let ptr = bm.alloc::<u8, _>(backing.as_mut_ptr(), 129).unwrap();
            ptr[0] = b'A';
            bm.free(backing.as_mut_ptr(), ptr);
        }
    }

    #[test]
    fn bitmap_allocator_alloc_overflow_returns_out_of_memory() {
        let (mut backing, mut bm, _) = allocator::<2>(256);

        unsafe {
            let result = bm.alloc::<u8, _>(backing.as_mut_ptr(), usize::MAX);
            assert_eq!(Err(BitmapAllocError::OutOfMemory), result.map(|_| ()));
        }
    }

    fn expect_filled(slice: &[u8], value: u8) {
        assert!(slice.iter().all(|actual| *actual == value));
    }

    fn expect_all_free<const N: usize>(bm: &BitmapAllocator<N>, backing: &Backing, count: usize) {
        assert_eq!(vec![u64::MAX; count], bitmap_words(bm, backing));
    }

    #[test]
    fn bitmap_allocator_alloc_and_free_one_bitmap() {
        let (mut backing, mut bm, _) = allocator::<1>(BitmapAllocator::<1>::BITMAP_BIT_SIZE * 3);

        unsafe {
            let slice = bm
                .alloc::<u8, _>(backing.as_mut_ptr(), BitmapAllocator::<1>::BITMAP_BIT_SIZE)
                .unwrap();
            assert_eq!(BitmapAllocator::<1>::BITMAP_BIT_SIZE, slice.len());
            slice.fill(0x11);
            expect_filled(slice, 0x11);
            let slice_ptr = slice.as_mut_ptr();
            let slice_len = slice.len();

            assert!(bm.is_allocated(backing.as_mut_ptr(), slice));
            let slice = std::slice::from_raw_parts_mut(slice_ptr, slice_len);
            bm.free(backing.as_mut_ptr(), slice);
            assert!(!bm.is_allocated(backing.as_mut_ptr(), slice));
        }

        expect_all_free(&bm, &backing, 3);
    }

    #[test]
    fn bitmap_allocator_alloc_and_free_half_bitmap() {
        let (mut backing, mut bm, _) = allocator::<1>(BitmapAllocator::<1>::BITMAP_BIT_SIZE * 3);

        unsafe {
            let slice = bm
                .alloc::<u8, _>(
                    backing.as_mut_ptr(),
                    BitmapAllocator::<1>::BITMAP_BIT_SIZE / 2,
                )
                .unwrap();
            assert_eq!(BitmapAllocator::<1>::BITMAP_BIT_SIZE / 2, slice.len());
            slice.fill(0x11);
            expect_filled(slice, 0x11);
            let slice_ptr = slice.as_mut_ptr();
            let slice_len = slice.len();

            assert!(bm.is_allocated(backing.as_mut_ptr(), slice));
            let slice = std::slice::from_raw_parts_mut(slice_ptr, slice_len);
            bm.free(backing.as_mut_ptr(), slice);
            assert!(!bm.is_allocated(backing.as_mut_ptr(), slice));
        }

        expect_all_free(&bm, &backing, 3);
    }

    #[test]
    fn bitmap_allocator_alloc_and_free_two_half_bitmaps() {
        let (mut backing, mut bm, _) = allocator::<1>(BitmapAllocator::<1>::BITMAP_BIT_SIZE * 3);

        unsafe {
            let slice = bm
                .alloc::<u8, _>(
                    backing.as_mut_ptr(),
                    BitmapAllocator::<1>::BITMAP_BIT_SIZE / 2,
                )
                .unwrap();
            slice.fill(0x11);
            let slice_ptr = slice.as_mut_ptr();
            let slice_len = slice.len();

            let slice2 = bm
                .alloc::<u8, _>(
                    backing.as_mut_ptr(),
                    BitmapAllocator::<1>::BITMAP_BIT_SIZE / 2,
                )
                .unwrap();
            slice2.fill(0x22);
            expect_filled(slice2, 0x22);
            expect_filled(std::slice::from_raw_parts(slice_ptr, slice_len), 0x11);
            let slice2_ptr = slice2.as_mut_ptr();
            let slice2_len = slice2.len();

            let slice2 = std::slice::from_raw_parts_mut(slice2_ptr, slice2_len);
            assert!(bm.is_allocated(backing.as_mut_ptr(), slice2));
            bm.free(backing.as_mut_ptr(), slice2);
            assert!(!bm.is_allocated(backing.as_mut_ptr(), slice2));

            let slice = std::slice::from_raw_parts_mut(slice_ptr, slice_len);
            assert!(bm.is_allocated(backing.as_mut_ptr(), slice));
            bm.free(backing.as_mut_ptr(), slice);
            assert!(!bm.is_allocated(backing.as_mut_ptr(), slice));
        }

        expect_all_free(&bm, &backing, 3);
    }

    #[test]
    fn bitmap_allocator_alloc_and_free_one_and_half_bitmaps() {
        let (mut backing, mut bm, _) = allocator::<1>(BitmapAllocator::<1>::BITMAP_BIT_SIZE * 3);

        unsafe {
            let len = 3 * BitmapAllocator::<1>::BITMAP_BIT_SIZE / 2;
            let slice = bm.alloc::<u8, _>(backing.as_mut_ptr(), len).unwrap();
            assert_eq!(len, slice.len());
            slice.fill(0x11);
            expect_filled(slice, 0x11);
            let slice_ptr = slice.as_mut_ptr();

            assert!(bm.is_allocated(backing.as_mut_ptr(), slice));
            let slice = std::slice::from_raw_parts_mut(slice_ptr, len);
            bm.free(backing.as_mut_ptr(), slice);
            assert!(!bm.is_allocated(backing.as_mut_ptr(), slice));
        }

        expect_all_free(&bm, &backing, 3);
    }

    #[test]
    fn bitmap_allocator_alloc_and_free_two_one_and_half_bitmaps() {
        let (mut backing, mut bm, _) = allocator::<1>(BitmapAllocator::<1>::BITMAP_BIT_SIZE * 3);

        unsafe {
            let len = 3 * BitmapAllocator::<1>::BITMAP_BIT_SIZE / 2;
            let slice = bm.alloc::<u8, _>(backing.as_mut_ptr(), len).unwrap();
            slice.fill(0x11);
            let slice_ptr = slice.as_mut_ptr();

            let slice2 = bm.alloc::<u8, _>(backing.as_mut_ptr(), len).unwrap();
            slice2.fill(0x22);
            expect_filled(slice2, 0x22);
            expect_filled(std::slice::from_raw_parts(slice_ptr, len), 0x11);
            let slice2_ptr = slice2.as_mut_ptr();

            let slice2 = std::slice::from_raw_parts_mut(slice2_ptr, len);
            assert!(bm.is_allocated(backing.as_mut_ptr(), slice2));
            bm.free(backing.as_mut_ptr(), slice2);
            assert!(!bm.is_allocated(backing.as_mut_ptr(), slice2));

            let slice = std::slice::from_raw_parts_mut(slice_ptr, len);
            assert!(bm.is_allocated(backing.as_mut_ptr(), slice));
            bm.free(backing.as_mut_ptr(), slice);
            assert!(!bm.is_allocated(backing.as_mut_ptr(), slice));
        }

        expect_all_free(&bm, &backing, 3);
    }

    #[test]
    fn bitmap_allocator_alloc_and_free_one_and_half_bitmaps_offset_by_three_quarters() {
        let (mut backing, mut bm, _) = allocator::<1>(BitmapAllocator::<1>::BITMAP_BIT_SIZE * 3);

        unsafe {
            let short_len = 3 * BitmapAllocator::<1>::BITMAP_BIT_SIZE / 4;
            let long_len = 3 * BitmapAllocator::<1>::BITMAP_BIT_SIZE / 2;
            let slice = bm.alloc::<u8, _>(backing.as_mut_ptr(), short_len).unwrap();
            slice.fill(0x11);
            let slice_ptr = slice.as_mut_ptr();

            let slice2 = bm.alloc::<u8, _>(backing.as_mut_ptr(), long_len).unwrap();
            slice2.fill(0x22);
            expect_filled(slice2, 0x22);
            expect_filled(std::slice::from_raw_parts(slice_ptr, short_len), 0x11);
            let slice2_ptr = slice2.as_mut_ptr();

            let slice2 = std::slice::from_raw_parts_mut(slice2_ptr, long_len);
            assert!(bm.is_allocated(backing.as_mut_ptr(), slice2));
            bm.free(backing.as_mut_ptr(), slice2);
            assert!(!bm.is_allocated(backing.as_mut_ptr(), slice2));

            let slice = std::slice::from_raw_parts_mut(slice_ptr, short_len);
            assert!(bm.is_allocated(backing.as_mut_ptr(), slice));
            bm.free(backing.as_mut_ptr(), slice);
            assert!(!bm.is_allocated(backing.as_mut_ptr(), slice));
        }

        expect_all_free(&bm, &backing, 3);
    }

    #[test]
    fn bitmap_allocator_alloc_and_free_three_three_quarter_bitmaps() {
        let (mut backing, mut bm, _) = allocator::<1>(BitmapAllocator::<1>::BITMAP_BIT_SIZE * 3);

        unsafe {
            let len = 3 * BitmapAllocator::<1>::BITMAP_BIT_SIZE / 4;
            let slice = bm.alloc::<u8, _>(backing.as_mut_ptr(), len).unwrap();
            slice.fill(0x11);
            let slice_ptr = slice.as_mut_ptr();

            let slice2 = bm.alloc::<u8, _>(backing.as_mut_ptr(), len).unwrap();
            slice2.fill(0x22);
            let slice2_ptr = slice2.as_mut_ptr();

            let slice3 = bm.alloc::<u8, _>(backing.as_mut_ptr(), len).unwrap();
            slice3.fill(0x33);
            expect_filled(slice3, 0x33);
            expect_filled(std::slice::from_raw_parts(slice2_ptr, len), 0x22);
            expect_filled(std::slice::from_raw_parts(slice_ptr, len), 0x11);
            let slice3_ptr = slice3.as_mut_ptr();

            let slice2 = std::slice::from_raw_parts_mut(slice2_ptr, len);
            assert!(bm.is_allocated(backing.as_mut_ptr(), slice2));
            bm.free(backing.as_mut_ptr(), slice2);
            assert!(!bm.is_allocated(backing.as_mut_ptr(), slice2));

            let slice = std::slice::from_raw_parts_mut(slice_ptr, len);
            assert!(bm.is_allocated(backing.as_mut_ptr(), slice));
            bm.free(backing.as_mut_ptr(), slice);
            assert!(!bm.is_allocated(backing.as_mut_ptr(), slice));

            let slice3 = std::slice::from_raw_parts_mut(slice3_ptr, len);
            assert!(bm.is_allocated(backing.as_mut_ptr(), slice3));
            bm.free(backing.as_mut_ptr(), slice3);
            assert!(!bm.is_allocated(backing.as_mut_ptr(), slice3));
        }

        expect_all_free(&bm, &backing, 3);
    }

    #[test]
    fn bitmap_allocator_alloc_and_free_two_one_and_half_bitmaps_offset_three_quarters() {
        let (mut backing, mut bm, _) = allocator::<1>(BitmapAllocator::<1>::BITMAP_BIT_SIZE * 4);

        unsafe {
            let short_len = 3 * BitmapAllocator::<1>::BITMAP_BIT_SIZE / 4;
            let long_len = 3 * BitmapAllocator::<1>::BITMAP_BIT_SIZE / 2;
            let slice = bm.alloc::<u8, _>(backing.as_mut_ptr(), short_len).unwrap();
            slice.fill(0x11);
            let slice_ptr = slice.as_mut_ptr();

            let slice2 = bm.alloc::<u8, _>(backing.as_mut_ptr(), long_len).unwrap();
            slice2.fill(0x22);
            let slice2_ptr = slice2.as_mut_ptr();

            let slice3 = bm.alloc::<u8, _>(backing.as_mut_ptr(), long_len).unwrap();
            slice3.fill(0x33);
            expect_filled(slice3, 0x33);
            expect_filled(std::slice::from_raw_parts(slice2_ptr, long_len), 0x22);
            expect_filled(std::slice::from_raw_parts(slice_ptr, short_len), 0x11);
            let slice3_ptr = slice3.as_mut_ptr();

            let slice2 = std::slice::from_raw_parts_mut(slice2_ptr, long_len);
            assert!(bm.is_allocated(backing.as_mut_ptr(), slice2));
            bm.free(backing.as_mut_ptr(), slice2);
            assert!(!bm.is_allocated(backing.as_mut_ptr(), slice2));

            let slice = std::slice::from_raw_parts_mut(slice_ptr, short_len);
            assert!(bm.is_allocated(backing.as_mut_ptr(), slice));
            bm.free(backing.as_mut_ptr(), slice);
            assert!(!bm.is_allocated(backing.as_mut_ptr(), slice));

            let slice3 = std::slice::from_raw_parts_mut(slice3_ptr, long_len);
            assert!(bm.is_allocated(backing.as_mut_ptr(), slice3));
            bm.free(backing.as_mut_ptr(), slice3);
            assert!(!bm.is_allocated(backing.as_mut_ptr(), slice3));
        }

        expect_all_free(&bm, &backing, 4);
    }

    #[test]
    fn bitmap_allocator_bytes_required() {
        assert_eq!(16, BitmapAllocator::<16>::bytes_required::<u8>(1));
        assert_eq!(16, BitmapAllocator::<16>::bytes_required::<u8>(16));
        assert_eq!(32, BitmapAllocator::<16>::bytes_required::<u8>(17));
        assert_eq!(16, BitmapAllocator::<16>::bytes_required::<u32>(1));
        assert_eq!(16, BitmapAllocator::<16>::bytes_required::<u32>(4));
        assert_eq!(32, BitmapAllocator::<16>::bytes_required::<u32>(5));
        assert_eq!(32, BitmapAllocator::<16>::bytes_required::<u32>(6));

        assert_eq!(4, BitmapAllocator::<4>::bytes_required::<u8>(1));
        assert_eq!(4, BitmapAllocator::<4>::bytes_required::<u8>(4));
        assert_eq!(8, BitmapAllocator::<4>::bytes_required::<u8>(5));
        assert_eq!(4, BitmapAllocator::<4>::bytes_required::<u32>(1));
        assert_eq!(8, BitmapAllocator::<4>::bytes_required::<u32>(2));

        assert_eq!(32, BitmapAllocator::<32>::bytes_required::<u8>(1));
        assert_eq!(32, BitmapAllocator::<32>::bytes_required::<u8>(32));
        assert_eq!(64, BitmapAllocator::<32>::bytes_required::<u8>(33));
    }
}
