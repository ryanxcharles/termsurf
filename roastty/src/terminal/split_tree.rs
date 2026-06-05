//! The split-pane tree (port of upstream `datastruct/split_tree`). Lands the vocabulary
//! (`Handle` / `Layout` / `Direction`), the `Split` / `Slot` payloads, the spatial geometry, the
//! `Node<V>` arena (`SplitTree`) with the structural queries (`is_empty` / `is_split` / `deepest` /
//! `dimensions`) and `Rc<V>`-based view ref-counting, the leaf `iter`ator, `zoom`, and the `Goto`
//! enum. Still deferred: the tree-shaping operations (`split` / `remove` / `equalize` / `resize`),
//! the `goto` method, the `Spatial` container's normalization (`spatial` / `fillSpatialSlots`) and
//! `nearest`, and the formatters.

use half::f16;
use std::rc::Rc;

/// A handle into the tree's `nodes` array (upstream `Node.Handle`): a `u16`-backed index, so nodes
/// are referenced by 16-bit handles rather than pointers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Handle(u16);

impl Handle {
    /// The root node's handle (index 0) (upstream `.root`).
    pub(crate) const ROOT: Handle = Handle(0);

    /// Build a handle from an index (upstream `@enumFromInt`). The full `u16` range is valid —
    /// upstream's `enum(u16)` can represent `u16::MAX`, which the tree iterator uses as an
    /// end sentinel (`@enumFromInt(handle.idx() + 1)`).
    pub(crate) fn from_index(index: usize) -> Handle {
        assert!(index <= u16::MAX as usize, "split tree handle out of range");
        Handle(index as u16)
    }

    /// The index this handle refers to (upstream `idx`).
    pub(crate) fn idx(self) -> usize {
        self.0 as usize
    }

    /// Offset the handle by `v` (upstream `offset`), asserting the result stays below `u16::MAX`
    /// (matching upstream's `final < maxInt(Backing)`).
    pub(crate) fn offset(self, v: usize) -> Handle {
        let result = (self.0 as usize)
            .checked_add(v)
            .expect("split tree handle offset overflow");
        assert!(result < u16::MAX as usize, "split tree handle overflow");
        Handle(result as u16)
    }
}

/// The orientation of a split (upstream `Split.Layout`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Layout {
    Horizontal,
    Vertical,
}

/// The direction a new view is split off in (upstream `Split.Direction`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Left,
    Right,
    Down,
    Up,
}

impl Direction {
    /// The split layout and whether the new view goes on the first (left / top) side, for a split
    /// in this direction (upstream `split`'s direction switch).
    pub(crate) fn split_layout(self) -> (Layout, bool) {
        match self {
            Direction::Left => (Layout::Horizontal, true),
            Direction::Right => (Layout::Horizontal, false),
            Direction::Up => (Layout::Vertical, true),
            Direction::Down => (Layout::Vertical, false),
        }
    }
}

/// The payload of a split node (upstream `Split`): two child handles, the split orientation, and
/// the fraction of space given to the first (left / top) child.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Split {
    pub(crate) layout: Layout,
    pub(crate) ratio: f16,
    pub(crate) left: Handle,
    pub(crate) right: Handle,
}

/// A node's normalized 2D rectangle in the spatial representation (upstream `Spatial.Slot`); all
/// coordinates are in a 1×1 space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Slot {
    pub(crate) x: f16,
    pub(crate) y: f16,
    pub(crate) width: f16,
    pub(crate) height: f16,
}

impl Slot {
    /// The right edge, `x + width` (upstream `maxX`).
    pub(crate) fn max_x(self) -> f16 {
        self.x + self.width
    }

    /// The bottom edge, `y + height` (upstream `maxY`).
    pub(crate) fn max_y(self) -> f16 {
        self.y + self.height
    }

    /// Whether `self` (a candidate slot) lies in `direction` relative to `target` (upstream
    /// `nearest`'s direction switch).
    pub(crate) fn is_in_direction(self, target: Slot, direction: SpatialDirection) -> bool {
        match direction {
            SpatialDirection::Left => self.max_x() <= target.x,
            SpatialDirection::Right => self.x >= target.max_x(),
            SpatialDirection::Up => self.max_y() <= target.y,
            SpatialDirection::Down => self.y >= target.max_y(),
        }
    }

