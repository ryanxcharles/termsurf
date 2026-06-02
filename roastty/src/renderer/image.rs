#![allow(dead_code)]
// This renderer foundation is consumed by later renderer slices.

use std::collections::{BTreeMap, BTreeSet};

use crate::KittyGraphicsRenderPlacementSnapshot;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ImageId {
    Kitty(u32),
    Overlay,
}

impl ImageId {
    pub(crate) fn z_less_than(self, other: Self) -> bool {
        match (self, other) {
            (ImageId::Kitty(lhs), ImageId::Kitty(rhs)) => lhs < rhs,
            (ImageId::Kitty(_), ImageId::Overlay) => true,
            (ImageId::Overlay, ImageId::Kitty(_)) => false,
            (ImageId::Overlay, ImageId::Overlay) => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PixelFormat {
    Gray,
    GrayAlpha,
    Rgb,
    Rgba,
}

impl PixelFormat {
    pub(crate) fn bytes_per_pixel(self) -> usize {
        match self {
            PixelFormat::Gray => 1,
            PixelFormat::GrayAlpha => 2,
            PixelFormat::Rgb => 3,
            PixelFormat::Rgba => 4,
        }
    }

    fn from_kitty_snapshot_format(format: i32) -> Option<Self> {
        match format {
            0 => Some(PixelFormat::Rgb),
            1 => Some(PixelFormat::Rgba),
            3 => Some(PixelFormat::GrayAlpha),
            4 => Some(PixelFormat::Gray),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PendingImage {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) pixel_format: PixelFormat,
    pub(crate) data: Vec<u8>,
}

impl PendingImage {
    pub(crate) fn len(&self) -> usize {
        self.width as usize * self.height as usize * self.pixel_format.bytes_per_pixel()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RendererImage {
    Pending(PendingImage),
    Replace(PendingImage),
    UnloadPending(PendingImage),
    UnloadReplace(PendingImage),
}

impl RendererImage {
    pub(crate) fn is_pending(&self) -> bool {
        matches!(self, RendererImage::Pending(_) | RendererImage::Replace(_))
    }

    pub(crate) fn is_unloading(&self) -> bool {
        matches!(
            self,
            RendererImage::UnloadPending(_) | RendererImage::UnloadReplace(_)
        )
    }

    pub(crate) fn pending_image(&self) -> &PendingImage {
        match self {
            RendererImage::Pending(image)
            | RendererImage::Replace(image)
            | RendererImage::UnloadPending(image)
            | RendererImage::UnloadReplace(image) => image,
        }
    }

    pub(crate) fn mark_for_unload(&mut self) {
        let image = self.pending_image().clone();
        *self = match self {
            RendererImage::Replace(_) | RendererImage::UnloadReplace(_) => {
                RendererImage::UnloadReplace(image)
            }
            RendererImage::Pending(_) | RendererImage::UnloadPending(_) => {
                RendererImage::UnloadPending(image)
            }
        };
    }

    pub(crate) fn mark_for_replace(&mut self, pending: PendingImage) {
        *self = match self {
            RendererImage::UnloadPending(_) | RendererImage::UnloadReplace(_) => {
                RendererImage::UnloadReplace(pending)
            }
            RendererImage::Pending(_) | RendererImage::Replace(_) => {
                RendererImage::Replace(pending)
            }
        };
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Placement {
    pub(crate) image_id: ImageId,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) z: i32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) cell_offset_x: u32,
    pub(crate) cell_offset_y: u32,
    pub(crate) source_x: u32,
    pub(crate) source_y: u32,
    pub(crate) source_width: u32,
    pub(crate) source_height: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ImageState {
    pub(crate) images: BTreeMap<ImageId, RendererImage>,
    pub(crate) kitty_placements: Vec<Placement>,
    pub(crate) kitty_bg_end: u32,
    pub(crate) kitty_text_end: u32,
    pub(crate) kitty_virtual: bool,
}

impl ImageState {
    pub(crate) fn update_kitty_from_render_placements(
        &mut self,
        placements: &[KittyGraphicsRenderPlacementSnapshot],
    ) {
        self.kitty_placements.clear();
        self.kitty_virtual = false;

        let present = placements
            .iter()
            .map(|placement| ImageId::Kitty(placement.info.image_id))
            .collect::<BTreeSet<_>>();
        for (id, image) in self.images.iter_mut() {
            if matches!(id, ImageId::Kitty(_)) && !present.contains(id) {
                image.mark_for_unload();
            }
        }

        for placement in placements {
            let id = ImageId::Kitty(placement.info.image_id);
            let Some(pending) = pending_image_from_kitty_snapshot(placement) else {
                continue;
            };
            match self.images.get_mut(&id) {
                Some(image) if image.pending_image() == &pending => {
                    if image.is_unloading() {
                        *image = RendererImage::Pending(pending);
                    }
                }
                Some(image) => {
                    *image = RendererImage::Replace(pending);
                }
                None => {
                    self.images.insert(id, RendererImage::Pending(pending));
                }
            }

            self.kitty_virtual |= placement.info.is_virtual;
            self.kitty_placements.push(Placement {
                image_id: id,
                x: placement.info.viewport_col,
                y: placement.info.viewport_row,
                z: placement.info.z,
                width: placement.info.pixel_width,
                height: placement.info.pixel_height,
                cell_offset_x: placement.info.x_offset,
                cell_offset_y: placement.info.y_offset,
                source_x: placement.info.source_x,
                source_y: placement.info.source_y,
                source_width: placement.info.source_width,
                source_height: placement.info.source_height,
            });
        }

        self.kitty_placements.sort_by(|lhs, rhs| {
            lhs.z.cmp(&rhs.z).then_with(|| {
                if lhs.image_id.z_less_than(rhs.image_id) {
                    std::cmp::Ordering::Less
                } else if rhs.image_id.z_less_than(lhs.image_id) {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            })
        });

        let bg_limit = i32::MIN / 2;
        self.kitty_bg_end = first_index_at_or_after(&self.kitty_placements, bg_limit);
        self.kitty_text_end = first_index_at_or_after(&self.kitty_placements, 0);
    }
}

fn pending_image_from_kitty_snapshot(
    placement: &KittyGraphicsRenderPlacementSnapshot,
) -> Option<PendingImage> {
    Some(PendingImage {
        width: placement.image.width,
        height: placement.image.height,
        pixel_format: PixelFormat::from_kitty_snapshot_format(placement.image.format)?,
        data: placement.image.data.clone(),
    })
}

fn first_index_at_or_after(placements: &[Placement], z: i32) -> u32 {
    placements
        .iter()
        .position(|placement| placement.z >= z)
        .unwrap_or(placements.len()) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        KittyGraphicsImageSnapshot, KittyGraphicsRenderPlacementSnapshot,
        RoasttyKittyGraphicsRenderPlacementInfo,
    };

    fn snapshot(
        image_id: u32,
        is_virtual: bool,
        z: i32,
        viewport_col: i32,
        viewport_row: i32,
        data: &[u8],
    ) -> KittyGraphicsRenderPlacementSnapshot {
        KittyGraphicsRenderPlacementSnapshot {
            image: KittyGraphicsImageSnapshot {
                id: image_id,
                number: 0,
                width: 1,
                height: data.len() as u32 / 4,
                format: 1,
                compression: 0,
                data: data.to_vec(),
            },
            info: RoasttyKittyGraphicsRenderPlacementInfo {
                size: std::mem::size_of::<RoasttyKittyGraphicsRenderPlacementInfo>(),
                image_id,
                placement_id: 1,
                is_virtual,
                x_offset: 2,
                y_offset: 3,
                pixel_width: 4,
                pixel_height: 5,
                grid_cols: 1,
                grid_rows: 1,
                viewport_col,
                viewport_row,
                viewport_visible: true,
                source_x: 6,
                source_y: 7,
                source_width: 8,
                source_height: 9,
                z,
            },
            virtual_row: 0,
            virtual_col: 0,
            source_group: crate::KittyGraphicsRenderPlacementSourceGroup::Pinned,
            discovery_index: 0,
        }
    }

    #[test]
    fn renderer_image_id_z_tie_breaking_matches_upstream_shape() {
        assert!(ImageId::Kitty(1).z_less_than(ImageId::Kitty(2)));
        assert!(!ImageId::Kitty(2).z_less_than(ImageId::Kitty(1)));
        assert!(ImageId::Kitty(1).z_less_than(ImageId::Overlay));
        assert!(!ImageId::Overlay.z_less_than(ImageId::Kitty(1)));
        assert!(!ImageId::Overlay.z_less_than(ImageId::Overlay));
    }

    #[test]
    fn renderer_pending_image_owns_bytes_and_reports_len() {
        let source = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let image = PendingImage {
            width: 1,
            height: 2,
            pixel_format: PixelFormat::Rgba,
            data: source.clone(),
        };
        assert_eq!(image.len(), 8);
        drop(source);
        assert_eq!(image.data, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn renderer_image_state_updates_pinned_kitty_placements() {
        let mut state = ImageState::default();
        state.update_kitty_from_render_placements(&[snapshot(7, false, 1, 10, 11, &[1, 2, 3, 4])]);

        assert_eq!(state.images.len(), 1);
        assert!(matches!(
            state.images.get(&ImageId::Kitty(7)),
            Some(RendererImage::Pending(_))
        ));
        assert_eq!(
            state.kitty_placements,
            vec![Placement {
                image_id: ImageId::Kitty(7),
                x: 10,
                y: 11,
                z: 1,
                width: 4,
                height: 5,
                cell_offset_x: 2,
                cell_offset_y: 3,
                source_x: 6,
                source_y: 7,
                source_width: 8,
                source_height: 9,
            }]
        );
        assert!(!state.kitty_virtual);
    }

    #[test]
    fn renderer_image_state_tracks_virtual_placements() {
        let mut state = ImageState::default();
        state.update_kitty_from_render_placements(&[snapshot(7, true, -1, 0, 0, &[1, 2, 3, 4])]);
        assert!(state.kitty_virtual);
    }

    #[test]
    fn renderer_image_state_handles_duplicate_same_frame_image_ids() {
        let mut state = ImageState::default();
        state.update_kitty_from_render_placements(&[
            snapshot(7, false, 0, 0, 0, &[1, 2, 3, 4]),
            snapshot(7, false, 1, 1, 0, &[1, 2, 3, 4]),
        ]);

        assert_eq!(state.images.len(), 1);
        assert!(matches!(
            state.images.get(&ImageId::Kitty(7)),
            Some(RendererImage::Pending(_))
        ));
        assert_eq!(state.kitty_placements.len(), 2);
    }

    #[test]
    fn renderer_image_state_sorts_and_splits_kitty_layers() {
        let mut state = ImageState::default();
        state.update_kitty_from_render_placements(&[
            snapshot(3, false, 2, 0, 0, &[1, 2, 3, 4]),
            snapshot(2, false, 2, 0, 0, &[1, 2, 3, 4]),
            snapshot(1, false, i32::MIN / 2 - 1, 0, 0, &[1, 2, 3, 4]),
            snapshot(4, false, -1, 0, 0, &[1, 2, 3, 4]),
        ]);

        assert_eq!(
            state
                .kitty_placements
                .iter()
                .map(|placement| placement.image_id)
                .collect::<Vec<_>>(),
            vec![
                ImageId::Kitty(1),
                ImageId::Kitty(4),
                ImageId::Kitty(2),
                ImageId::Kitty(3)
            ]
        );
        assert_eq!(state.kitty_bg_end, 1);
        assert_eq!(state.kitty_text_end, 2);
    }

    #[test]
    fn renderer_image_state_reuses_unchanged_image_records() {
        let mut state = ImageState::default();
        state.update_kitty_from_render_placements(&[snapshot(7, false, 0, 0, 0, &[1, 2, 3, 4])]);
        state.update_kitty_from_render_placements(&[snapshot(7, false, 0, 1, 0, &[1, 2, 3, 4])]);

        assert!(matches!(
            state.images.get(&ImageId::Kitty(7)),
            Some(RendererImage::Pending(_))
        ));
        assert_eq!(state.kitty_placements.len(), 1);
        assert_eq!(state.kitty_placements[0].x, 1);
    }

    #[test]
    fn renderer_image_state_marks_changed_images_for_replace() {
        let mut state = ImageState::default();
        state.update_kitty_from_render_placements(&[snapshot(7, false, 0, 0, 0, &[1, 2, 3, 4])]);
        state.update_kitty_from_render_placements(&[snapshot(7, false, 0, 0, 0, &[4, 3, 2, 1])]);

        assert!(matches!(
            state.images.get(&ImageId::Kitty(7)),
            Some(RendererImage::Replace(_))
        ));
    }

    #[test]
    fn renderer_image_state_marks_removed_images_for_unload() {
        let mut state = ImageState::default();
        state.update_kitty_from_render_placements(&[snapshot(7, false, 0, 0, 0, &[1, 2, 3, 4])]);
        state.update_kitty_from_render_placements(&[]);

        let image = state.images.get(&ImageId::Kitty(7)).unwrap();
        assert!(image.is_unloading());
        assert!(state.kitty_placements.is_empty());
    }

    #[test]
    fn renderer_image_state_reappearing_image_cancels_unload() {
        let mut state = ImageState::default();
        let placement = snapshot(7, false, 0, 0, 0, &[1, 2, 3, 4]);
        state.update_kitty_from_render_placements(std::slice::from_ref(&placement));
        state.update_kitty_from_render_placements(&[]);

        assert!(state.images.get(&ImageId::Kitty(7)).unwrap().is_unloading());

        state.update_kitty_from_render_placements(&[placement]);

        assert!(matches!(
            state.images.get(&ImageId::Kitty(7)),
            Some(RendererImage::Pending(_))
        ));
        assert_eq!(state.kitty_placements.len(), 1);
    }
}
