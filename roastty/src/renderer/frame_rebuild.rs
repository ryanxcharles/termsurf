#![allow(dead_code)]
// Frame rebuild planning is consumed by later renderer integration slices.

//! Renderer frame rebuild planning.
//!
//! Faithful value-level port of the front half of upstream
//! `renderer/generic.zig`'s `rebuildCells`: decide whether the cell contents
//! grid must resize, whether the rebuild is full or row-level, which rows should
//! be rebuilt/cleared/marked clean, and whether preedit text masks the cursor
//! row. Actual terminal row formatting, glyph emission, cursor drawing, and
//! `Contents` mutation remain later integration work.

use crate::renderer::cell::Contents;
use crate::renderer::size::{GridSize, Unit};
use crate::renderer::state::{Preedit, PreeditRange};
use crate::terminal::point::Coordinate;

/// Terminal render-state dirty mode. Mirrors upstream
/// `terminal.RenderState.Dirty`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RenderDirty {
    Clean,
    Partial,
    Full,
}

/// Input to the frame rebuild planner. `row_dirty` is indexed by viewport row
/// after any terminal-state/search/link updates have already run.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FrameRebuildInput<'a> {
    pub(crate) current_grid: GridSize,
    pub(crate) terminal_grid: GridSize,
    pub(crate) dirty: RenderDirty,
    pub(crate) row_dirty: &'a [bool],
    pub(crate) preedit: Option<&'a Preedit>,
    pub(crate) cursor_viewport: Option<Coordinate>,
}

/// The preedit placement planned for this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FramePreeditRange {
    pub(crate) row: Unit,
    pub(crate) range: PreeditRange,
}