    /// The euclidean distance from `self` to `target` (upstream `nearest`'s
    /// `@sqrt(dx*dx + dy*dy)`). The `dx`/`dy`/products/sum are computed in `f16` (matching
    /// upstream's per-op binary16 arithmetic); the square root widens the `f16` sum to `f64`, takes
    /// the root there, and rounds back to `f16` (Rust's `half` has no `f16` sqrt). The wide `f64`
    /// intermediate makes this a single effective rounding, matching Zig's `@sqrt` on `f16`.
    pub(crate) fn distance_to(self, target: Slot) -> f16 {
        let dx = self.x - target.x;
        let dy = self.y - target.y;
        let sum = dx * dx + dy * dy;
        f16::from_f64(sum.to_f64().sqrt())
    }

    /// `self` shifted by one full normalized (1×1) grid for wrap-around in `direction` (upstream
    /// `nearestWrapped`'s target shift). Shifts in the opposite sense of travel so the nearest
    /// search re-finds across the wrap boundary.
    pub(crate) fn wrapped_for(self, direction: SpatialDirection) -> Slot {
        let one = f16::from_f32(1.0);
        match direction {
            SpatialDirection::Left => Slot {
                x: self.x + one,
                ..self
            },
            SpatialDirection::Right => Slot {
                x: self.x - one,
                ..self
            },
            SpatialDirection::Up => Slot {
                y: self.y + one,
                ..self
            },
            SpatialDirection::Down => Slot {
                y: self.y - one,
                ..self
            },
        }
    }
}

/// A spatial navigation direction — the nearest surface visually in this direction (upstream
/// `Spatial.Direction`; a separate type from `Direction`, with the same variants).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpatialDirection {
    Left,
    Right,
    Down,
    Up,
}

/// A node in the split tree (upstream `Node`): a leaf holding a (ref-counted) view, or an internal
/// split.
#[derive(Debug, Clone)]
pub(crate) enum Node<V> {
    Leaf(Rc<V>),
    Split(Split),
}

/// Which child to descend into (upstream `Side`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    Left,
    Right,
}

/// Relative tree dimensions in leaf units (upstream `dimensions`' anonymous return).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Dimensions {
    pub(crate) width: u16,
    pub(crate) height: u16,
}

/// An immutable binary tree of split panes, stored as a flat node arena (upstream `SplitTree(V)`).
/// Index 0 is the root. Cloning the tree clones each leaf's `Rc<V>` (upstream `clone` / `refNodes`);
/// dropping it drops them (upstream `deinit` / `viewUnref`).
#[derive(Debug, Clone)]
pub(crate) struct SplitTree<V> {
    nodes: Vec<Node<V>>,
    zoomed: Option<Handle>,
}

impl<V> SplitTree<V> {
    /// An empty tree with no nodes (upstream `empty`).
    pub(crate) fn empty() -> Self {
        SplitTree {
            nodes: Vec::new(),
            zoomed: None,
        }
    }

    /// A single-leaf tree holding `view` (upstream `init`). The caller's `Rc` is stored (its
    /// refcount is the view's ref).
    pub(crate) fn new(view: Rc<V>) -> Self {
        SplitTree {
            nodes: vec![Node::Leaf(view)],
            zoomed: None,
        }
    }

    /// Whether the tree has no nodes (upstream `isEmpty`).
    pub(crate) fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Whether the root is a split — i.e. the tree has more than one view (upstream `isSplit`).
    pub(crate) fn is_split(&self) -> bool {
        matches!(self.nodes.first(), Some(Node::Split(_)))
    }

    /// The deepest leaf reached by always descending into the `side` child from `from` (upstream
    /// `deepest`).
    pub(crate) fn deepest(&self, side: Side, from: Handle) -> Handle {
        let mut current = from;
        loop {
            match &self.nodes[current.idx()] {
                Node::Leaf(_) => return current,
                Node::Split(s) => {
                    current = match side {
                        Side::Left => s.left,
                        Side::Right => s.right,
                    }
                }
            }
        }
    }

