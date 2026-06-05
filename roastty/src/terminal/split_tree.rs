//! Foundational types for the split-pane tree (port of the vocabulary of upstream
//! `datastruct/split_tree`). The tree itself — the node arena, view ref-counting, and the
//! spatial normalization / resize logic — is deferred.

use half::f16;

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
}
