//! A fixed-size collection of growable lists with a bulk capacity-retaining `reset` (port of
//! upstream `datastruct/array_list_collection`).

/// A collection of `list_count` growable lists, supporting a bulk `reset` that clears every list
/// while retaining its allocated capacity (upstream `ArrayListCollection`).
pub(crate) struct ArrayListCollection<T> {
    lists: Vec<Vec<T>>,
}

impl<T> ArrayListCollection<T> {
    /// Create a collection of `list_count` empty lists, each pre-allocated to `initial_capacity`
    /// (upstream `init`). Allocation is infallible in roastty, so this returns `Self` directly.
    pub(crate) fn new(list_count: usize, initial_capacity: usize) -> Self {
        let mut lists = Vec::with_capacity(list_count);
        for _ in 0..list_count {
            lists.push(Vec::with_capacity(initial_capacity));
        }
        Self { lists }
    }

    /// Clear every list, retaining its capacity (upstream `reset` / `clearRetainingCapacity`).
    pub(crate) fn reset(&mut self) {
        for list in &mut self.lists {
            list.clear();
        }
    }

    /// The number of lists in the collection.
    pub(crate) fn len(&self) -> usize {
        self.lists.len()
    }

    /// The lists, immutably (upstream's public `lists` field).
    pub(crate) fn lists(&self) -> &[Vec<T>] {
        &self.lists
    }

    /// The lists, mutably (so callers can append to a chosen list).
    pub(crate) fn lists_mut(&mut self) -> &mut [Vec<T>] {
        &mut self.lists
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_empty_lists_with_capacity() {
        let c: ArrayListCollection<u32> = ArrayListCollection::new(3, 8);
        assert_eq!(c.len(), 3);
        for list in c.lists() {
            assert!(list.is_empty());
            assert!(list.capacity() >= 8);
        }
    }

    #[test]
    fn reset_clears_all_lists_retaining_capacity() {
        let mut c: ArrayListCollection<u32> = ArrayListCollection::new(2, 8);
        for (i, list) in c.lists_mut().iter_mut().enumerate() {
            for j in 0..4 {
                list.push((i * 10 + j) as u32);
            }
        }
        assert!(c.lists().iter().all(|l| l.len() == 4));

        // Record each list's exact capacity, then assert `reset` empties without reallocating.
        let caps_before: Vec<usize> = c.lists().iter().map(|l| l.capacity()).collect();
        c.reset();
        for (list, cap_before) in c.lists().iter().zip(caps_before) {
            assert!(list.is_empty());
            assert_eq!(list.capacity(), cap_before); // capacity retained exactly
        }
    }

    #[test]
    fn lists_mut_allows_independent_appends() {
        let mut c: ArrayListCollection<&'static str> = ArrayListCollection::new(2, 4);
        c.lists_mut()[0].push("a");
        c.lists_mut()[0].push("b");
        c.lists_mut()[1].push("z");

        assert_eq!(c.lists()[0], vec!["a", "b"]);
        assert_eq!(c.lists()[1], vec!["z"]);
    }
}