    /// Iterate the tree's leaf views in node-arena order (upstream `iterator`).
    pub(crate) fn iter(&self) -> Iter<'_, V> {
        Iter {
            nodes: &self.nodes,
            i: 0,
        }
    }

    /// Set (or clear) the zoomed node (upstream `zoom`). Asserts the handle is in range.
    pub(crate) fn zoom(&mut self, handle: Option<Handle>) {
        if let Some(h) = handle {
            assert!(h.idx() < self.nodes.len(), "zoom handle out of range");
        }
        self.zoomed = handle;
    }

    /// The currently-zoomed node, if any (upstream's `zoomed` field).
    pub(crate) fn zoomed(&self) -> Option<Handle> {
        self.zoomed
    }

    /// Relative dimensions of the subtree at `from`, in leaf units (upstream `dimensions`).
    pub(crate) fn dimensions(&self, from: Handle) -> Dimensions {
        match &self.nodes[from.idx()] {
            Node::Leaf(_) => Dimensions {
                width: 1,
                height: 1,
            },
            Node::Split(s) => {
                let left = self.dimensions(s.left);
                let right = self.dimensions(s.right);
                match s.layout {
                    Layout::Horizontal => Dimensions {
                        width: left.width + right.width,
                        height: left.height.max(right.height),
                    },
                    Layout::Vertical => Dimensions {
                        width: left.width.max(right.width),
                        height: left.height + right.height,
                    },
                }
            }
        }
    }
}

/// A leaf visited by the tree iterator (upstream `ViewEntry`).
pub(crate) struct ViewEntry<'a, V> {
    pub(crate) handle: Handle,
    pub(crate) view: &'a Rc<V>,
}

/// An iterator over the tree's leaf views, in node-arena order (upstream `Iterator`).
pub(crate) struct Iter<'a, V> {
    nodes: &'a [Node<V>],
    i: usize,
}

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = ViewEntry<'a, V>;

    fn next(&mut self) -> Option<ViewEntry<'a, V>> {
        while self.i < self.nodes.len() {
            let handle = Handle::from_index(self.i);
            self.i += 1;
            if let Node::Leaf(view) = &self.nodes[handle.idx()] {
                return Some(ViewEntry { handle, view });
            }
            // split → skip, advance to the next node (upstream's `self.next()` tail recursion).
        }
        None
    }
}