/// The value-level plan that a future `rebuildCells` integration can apply to
/// `Contents` and terminal row-dirty flags before formatting rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FrameRebuildPlan {
    pub(crate) grid_changed: bool,
    pub(crate) resize_to: Option<GridSize>,
    pub(crate) effective_grid: GridSize,
    pub(crate) full_rebuild: bool,
    pub(crate) row_count: Unit,
    pub(crate) rows_to_rebuild: Vec<Unit>,
    pub(crate) reset_contents: bool,
    pub(crate) clear_rows: Vec<Unit>,
    pub(crate) rows_to_mark_clean: Vec<Unit>,
    pub(crate) preedit_range: Option<FramePreeditRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameRebuildPlanError {
    DirtyRowsTooShort { needed: usize, actual: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameRebuildApplyError {
    ContentsGridMismatch {
        expected: GridSize,
        actual: GridSize,
    },
    ResizeGridMismatch {
        resize_to: GridSize,
        effective_grid: GridSize,
    },
    ClearRowOutOfBounds {
        row: Unit,
        rows: Unit,
    },
    DirtyRowsTooShort {
        needed: usize,
        actual: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FrameRebuildApplication {
    pub(crate) resized_to: Option<GridSize>,
    pub(crate) reset_contents: bool,
    pub(crate) cleared_rows: Vec<Unit>,
    pub(crate) marked_clean_rows: Vec<Unit>,
}

impl FrameRebuildPlan {
    pub(crate) fn build(input: FrameRebuildInput<'_>) -> Result<Self, FrameRebuildPlanError> {
        let needed_dirty_rows = input.terminal_grid.rows as usize;
        if input.row_dirty.len() < needed_dirty_rows {
            return Err(FrameRebuildPlanError::DirtyRowsTooShort {
                needed: needed_dirty_rows,
                actual: input.row_dirty.len(),
            });
        }

        let grid_changed = input.current_grid != input.terminal_grid;
        let resize_to = grid_changed.then_some(input.terminal_grid);
        let effective_grid = resize_to.unwrap_or(input.current_grid);
        let row_count = input.terminal_grid.rows.min(effective_grid.rows);
        let full_rebuild = matches!(input.dirty, RenderDirty::Full) || grid_changed;

        let rows_to_rebuild: Vec<Unit> = if full_rebuild {
            (0..row_count).collect()
        } else {
            input
                .row_dirty
                .iter()
                .take(row_count as usize)
                .enumerate()
                .filter_map(|(row, dirty)| dirty.then_some(row as Unit))
                .collect()
        };

        let reset_contents = full_rebuild;
        let clear_rows = if full_rebuild {
            Vec::new()
        } else {
            rows_to_rebuild.clone()
        };
        let rows_to_mark_clean = rows_to_rebuild.clone();
        let preedit_range = plan_preedit_range(input, row_count, &rows_to_rebuild);

        Ok(Self {
            grid_changed,
            resize_to,
            effective_grid,
            full_rebuild,
            row_count,
            rows_to_rebuild,
            reset_contents,
            clear_rows,
            rows_to_mark_clean,
            preedit_range,
        })
    }

    pub(crate) fn apply_to_contents(
        &self,
        contents: &mut Contents,
        row_dirty: &mut [bool],
    ) -> Result<FrameRebuildApplication, FrameRebuildApplyError> {
        self.validate_application(contents, row_dirty)?;

        if let Some(size) = self.resize_to {
            contents.resize(size);
        }
        if self.reset_contents {
            contents.reset();
        }
        for row in &self.clear_rows {
            contents.clear(*row);
        }
        for row in &self.rows_to_mark_clean {
            row_dirty[*row as usize] = false;
        }

        Ok(FrameRebuildApplication {
            resized_to: self.resize_to,
            reset_contents: self.reset_contents,
            cleared_rows: self.clear_rows.clone(),
            marked_clean_rows: self.rows_to_mark_clean.clone(),
        })
    }

    fn validate_application(
        &self,
        contents: &Contents,
        row_dirty: &[bool],
    ) -> Result<(), FrameRebuildApplyError> {
        if let Some(resize_to) = self.resize_to {
            if resize_to != self.effective_grid {
                return Err(FrameRebuildApplyError::ResizeGridMismatch {
                    resize_to,
                    effective_grid: self.effective_grid,
                });
            }
        } else if contents.size() != self.effective_grid {
            return Err(FrameRebuildApplyError::ContentsGridMismatch {
                expected: self.effective_grid,
                actual: contents.size(),
            });
        }

        for row in &self.clear_rows {
            if *row >= self.effective_grid.rows {
                return Err(FrameRebuildApplyError::ClearRowOutOfBounds {
                    row: *row,
                    rows: self.effective_grid.rows,
                });
            }
        }

        let needed_dirty_rows = self
            .rows_to_mark_clean
            .iter()
            .copied()
            .max()
            .map_or(0, |row| row as usize + 1);
        if row_dirty.len() < needed_dirty_rows {
            return Err(FrameRebuildApplyError::DirtyRowsTooShort {
                needed: needed_dirty_rows,
                actual: row_dirty.len(),
            });
        }

        Ok(())
    }
}

fn plan_preedit_range(
    input: FrameRebuildInput<'_>,
    row_count: Unit,
    rows_to_rebuild: &[Unit],
) -> Option<FramePreeditRange> {
    let preedit = input.preedit?;
    let cursor = input.cursor_viewport?;
    let row = Unit::try_from(cursor.y).ok()?;
    if row >= row_count || cursor.x >= input.terminal_grid.columns {
        return None;
    }
    if input.terminal_grid.columns == 0 {
        return None;
    }
    if !rows_to_rebuild.contains(&row) {
        return None;
    }

    Some(FramePreeditRange {
        row,
        range: preedit.range(cursor.x, input.terminal_grid.columns - 1),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::cell::{Contents, Key};
    use crate::renderer::cursor::Style as CursorStyle;
    use crate::renderer::shader::{CellBg, CellTextAtlas, CellTextFlags, CellTextVertex};
    use crate::renderer::state::Codepoint;

    fn grid(columns: Unit, rows: Unit) -> GridSize {
        GridSize { columns, rows }
    }

    fn preedit(widths: &[bool]) -> Preedit {
        Preedit {
            codepoints: widths
                .iter()
                .map(|wide| Codepoint {
                    codepoint: 'x' as u32,
                    wide: *wide,
                })
                .collect(),
        }
    }

    fn input<'a>(
        current_grid: GridSize,
        terminal_grid: GridSize,
        dirty: RenderDirty,
        row_dirty: &'a [bool],
    ) -> FrameRebuildInput<'a> {
        FrameRebuildInput {
            current_grid,
            terminal_grid,
            dirty,
            row_dirty,
            preedit: None,
            cursor_viewport: None,
        }
    }

    fn dummy_vertex(row: Unit, marker: u8) -> CellTextVertex {
        CellTextVertex {
            glyph_pos: [marker as u32, 0],
            glyph_size: [1, 1],
            bearings: [0, 0],
            grid_pos: [0, row],
            color: [marker, marker, marker, 255],
            atlas: CellTextAtlas::Grayscale,
            flags: CellTextFlags::new(false, false),
            _padding: [0, 0],
        }
    }

    fn contents_with_rows(size: GridSize) -> Contents {
        let mut contents = Contents::default();
        contents.resize(size);
        for row in 0..size.rows {
            for col in 0..size.columns {
                *contents.bg_cell_mut(row as usize, col as usize) =
                    CellBg([row as u8 + 1, col as u8 + 1, 7, 255]);
            }
            contents.add(Key::Text, dummy_vertex(row, row as u8 + 10));
        }
        contents
    }

    #[test]
    fn full_dirty_rebuilds_all_rows_and_resets_contents() {
        let plan = FrameRebuildPlan::build(input(
            grid(4, 3),
            grid(4, 3),
            RenderDirty::Full,
            &[false, true, false],
        ))
        .expect("plan");

        assert!(!plan.grid_changed);
        assert_eq!(plan.resize_to, None);
        assert_eq!(plan.effective_grid, grid(4, 3));
        assert!(plan.full_rebuild);
        assert_eq!(plan.row_count, 3);
        assert_eq!(plan.rows_to_rebuild, vec![0, 1, 2]);
        assert!(plan.reset_contents);
        assert!(plan.clear_rows.is_empty());
        assert_eq!(plan.rows_to_mark_clean, vec![0, 1, 2]);
    }

    #[test]
    fn partial_rebuilds_only_dirty_rows_and_clears_them() {
        let plan = FrameRebuildPlan::build(input(
            grid(4, 4),
            grid(4, 4),
            RenderDirty::Partial,
            &[false, true, false, true],
        ))
        .expect("plan");

        assert!(!plan.full_rebuild);
        assert_eq!(plan.rows_to_rebuild, vec![1, 3]);
        assert!(!plan.reset_contents);
        assert_eq!(plan.clear_rows, vec![1, 3]);
        assert_eq!(plan.rows_to_mark_clean, vec![1, 3]);
    }

    #[test]
    fn clean_still_rebuilds_dirty_rows() {
        let plan = FrameRebuildPlan::build(input(
            grid(4, 3),
            grid(4, 3),
            RenderDirty::Clean,
            &[false, true, false],
        ))
        .expect("plan");

        assert!(!plan.full_rebuild);
        assert_eq!(plan.rows_to_rebuild, vec![1]);
        assert_eq!(plan.clear_rows, vec![1]);
        assert_eq!(plan.rows_to_mark_clean, vec![1]);
    }

    #[test]
    fn grid_growth_uses_post_resize_rows() {
        let plan = FrameRebuildPlan::build(input(
            grid(4, 2),
            grid(4, 5),
            RenderDirty::Clean,
            &[false, false, false, false, false],
        ))
        .expect("plan");

        assert!(plan.grid_changed);
        assert_eq!(plan.resize_to, Some(grid(4, 5)));
        assert_eq!(plan.effective_grid, grid(4, 5));
        assert!(plan.full_rebuild);
        assert_eq!(plan.row_count, 5);
        assert_eq!(plan.rows_to_rebuild, vec![0, 1, 2, 3, 4]);
        assert!(plan.reset_contents);
    }

    #[test]
    fn grid_shrink_uses_post_resize_rows() {
        let plan = FrameRebuildPlan::build(input(
            grid(4, 5),
            grid(4, 2),
            RenderDirty::Partial,
            &[false, false],
        ))
        .expect("plan");

        assert!(plan.grid_changed);
        assert_eq!(plan.resize_to, Some(grid(4, 2)));
        assert_eq!(plan.effective_grid, grid(4, 2));
        assert_eq!(plan.row_count, 2);
        assert_eq!(plan.rows_to_rebuild, vec![0, 1]);
    }

    #[test]
    fn row_count_clamps_to_effective_grid_rows() {
        let plan = FrameRebuildPlan::build(input(
            grid(4, 2),
            grid(4, 3),
            RenderDirty::Full,
            &[false, false, false],
        ))
        .expect("plan");

        assert_eq!(plan.row_count, 3);
        assert_eq!(plan.rows_to_rebuild, vec![0, 1, 2]);
    }

    #[test]
    fn short_dirty_slice_errors() {
        let err = FrameRebuildPlan::build(input(
            grid(4, 3),
            grid(4, 3),
            RenderDirty::Full,
            &[false, false],
        ))
        .expect_err("short row dirty slice should error");

        assert_eq!(
            err,
            FrameRebuildPlanError::DirtyRowsTooShort {
                needed: 3,
                actual: 2,
            }
        );
    }

    #[test]
    fn extra_dirty_flags_are_ignored() {
        let plan = FrameRebuildPlan::build(input(
            grid(4, 2),
            grid(4, 2),
            RenderDirty::Clean,
            &[false, true, true],
        ))
        .expect("plan");

        assert_eq!(plan.rows_to_rebuild, vec![1]);
    }

    #[test]
    fn zero_sized_grids_plan_no_rows_or_preedit() {
        let p = preedit(&[false]);
        let plan = FrameRebuildPlan::build(FrameRebuildInput {
            current_grid: grid(0, 0),
            terminal_grid: grid(0, 0),
            dirty: RenderDirty::Full,
            row_dirty: &[],
            preedit: Some(&p),
            cursor_viewport: Some(Coordinate::new(0, 0)),
        })
        .expect("plan");

        assert_eq!(plan.row_count, 0);
        assert!(plan.rows_to_rebuild.is_empty());
        assert_eq!(plan.preedit_range, None);
    }

    #[test]
    fn preedit_range_is_planned_for_rebuilt_cursor_row() {
        let p = preedit(&[false, true]);
        let plan = FrameRebuildPlan::build(FrameRebuildInput {
            current_grid: grid(4, 3),
            terminal_grid: grid(4, 3),
            dirty: RenderDirty::Partial,
            row_dirty: &[false, true, false],
            preedit: Some(&p),
            cursor_viewport: Some(Coordinate::new(2, 1)),
        })
        .expect("plan");

        assert_eq!(
            plan.preedit_range,
            Some(FramePreeditRange {
                row: 1,
                range: PreeditRange {
                    start: 1,
                    end: 3,
                    cp_offset: 0,
                },
            })
        );
    }

    #[test]
    fn preedit_range_is_planned_on_full_rebuild_even_when_row_clean() {
        let p = preedit(&[false]);
        let plan = FrameRebuildPlan::build(FrameRebuildInput {
            current_grid: grid(4, 2),
            terminal_grid: grid(4, 2),
            dirty: RenderDirty::Full,
            row_dirty: &[false, false],
            preedit: Some(&p),
            cursor_viewport: Some(Coordinate::new(1, 1)),
        })
        .expect("plan");

        assert!(plan.preedit_range.is_some());
    }

    #[test]
    fn preedit_range_is_skipped_when_partial_cursor_row_clean() {
        let p = preedit(&[false]);
        let plan = FrameRebuildPlan::build(FrameRebuildInput {
            current_grid: grid(4, 2),
            terminal_grid: grid(4, 2),
            dirty: RenderDirty::Partial,
            row_dirty: &[true, false],
            preedit: Some(&p),
            cursor_viewport: Some(Coordinate::new(1, 1)),
        })
        .expect("plan");

        assert_eq!(plan.rows_to_rebuild, vec![0]);
        assert_eq!(plan.preedit_range, None);
    }

    #[test]
    fn preedit_range_is_skipped_when_cursor_outside_viewport() {
        let p = preedit(&[false]);
        let plan = FrameRebuildPlan::build(FrameRebuildInput {
            current_grid: grid(4, 2),
            terminal_grid: grid(4, 2),
            dirty: RenderDirty::Full,
            row_dirty: &[false, false],
            preedit: Some(&p),
            cursor_viewport: Some(Coordinate::new(4, 2)),
        })
        .expect("plan");

        assert_eq!(plan.preedit_range, None);
    }

    #[test]
    fn apply_resizes_before_reset_and_reports_post_size() {
        let plan = FrameRebuildPlan::build(input(
            grid(2, 1),
            grid(3, 2),
            RenderDirty::Clean,
            &[true, true],
        ))
        .expect("plan");
        let mut contents = contents_with_rows(grid(2, 1));
        let mut row_dirty = vec![true, true];

        let applied = plan
            .apply_to_contents(&mut contents, &mut row_dirty)
            .expect("apply plan");

        assert_eq!(contents.size(), grid(3, 2));
        assert_eq!(applied.resized_to, Some(grid(3, 2)));
        assert!(applied.reset_contents);
        assert_eq!(applied.marked_clean_rows, vec![0, 1]);
        assert_eq!(row_dirty, vec![false, false]);
        for row in 0..2 {
            for col in 0..3 {
                assert_eq!(*contents.bg_cell(row, col), CellBg([0, 0, 0, 0]));
            }
        }
    }

    #[test]
    fn apply_full_rebuild_resets_cursor_reserved_lists() {
        let plan = FrameRebuildPlan::build(input(
            grid(2, 2),
            grid(2, 2),
            RenderDirty::Full,
            &[true, true],
        ))
        .expect("plan");
        let mut contents = contents_with_rows(grid(2, 2));
        contents.set_cursor(Some(dummy_vertex(0, 99)), Some(CursorStyle::Block));
        assert!(contents.get_cursor_glyph().is_some());
        let mut row_dirty = vec![true, true];

        plan.apply_to_contents(&mut contents, &mut row_dirty)
            .expect("apply plan");

        assert_eq!(contents.get_cursor_glyph(), None);
        assert!(contents.fg_rows().iter().all(Vec::is_empty));
        assert_eq!(row_dirty, vec![false, false]);
    }

    #[test]
    fn apply_partial_clears_only_planned_rows() {
        let plan = FrameRebuildPlan::build(input(
            grid(2, 3),
            grid(2, 3),
            RenderDirty::Partial,
            &[false, true, false],
        ))
        .expect("plan");
        let mut contents = contents_with_rows(grid(2, 3));
        let preserved_bg = *contents.bg_cell(0, 0);
        let preserved_fg = contents.fg_rows()[1].clone();
        let mut row_dirty = vec![false, true, false];

        let applied = plan
            .apply_to_contents(&mut contents, &mut row_dirty)
            .expect("apply plan");

        assert_eq!(applied.cleared_rows, vec![1]);
        assert_eq!(applied.marked_clean_rows, vec![1]);
        assert_eq!(*contents.bg_cell(0, 0), preserved_bg);
        assert_eq!(contents.fg_rows()[1], preserved_fg);
        assert_eq!(*contents.bg_cell(1, 0), CellBg([0, 0, 0, 0]));
        assert!(contents.fg_rows()[2].is_empty());
        assert_eq!(row_dirty, vec![false, false, false]);
    }

    #[test]
    fn apply_clean_empty_plan_is_noop() {
        let plan = FrameRebuildPlan::build(input(
            grid(2, 2),
            grid(2, 2),
            RenderDirty::Clean,
            &[false, false],
        ))
        .expect("plan");
        let mut contents = contents_with_rows(grid(2, 2));
        let bg = contents.bg_cells().to_vec();
        let fg = contents.fg_rows().to_vec();
        let mut row_dirty = vec![false, false];

        let applied = plan
            .apply_to_contents(&mut contents, &mut row_dirty)
            .expect("apply plan");

        assert_eq!(applied.resized_to, None);
        assert!(!applied.reset_contents);
        assert!(applied.cleared_rows.is_empty());
        assert!(applied.marked_clean_rows.is_empty());
        assert_eq!(contents.bg_cells(), bg);
        assert_eq!(contents.fg_rows(), fg);
        assert_eq!(row_dirty, vec![false, false]);
    }

    #[test]
    fn apply_rejects_short_dirty_slice_without_mutation() {
        let plan = FrameRebuildPlan::build(input(
            grid(2, 3),
            grid(2, 3),
            RenderDirty::Full,
            &[true, true, true],
        ))
        .expect("plan");
        let mut contents = contents_with_rows(grid(2, 3));
        let bg = contents.bg_cells().to_vec();
        let fg = contents.fg_rows().to_vec();
        let mut row_dirty = vec![true, true];

        let err = plan
            .apply_to_contents(&mut contents, &mut row_dirty)
            .expect_err("short dirty slice should error before mutation");

        assert_eq!(
            err,
            FrameRebuildApplyError::DirtyRowsTooShort {
                needed: 3,
                actual: 2,
            }
        );
        assert_eq!(contents.bg_cells(), bg);
        assert_eq!(contents.fg_rows(), fg);
        assert_eq!(row_dirty, vec![true, true]);
    }

    #[test]
    fn apply_rejects_out_of_bounds_clear_row_without_mutation() {
        let mut plan = FrameRebuildPlan::build(input(
            grid(2, 2),
            grid(2, 2),
            RenderDirty::Partial,
            &[false, true],
        ))
        .expect("plan");
        plan.clear_rows = vec![2];
        let mut contents = contents_with_rows(grid(2, 2));
        let bg = contents.bg_cells().to_vec();
        let fg = contents.fg_rows().to_vec();
        let mut row_dirty = vec![false, true];

        let err = plan
            .apply_to_contents(&mut contents, &mut row_dirty)
            .expect_err("invalid clear row should error before mutation");

        assert_eq!(
            err,
            FrameRebuildApplyError::ClearRowOutOfBounds { row: 2, rows: 2 }
        );
        assert_eq!(contents.bg_cells(), bg);
        assert_eq!(contents.fg_rows(), fg);
        assert_eq!(row_dirty, vec![false, true]);
    }

    #[test]
    fn apply_rejects_grid_mismatch_without_mutation() {
        let plan = FrameRebuildPlan::build(input(
            grid(2, 2),
            grid(2, 2),
            RenderDirty::Clean,
            &[false, false],
        ))
        .expect("plan");
        let mut contents = contents_with_rows(grid(3, 2));
        let mut row_dirty = vec![false, false];

        let err = plan
            .apply_to_contents(&mut contents, &mut row_dirty)
            .expect_err("contents grid mismatch should error");

        assert_eq!(
            err,
            FrameRebuildApplyError::ContentsGridMismatch {
                expected: grid(2, 2),
                actual: grid(3, 2),
            }
        );
        assert_eq!(contents.size(), grid(3, 2));
    }

    #[test]
    fn apply_rejects_resize_effective_grid_mismatch_without_mutation() {
        let mut plan = FrameRebuildPlan::build(input(
            grid(2, 2),
            grid(3, 2),
            RenderDirty::Clean,
            &[false, false],
        ))
        .expect("plan");
        plan.effective_grid = grid(4, 2);
        let mut contents = contents_with_rows(grid(2, 2));
        let mut row_dirty = vec![false, false];

        let err = plan
            .apply_to_contents(&mut contents, &mut row_dirty)
            .expect_err("resize/effective mismatch should error");

        assert_eq!(
            err,
            FrameRebuildApplyError::ResizeGridMismatch {
                resize_to: grid(3, 2),
                effective_grid: grid(4, 2),
            }
        );
        assert_eq!(contents.size(), grid(2, 2));
    }
}