/// A navigation target for `goto` (upstream `Goto`): the previous / next view (optionally wrapped),
/// or a spatial direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Goto {
    Previous,
    Next,
    PreviousWrapped,
    NextWrapped,
    Spatial(SpatialDirection),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_root_idx_offset() {
        assert_eq!(Handle::ROOT.idx(), 0);
        assert_eq!(Handle::from_index(5).idx(), 5);
        assert_eq!(Handle::ROOT.offset(3).idx(), 3);
        assert_eq!(Handle::from_index(2).offset(4).idx(), 6);
    }

    #[test]
    fn from_index_allows_u16_max() {
        // Upstream's enum can represent u16::MAX (the iterator's end sentinel).
        assert_eq!(
            Handle::from_index(u16::MAX as usize).idx(),
            u16::MAX as usize
        );
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn from_index_above_u16_max_panics() {
        let _ = Handle::from_index(u16::MAX as usize + 1);
    }

    #[test]
    fn offset_just_below_u16_max_succeeds() {
        assert_eq!(
            Handle::ROOT.offset(u16::MAX as usize - 1).idx(),
            u16::MAX as usize - 1
        );
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn offset_reaching_u16_max_panics() {
        let _ = Handle::ROOT.offset(u16::MAX as usize);
    }

    #[test]
    fn split_layout_mapping() {
        assert_eq!(Direction::Left.split_layout(), (Layout::Horizontal, true));
        assert_eq!(Direction::Right.split_layout(), (Layout::Horizontal, false));
        assert_eq!(Direction::Up.split_layout(), (Layout::Vertical, true));
        assert_eq!(Direction::Down.split_layout(), (Layout::Vertical, false));
    }

    #[test]
    fn enum_variants_are_distinct() {
        assert_ne!(Layout::Horizontal, Layout::Vertical);
        assert_ne!(Direction::Left, Direction::Right);
        assert_ne!(Direction::Up, Direction::Down);
        assert_ne!(Direction::Left, Direction::Up);
    }

    #[test]
    fn split_fields_and_equality() {
        let split = Split {
            layout: Layout::Horizontal,
            ratio: f16::from_f32(0.5),
            left: Handle::from_index(1),
            right: Handle::from_index(2),
        };
        assert_eq!(split.layout, Layout::Horizontal);
        assert_eq!(split.left, Handle::from_index(1));
        assert_eq!(split.right, Handle::from_index(2));

        let same = split;
        assert_eq!(split, same);

        let different = Split {
            ratio: f16::from_f32(0.25),
            ..split
        };
        assert_ne!(split, different);
    }

    #[test]
    fn ratio_round_trips_through_f16() {
        let split = Split {
            layout: Layout::Vertical,
            ratio: f16::from_f32(0.5),
            left: Handle::ROOT,
            right: Handle::from_index(1),
        };
        assert_eq!(split.ratio.to_f32(), 0.5);
    }

    #[test]
    fn slot_max_x_and_max_y() {
        // Binary-exact half fractions, so there is no decimal-rounding ambiguity.
        let slot = Slot {
            x: f16::from_f32(0.25),
            y: f16::from_f32(0.125),
            width: f16::from_f32(0.5),
            height: f16::from_f32(0.25),
        };
        assert_eq!(slot.max_x(), f16::from_f32(0.75));
        assert_eq!(slot.max_y(), f16::from_f32(0.375));
        // Compare like-for-like against the explicit half addition.
        assert_eq!(slot.max_y(), f16::from_f32(0.125) + f16::from_f32(0.25));
    }

    /// A square slot of the given size at the given position.
    fn slot(x: f32, y: f32, w: f32, h: f32) -> Slot {
        Slot {
            x: f16::from_f32(x),
            y: f16::from_f32(y),
            width: f16::from_f32(w),
            height: f16::from_f32(h),
        }
    }

    #[test]
    fn is_in_direction_basic() {
        // Target occupies [0.5, 0.75] x [0.5, 0.75].
        let target = slot(0.5, 0.5, 0.25, 0.25);

        let left = slot(0.0, 0.5, 0.25, 0.25); // max_x 0.25 <= 0.5
        assert!(left.is_in_direction(target, SpatialDirection::Left));
        assert!(!left.is_in_direction(target, SpatialDirection::Right));

        let right = slot(0.75, 0.5, 0.25, 0.25); // x 0.75 >= max_x 0.75
        assert!(right.is_in_direction(target, SpatialDirection::Right));
        assert!(!right.is_in_direction(target, SpatialDirection::Left));

        let up = slot(0.5, 0.0, 0.25, 0.25); // max_y 0.25 <= 0.5
        assert!(up.is_in_direction(target, SpatialDirection::Up));
        assert!(!up.is_in_direction(target, SpatialDirection::Down));

        let down = slot(0.5, 0.75, 0.25, 0.25); // y 0.75 >= max_y 0.75
        assert!(down.is_in_direction(target, SpatialDirection::Down));
        assert!(!down.is_in_direction(target, SpatialDirection::Up));

        // The target itself overlaps and is in no direction.
        assert!(!target.is_in_direction(target, SpatialDirection::Left));
        assert!(!target.is_in_direction(target, SpatialDirection::Right));
        assert!(!target.is_in_direction(target, SpatialDirection::Up));
        assert!(!target.is_in_direction(target, SpatialDirection::Down));
    }

    #[test]
    fn is_in_direction_boundary_touch_is_inclusive() {
        let target = slot(0.5, 0.5, 0.25, 0.25);

        // candidate.max_x() == target.x → Left (inclusive `<=`).
        let touch_left = slot(0.25, 0.5, 0.25, 0.25);
        assert_eq!(touch_left.max_x(), target.x);
        assert!(touch_left.is_in_direction(target, SpatialDirection::Left));

        // candidate.x == target.max_x() → Right (inclusive `>=`).
        let touch_right = slot(0.75, 0.5, 0.25, 0.25);
        assert_eq!(touch_right.x, target.max_x());
        assert!(touch_right.is_in_direction(target, SpatialDirection::Right));

        // candidate.max_y() == target.y → Up.
        let touch_up = slot(0.5, 0.25, 0.25, 0.25);
        assert_eq!(touch_up.max_y(), target.y);
        assert!(touch_up.is_in_direction(target, SpatialDirection::Up));

        // candidate.y == target.max_y() → Down.
        let touch_down = slot(0.5, 0.75, 0.25, 0.25);
        assert_eq!(touch_down.y, target.max_y());
        assert!(touch_down.is_in_direction(target, SpatialDirection::Down));
    }

    #[test]
    fn distance_to_uses_euclidean_geometry() {
        let target = slot(0.0, 0.0, 0.0, 0.0);

        // Zero separation.
        assert_eq!(target.distance_to(target), f16::from_f32(0.0));

        // Axis-aligned: dy = 0.5.
        let axis = slot(0.0, 0.5, 0.0, 0.0);
        assert_eq!(axis.distance_to(target), f16::from_f32(0.5));

        // Binary-exact 3-4-5 triangle: 0.75² + 1.0² = 1.5625 = 1.25².
        let diag = slot(0.75, 1.0, 0.0, 0.0);
        assert_eq!(diag.distance_to(target), f16::from_f32(1.25));
    }

    #[test]
    fn wrapped_for_shifts_one_grid() {
        let s = slot(0.25, 0.5, 0.1, 0.2);
        let one = f16::from_f32(1.0);

        let left = s.wrapped_for(SpatialDirection::Left);
        assert_eq!(left.x, s.x + one);
        assert_eq!((left.y, left.width, left.height), (s.y, s.width, s.height));

        let right = s.wrapped_for(SpatialDirection::Right);
        assert_eq!(right.x, s.x - one);
        assert_eq!(right.y, s.y);

        let up = s.wrapped_for(SpatialDirection::Up);
        assert_eq!(up.y, s.y + one);
        assert_eq!(up.x, s.x);

        let down = s.wrapped_for(SpatialDirection::Down);
        assert_eq!(down.y, s.y - one);
        assert_eq!(down.x, s.x);
    }

    // A split node payload for hand-built test trees.
    fn split(layout: Layout, left: usize, right: usize) -> Split {
        Split {
            layout,
            ratio: f16::from_f32(0.5),
            left: Handle::from_index(left),
            right: Handle::from_index(right),
        }
    }

    #[test]
    fn single_leaf_tree_queries() {
        let tree = SplitTree::new(Rc::new("view"));
        assert!(!tree.is_empty());
        assert!(!tree.is_split());
        assert_eq!(tree.deepest(Side::Left, Handle::ROOT), Handle::ROOT);
        assert_eq!(tree.deepest(Side::Right, Handle::ROOT), Handle::ROOT);
        assert_eq!(
            tree.dimensions(Handle::ROOT),
            Dimensions {
                width: 1,
                height: 1
            }
        );
    }

    #[test]
    fn empty_tree_queries() {
        let tree: SplitTree<&str> = SplitTree::empty();
        assert!(tree.is_empty());
        assert!(!tree.is_split());
    }

    #[test]
    fn horizontal_split_of_two_leaves() {
        // root = H split(left=1, right=2); nodes[1]=leaf, nodes[2]=leaf.
        let tree = SplitTree {
            nodes: vec![
                Node::Split(split(Layout::Horizontal, 1, 2)),
                Node::Leaf(Rc::new("a")),
                Node::Leaf(Rc::new("b")),
            ],
            zoomed: None,
        };
        assert!(tree.is_split());
        assert_eq!(
            tree.deepest(Side::Left, Handle::ROOT),
            Handle::from_index(1)
        );
        assert_eq!(
            tree.deepest(Side::Right, Handle::ROOT),
            Handle::from_index(2)
        );
        assert_eq!(
            tree.dimensions(Handle::ROOT),
            Dimensions {
                width: 2,
                height: 1
            }
        );
    }

    #[test]
    fn vertical_split_dimensions() {
        let tree = SplitTree {
            nodes: vec![
                Node::Split(split(Layout::Vertical, 1, 2)),
                Node::Leaf(Rc::new("a")),
                Node::Leaf(Rc::new("b")),
            ],
            zoomed: None,
        };
        assert_eq!(
            tree.dimensions(Handle::ROOT),
            Dimensions {
                width: 1,
                height: 2
            }
        );
    }

    #[test]
    fn nested_tree_deepest_and_dimensions() {
        // root = H split(left=1, right=4); node1 = V split(left=2, right=3) of two leaves; node4 =
        // leaf. Layout: a 1x2 column on the left, a single leaf on the right → width 2, height 2.
        let tree = SplitTree {
            nodes: vec![
                Node::Split(split(Layout::Horizontal, 1, 4)),
                Node::Split(split(Layout::Vertical, 2, 3)),
                Node::Leaf(Rc::new("a")),
                Node::Leaf(Rc::new("b")),
                Node::Leaf(Rc::new("c")),
            ],
            zoomed: None,
        };
        assert!(tree.is_split());
        // deepest-left descends root→node1→node2.
        assert_eq!(
            tree.deepest(Side::Left, Handle::ROOT),
            Handle::from_index(2)
        );
        // deepest-right descends root→node4 (a leaf).
        assert_eq!(
            tree.deepest(Side::Right, Handle::ROOT),
            Handle::from_index(4)
        );
        // left column is 1 wide, 2 tall; right leaf is 1x1. Horizontal: width 1+1=2, height
        // max(2,1)=2.
        assert_eq!(
            tree.dimensions(Handle::ROOT),
            Dimensions {
                width: 2,
                height: 2
            }
        );
    }

    #[test]
    fn clone_ref_counts_the_views() {
        let view = Rc::new("shared");
        let tree = SplitTree::new(Rc::clone(&view));
        // `view` + the leaf's Rc.
        assert_eq!(Rc::strong_count(&view), 2);

        let cloned = tree.clone();
        // Cloning the tree refs the view again (upstream `refNodes`).
        assert_eq!(Rc::strong_count(&view), 3);

        drop(cloned);
        // Dropping the clone unrefs it (upstream `deinit` / `viewUnref`).
        assert_eq!(Rc::strong_count(&view), 2);

        drop(tree);
        assert_eq!(Rc::strong_count(&view), 1);
    }

    /// Collect `(handle index, view)` pairs from an iterator.
    fn collect_views<V: Copy>(tree: &SplitTree<V>) -> Vec<(usize, V)> {
        tree.iter().map(|e| (e.handle.idx(), **e.view)).collect()
    }

    #[test]
    fn iterate_single_leaf() {
        let tree = SplitTree::new(Rc::new("v"));
        assert_eq!(collect_views(&tree), vec![(0, "v")]);
    }

    #[test]
    fn iterate_empty_tree() {
        let tree: SplitTree<&str> = SplitTree::empty();
        assert_eq!(collect_views(&tree), vec![]);
    }

    #[test]
    fn iterate_horizontal_split_skips_the_split() {
        let tree = SplitTree {
            nodes: vec![
                Node::Split(split(Layout::Horizontal, 1, 2)),
                Node::Leaf(Rc::new("a")),
                Node::Leaf(Rc::new("b")),
            ],
            zoomed: None,
        };
        // The split at index 0 is skipped; leaves at 1, 2 visited in order.
        assert_eq!(collect_views(&tree), vec![(1, "a"), (2, "b")]);
    }

    #[test]
    fn iterate_nested_tree_visits_all_leaves_in_arena_order() {
        let tree = SplitTree {
            nodes: vec![
                Node::Split(split(Layout::Horizontal, 1, 4)),
                Node::Split(split(Layout::Vertical, 2, 3)),
                Node::Leaf(Rc::new("a")),
                Node::Leaf(Rc::new("b")),
                Node::Leaf(Rc::new("c")),
            ],
            zoomed: None,
        };
        // Splits at 0, 1 skipped; leaves at 2, 3, 4 in arena order.
        assert_eq!(collect_views(&tree), vec![(2, "a"), (3, "b"), (4, "c")]);
    }

    #[test]
    fn zoom_sets_and_clears() {
        let mut tree = SplitTree {
            nodes: vec![
                Node::Split(split(Layout::Horizontal, 1, 2)),
                Node::Leaf(Rc::new("a")),
                Node::Leaf(Rc::new("b")),
            ],
            zoomed: None,
        };
        assert_eq!(tree.zoomed(), None);
        tree.zoom(Some(Handle::from_index(2)));
        assert_eq!(tree.zoomed(), Some(Handle::from_index(2)));
        tree.zoom(None);
        assert_eq!(tree.zoomed(), None);
    }

    #[test]
    #[should_panic(expected = "zoom handle out of range")]
    fn zoom_out_of_range_panics() {
        let mut tree = SplitTree::new(Rc::new("v"));
        tree.zoom(Some(Handle::from_index(5)));
    }

    #[test]
    fn goto_variants_are_distinct() {
        assert_ne!(Goto::Previous, Goto::Next);
        assert_ne!(Goto::PreviousWrapped, Goto::NextWrapped);
        assert_ne!(Goto::Next, Goto::Spatial(SpatialDirection::Left));
        assert_ne!(
            Goto::Spatial(SpatialDirection::Left),
            Goto::Spatial(SpatialDirection::Right)
        );
        assert_eq!(
            Goto::Spatial(SpatialDirection::Up),
            Goto::Spatial(SpatialDirection::Up)
        );
    }
}
