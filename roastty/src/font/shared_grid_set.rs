//! Shared ownership set for expensive [`SharedGrid`] instances.
//!
//! This is the ownership/refcount/locking foundation of upstream
//! `font/SharedGridSet.zig`. Roastty does not have the full upstream
//! config-derived font key yet, so the set is generic over the key and accepts a
//! caller-supplied grid constructor.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use crate::config;
use crate::font::codepoint_map::CodepointMap;
use crate::font::codepoint_resolver::CodepointResolver;
use crate::font::collection::{Collection, CompleteError, SetPointSizeError, SyntheticStyle};
use crate::font::discovery::{Descriptor, Variation};
use crate::font::face::coretext::Face;
use crate::font::metrics::{Key as MetricKey, Modifier, ModifierSet};
use crate::font::shared_grid::SharedGrid;
use crate::font::Style;

/// A shared grid reference returned by [`SharedGridSet::ref_grid`].
pub(crate) struct SharedGridHandle<K> {
    key: K,
    grid: Arc<Mutex<SharedGrid>>,
}

impl<K> SharedGridHandle<K> {
    pub(crate) fn key(&self) -> &K {
        &self.key
    }

    pub(crate) fn grid(&self) -> &Arc<Mutex<SharedGrid>> {
        &self.grid
    }
}

struct ReffedGrid {
    grid: Arc<Mutex<SharedGrid>>,
    refs: usize,
}

/// Result of releasing a grid reference from the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DerefResult {
    /// No grid existed for the key.
    Missing,
    /// The refcount was decremented and the grid remains cached.
    Decremented,
    /// The final reference was released and the grid was removed.
    Removed,
}

/// A keyed set of shared font grids with explicit set-owned refcounts.
pub(crate) struct SharedGridSet<K> {
    grids: Mutex<HashMap<K, ReffedGrid>>,
}

impl<K> Default for SharedGridSet<K> {
    fn default() -> Self {
        SharedGridSet {
            grids: Mutex::new(HashMap::new()),
        }
    }
}

impl<K> SharedGridSet<K>
where
    K: Clone + Eq + Hash,
{
    pub(crate) fn new() -> SharedGridSet<K> {
        SharedGridSet::default()
    }

    /// Returns the number of cached grids.
    pub(crate) fn count(&self) -> usize {
        self.grids
            .lock()
            .expect("shared grid set mutex poisoned")
            .len()
    }

    /// References the grid for `key`, constructing and caching one if needed.
    pub(crate) fn ref_grid<F>(&self, key: K, make_grid: F) -> SharedGridHandle<K>
    where
        F: FnOnce() -> SharedGrid,
    {
        let mut grids = self.grids.lock().expect("shared grid set mutex poisoned");

        if let Some(reffed) = grids.get_mut(&key) {
            reffed.refs = reffed
                .refs
                .checked_add(1)
                .expect("shared grid refcount overflow");
            return SharedGridHandle {
                key,
                grid: Arc::clone(&reffed.grid),
            };
        }

        let grid = Arc::new(Mutex::new(make_grid()));
        grids.insert(
            key.clone(),
            ReffedGrid {
                grid: Arc::clone(&grid),
                refs: 1,
            },
        );

        SharedGridHandle { key, grid }
    }

    /// Releases one reference for `key`.
    pub(crate) fn deref_grid(&self, key: &K) -> DerefResult {
        let mut grids = self.grids.lock().expect("shared grid set mutex poisoned");

        let Some(reffed) = grids.get_mut(key) else {
            return DerefResult::Missing;
        };

        if reffed.refs > 1 {
            reffed.refs -= 1;
            return DerefResult::Decremented;
        }

        grids.remove(key);
        DerefResult::Removed
    }

    #[cfg(test)]
    fn ref_count(&self, key: &K) -> Option<usize> {
        self.grids
            .lock()
            .expect("shared grid set mutex poisoned")
            .get(key)
            .map(|reffed| reffed.refs)
    }
}

/// Font config snapshot needed to build a shared grid key.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DerivedConfig {
    pub font_family: config::RepeatableString,
    pub font_family_bold: config::RepeatableString,
    pub font_family_italic: config::RepeatableString,
    pub font_family_bold_italic: config::RepeatableString,
    pub font_style: config::FontStyle,
    pub font_style_bold: config::FontStyle,
    pub font_style_italic: config::FontStyle,
    pub font_style_bold_italic: config::FontStyle,
    pub font_variation: config::RepeatableFontVariation,
    pub font_variation_bold: config::RepeatableFontVariation,
    pub font_variation_italic: config::RepeatableFontVariation,
    pub font_variation_bold_italic: config::RepeatableFontVariation,
    pub font_codepoint_map: config::RepeatableCodepointMap,
    pub font_synthetic_style: config::FontSyntheticStyle,
    pub adjust_cell_width: Option<Modifier>,
    pub adjust_cell_height: Option<Modifier>,
    pub adjust_font_baseline: Option<Modifier>,
    pub adjust_underline_position: Option<Modifier>,
    pub adjust_underline_thickness: Option<Modifier>,
    pub adjust_strikethrough_position: Option<Modifier>,
    pub adjust_strikethrough_thickness: Option<Modifier>,
    pub adjust_overline_position: Option<Modifier>,
    pub adjust_overline_thickness: Option<Modifier>,
    pub adjust_cursor_thickness: Option<Modifier>,
    pub adjust_cursor_height: Option<Modifier>,
    pub adjust_box_thickness: Option<Modifier>,
    pub adjust_icon_height: Option<Modifier>,
}

impl DerivedConfig {
    pub(crate) fn from_config(config: &config::Config) -> DerivedConfig {
        DerivedConfig {
            font_family: config.font_family.clone(),
            font_family_bold: config.font_family_bold.clone(),
            font_family_italic: config.font_family_italic.clone(),
            font_family_bold_italic: config.font_family_bold_italic.clone(),
            font_style: config.font_style.clone(),
            font_style_bold: config.font_style_bold.clone(),
            font_style_italic: config.font_style_italic.clone(),
            font_style_bold_italic: config.font_style_bold_italic.clone(),
            font_variation: config.font_variation.clone(),
            font_variation_bold: config.font_variation_bold.clone(),
            font_variation_italic: config.font_variation_italic.clone(),
            font_variation_bold_italic: config.font_variation_bold_italic.clone(),
            font_codepoint_map: config.font_codepoint_map.clone(),
            font_synthetic_style: config.font_synthetic_style,
            adjust_cell_width: config.adjust_cell_width,
            adjust_cell_height: config.adjust_cell_height,
            adjust_font_baseline: config.adjust_font_baseline,
            adjust_underline_position: config.adjust_underline_position,
            adjust_underline_thickness: config.adjust_underline_thickness,
            adjust_strikethrough_position: config.adjust_strikethrough_position,
            adjust_strikethrough_thickness: config.adjust_strikethrough_thickness,
            adjust_overline_position: config.adjust_overline_position,
            adjust_overline_thickness: config.adjust_overline_thickness,
            adjust_cursor_thickness: config.adjust_cursor_thickness,
            adjust_cursor_height: config.adjust_cursor_height,
            adjust_box_thickness: config.adjust_box_thickness,
            adjust_icon_height: config.adjust_icon_height,
        }
    }
}

/// The config-derived key for a shared font grid.
#[derive(Debug, Clone)]
pub(crate) struct Key {
    descriptors: Vec<Descriptor>,
    style_offsets: [usize; 4],
    codepoint_map: CodepointMap,
    metric_modifiers: ModifierSet,
    font_size_points: f32,
}

impl Key {
    pub(crate) fn new(config: &DerivedConfig, font_size_points: f32) -> Key {
        let mut descriptors = Vec::new();

        append_descriptors(
            &mut descriptors,
            &config.font_family,
            &config.font_style,
            Style::Regular,
            font_size_points,
            &config.font_variation,
        );
        let regular_offset = descriptors.len();
        append_descriptors(
            &mut descriptors,
            &config.font_family_bold,
            &config.font_style_bold,
            Style::Bold,
            font_size_points,
            &config.font_variation_bold,
        );
        let bold_offset = descriptors.len();
        append_descriptors(
            &mut descriptors,
            &config.font_family_italic,
            &config.font_style_italic,
            Style::Italic,
            font_size_points,
            &config.font_variation_italic,
        );
        let italic_offset = descriptors.len();
        append_descriptors(
            &mut descriptors,
            &config.font_family_bold_italic,
            &config.font_style_bold_italic,
            Style::BoldItalic,
            font_size_points,
            &config.font_variation_bold_italic,
        );
        let bold_italic_offset = descriptors.len();

        Key {
            descriptors,
            style_offsets: [
                regular_offset,
                bold_offset,
                italic_offset,
                bold_italic_offset,
            ],
            codepoint_map: config.font_codepoint_map.map.clone(),
            metric_modifiers: metric_modifiers_from_config(config),
            font_size_points,
        }
    }

    pub(crate) fn descriptors_for_style(&self, style: Style) -> &[Descriptor] {
        let idx = style as usize;
        let start = if idx == 0 {
            0
        } else {
            self.style_offsets[idx - 1]
        };
        let end = self.style_offsets[idx];
        &self.descriptors[start..end]
    }

    pub(crate) fn codepoint_map(&self) -> &CodepointMap {
        &self.codepoint_map
    }

    pub(crate) fn metric_modifiers(&self) -> &ModifierSet {
        &self.metric_modifiers
    }

    pub(crate) fn hashcode(&self) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut h);
        h.finish()
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.font_size_points.to_bits() == other.font_size_points.to_bits()
            && self.style_offsets == other.style_offsets
            && self.descriptors == other.descriptors
            && self.codepoint_map == other.codepoint_map
            && self.metric_modifiers == other.metric_modifiers
    }
}

impl Eq for Key {}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.font_size_points.to_bits().hash(state);
        self.style_offsets.hash(state);
        self.descriptors.len().hash(state);
        for descriptor in &self.descriptors {
            descriptor.hashcode().hash(state);
        }
        self.codepoint_map.hashcode().hash(state);
        self.metric_modifiers.len().hash(state);
        for key in MetricKey::ALL {
            if let Some(modifier) = self.metric_modifiers.get(&key) {
                key.hash(state);
                modifier.hash(state);
            }
        }
    }
}

fn append_descriptors(
    descriptors: &mut Vec<Descriptor>,
    families: &config::RepeatableString,
    style_config: &config::FontStyle,
    style: Style,
    font_size_points: f32,
    variations: &config::RepeatableFontVariation,
) {
    let exact_style = style_config.name_value();
    let variations = discovery_variations(variations);
    for family in &families.list {
        let mut descriptor = Descriptor {
            family: Some(family.clone()),
            style: exact_style.map(ToOwned::to_owned),
            size: font_size_points,
            monospace: true,
            variations: variations.clone(),
            ..Default::default()
        };
        if exact_style.is_none() {
            match style {
                Style::Regular => {}
                Style::Bold => descriptor.bold = true,
                Style::Italic => descriptor.italic = true,
                Style::BoldItalic => {
                    descriptor.bold = true;
                    descriptor.italic = true;
                }
            }
        }
        descriptors.push(descriptor);
    }
}

fn discovery_variations(variations: &config::RepeatableFontVariation) -> Vec<Variation> {
    variations
        .list
        .iter()
        .map(|v| Variation {
            id: Variation::id_from_tag(&v.id),
            value: v.value,
        })
        .collect()
}

fn metric_modifiers_from_config(config: &DerivedConfig) -> ModifierSet {
    let mut set = ModifierSet::new();
    let pairs = [
        (MetricKey::CellWidth, config.adjust_cell_width),
        (MetricKey::CellHeight, config.adjust_cell_height),
        (MetricKey::CellBaseline, config.adjust_font_baseline),
        (
            MetricKey::UnderlinePosition,
            config.adjust_underline_position,
        ),
        (
            MetricKey::UnderlineThickness,
            config.adjust_underline_thickness,
        ),
        (
            MetricKey::StrikethroughPosition,
            config.adjust_strikethrough_position,
        ),
        (
            MetricKey::StrikethroughThickness,
            config.adjust_strikethrough_thickness,
        ),
        (MetricKey::OverlinePosition, config.adjust_overline_position),
        (
            MetricKey::OverlineThickness,
            config.adjust_overline_thickness,
        ),
        (MetricKey::CursorThickness, config.adjust_cursor_thickness),
        (MetricKey::CursorHeight, config.adjust_cursor_height),
        (MetricKey::BoxThickness, config.adjust_box_thickness),
        (MetricKey::IconHeight, config.adjust_icon_height),
    ];
    for (key, modifier) in pairs {
        if let Some(modifier) = modifier {
            set.insert(key, modifier);
        }
    }
    set
}

/// Errors building a shared grid from config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BuildGridError {
    MissingPrimaryFont,
    CompleteStyles,
    InvalidPointSize,
}

impl From<CompleteError> for BuildGridError {
    fn from(_: CompleteError) -> Self {
        BuildGridError::CompleteStyles
    }
}

impl From<SetPointSizeError> for BuildGridError {
    fn from(error: SetPointSizeError) -> Self {
        match error {
            SetPointSizeError::InvalidPointSize => BuildGridError::InvalidPointSize,
            SetPointSizeError::CannotLoadPrimaryFont => BuildGridError::MissingPrimaryFont,
        }
    }
}

/// Build a `SharedGrid` from represented config fields. This is the config
/// assembly half of upstream `SharedGridSet.ref`.
pub(crate) fn build_grid_from_config(
    config: &config::Config,
    font_size_points: f32,
) -> Result<SharedGrid, BuildGridError> {
    let mut config = config.clone();
    config.finalize();
    let derived = DerivedConfig::from_config(&config);
    let key = Key::new(&derived, font_size_points);
    build_grid_for_key(&key, &derived)
}

fn build_grid_for_key(key: &Key, config: &DerivedConfig) -> Result<SharedGrid, BuildGridError> {
    let collection = collection_for_key(key, config)?;
    let metrics = *collection
        .metrics()
        .ok_or(BuildGridError::MissingPrimaryFont)?;

    let mut resolver = CodepointResolver::new(collection);
    resolver.set_style_enabled(Style::Bold, config.font_style_bold.enabled());
    resolver.set_style_enabled(Style::Italic, config.font_style_italic.enabled());
    resolver.set_style_enabled(Style::BoldItalic, config.font_style_bold_italic.enabled());
    resolver.set_discover_enabled(true);
    if !key.codepoint_map().is_empty() {
        resolver.set_codepoint_map(Some(key.codepoint_map().clone()));
    }

    Ok(SharedGrid::new(resolver, metrics))
}

fn collection_for_key(key: &Key, config: &DerivedConfig) -> Result<Collection, BuildGridError> {
    let mut collection = Collection::new();
    collection.set_metric_modifiers(key.metric_modifiers().clone());

    for style in [
        Style::Regular,
        Style::Bold,
        Style::Italic,
        Style::BoldItalic,
    ] {
        for descriptor in key.descriptors_for_style(style) {
            let mut face = descriptor.discover_deferred_faces().next();
            if face.is_none() && style != Style::Regular && !descriptor.variations.is_empty() {
                let mut retry = descriptor.clone();
                retry.bold = false;
                retry.italic = false;
                face = retry.discover_deferred_faces().next();
            }

            if let Some(face) = face {
                collection
                    .add_deferred_with_adjustment(
                        face,
                        style,
                        false,
                        crate::font::collection::SizeAdjustment::None,
                    )
                    .map_err(|_| BuildGridError::MissingPrimaryFont)?;
            }
        }
    }

    if collection.face_count(Style::Regular) == 0 {
        collection
            .add(
                Face::new("Menlo", key.font_size_points.max(1.0) as f64),
                Style::Regular,
                false,
            )
            .map_err(|_| BuildGridError::MissingPrimaryFont)?;
    }

    collection.set_point_size(key.font_size_points.max(1.0) as f64)?;
    collection.complete_styles(SyntheticStyle::from(config.font_synthetic_style))?;
    add_apple_emoji_fallback(&mut collection)?;
    collection
        .update_metrics()
        .map_err(|_| BuildGridError::MissingPrimaryFont)?;
    Ok(collection)
}

fn add_apple_emoji_fallback(collection: &mut Collection) -> Result<(), BuildGridError> {
    let descriptor = Descriptor {
        family: Some("Apple Color Emoji".to_string()),
        ..Default::default()
    };
    if let Some(face) = descriptor.discover_deferred_faces().next() {
        collection
            .add_deferred_with_adjustment(
                face,
                Style::Regular,
                true,
                crate::font::collection::SizeAdjustment::None,
            )
            .map_err(|_| BuildGridError::MissingPrimaryFont)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::config::{Config, FontStyle, FontSyntheticStyle};
    use crate::font::codepoint_resolver::CodepointResolver;
    use crate::font::collection::{Collection, Index};
    use crate::font::face::coretext::Face;
    use crate::font::Style;

    fn menlo_grid() -> SharedGrid {
        let mut collection = Collection::new();
        collection
            .add(Face::new("Menlo", 32.0), Style::Regular, false)
            .unwrap();
        collection.update_metrics().unwrap();
        let metrics = *collection.metrics().unwrap();
        SharedGrid::new(CodepointResolver::new(collection), metrics)
    }

    fn variation(tag: &[u8; 4], value: f64) -> Variation {
        Variation {
            id: Variation::id_from_tag(tag),
            value,
        }
    }

    #[test]
    fn ref_grid_reuses_cached_grid_and_counts_refs() {
        let set = SharedGridSet::new();

        let first = set.ref_grid("menlo", menlo_grid);
        assert_eq!(set.count(), 1);
        assert_eq!(set.ref_count(first.key()), Some(1));

        let second = set.ref_grid("menlo", menlo_grid);
        assert_eq!(set.count(), 1);
        assert_eq!(set.ref_count(first.key()), Some(2));
        assert!(Arc::ptr_eq(first.grid(), second.grid()));

        assert_eq!(set.deref_grid(second.key()), DerefResult::Decremented);
        assert_eq!(set.count(), 1);
        assert_eq!(set.ref_count(first.key()), Some(1));

        assert_eq!(set.deref_grid(first.key()), DerefResult::Removed);
        assert_eq!(set.count(), 0);
        assert_eq!(set.ref_count(first.key()), None);
    }

    #[test]
    fn different_keys_create_distinct_grids() {
        let set = SharedGridSet::new();

        let regular = set.ref_grid(("Menlo", 32), menlo_grid);
        let large = set.ref_grid(("Menlo", 40), menlo_grid);

        assert_eq!(set.count(), 2);
        assert_eq!(set.ref_count(regular.key()), Some(1));
        assert_eq!(set.ref_count(large.key()), Some(1));
        assert!(!Arc::ptr_eq(regular.grid(), large.grid()));
    }

    #[test]
    fn deref_missing_key_is_noop() {
        let set: SharedGridSet<&str> = SharedGridSet::new();

        assert_eq!(set.deref_grid(&"missing"), DerefResult::Missing);
        assert_eq!(set.count(), 0);
    }

    #[test]
    fn shared_handle_allows_mutable_grid_access() {
        let set = SharedGridSet::new();
        let handle = set.ref_grid("menlo", menlo_grid);

        let mut grid = handle.grid().lock().unwrap();
        let index = grid
            .get_index('A' as u32, Style::Regular, None)
            .unwrap()
            .expect("Menlo resolves A");

        assert!(grid.has_codepoint(index, 'A' as u32, None));
    }

    #[test]
    fn shared_grid_set_key_default_is_stable_and_size_sensitive() {
        let cfg = Config::default();
        let derived = DerivedConfig::from_config(&cfg);
        let key = Key::new(&derived, 13.0);
        let same = Key::new(&derived, 13.0);
        let larger = Key::new(&derived, 14.0);

        assert_eq!(key, same);
        assert_eq!(key.hashcode(), same.hashcode());
        assert_ne!(key, larger);
        assert_ne!(key.hashcode(), larger.hashcode());
        assert!(key.descriptors_for_style(Style::Regular).is_empty());
    }

    #[test]
    fn shared_grid_set_key_builds_style_ordered_descriptors() {
        let mut cfg = Config::default();
        cfg.font_family.parse_cli(Some("Regular")).unwrap();
        cfg.font_family_bold.parse_cli(Some("Bold")).unwrap();
        cfg.font_family_italic.parse_cli(Some("Italic")).unwrap();
        cfg.font_family_bold_italic
            .parse_cli(Some("Bold Italic"))
            .unwrap();
        cfg.font_style_bold = FontStyle::Name("Heavy".to_string());
        cfg.finalize();

        let derived = DerivedConfig::from_config(&cfg);
        let key = Key::new(&derived, 13.0);
        let regular = key.descriptors_for_style(Style::Regular);
        let bold = key.descriptors_for_style(Style::Bold);
        let italic = key.descriptors_for_style(Style::Italic);
        let bold_italic = key.descriptors_for_style(Style::BoldItalic);

        assert_eq!(regular.len(), 1);
        assert_eq!(regular[0].family.as_deref(), Some("Regular"));
        assert_eq!(regular[0].style, None);
        assert!(!regular[0].bold);
        assert!(!regular[0].italic);

        assert_eq!(bold.len(), 1);
        assert_eq!(bold[0].family.as_deref(), Some("Bold"));
        assert_eq!(bold[0].style.as_deref(), Some("Heavy"));
        assert!(!bold[0].bold, "exact style disables bold category search");

        assert_eq!(italic.len(), 1);
        assert_eq!(italic[0].family.as_deref(), Some("Italic"));
        assert!(italic[0].italic);

        assert_eq!(bold_italic.len(), 1);
        assert_eq!(bold_italic[0].family.as_deref(), Some("Bold Italic"));
        assert!(bold_italic[0].bold);
        assert!(bold_italic[0].italic);
    }

    #[test]
    fn font_variation_runtime_key_maps_each_style_variations() {
        let mut cfg = Config::default();
        cfg.font_family.parse_cli(Some("Regular")).unwrap();
        cfg.font_family_bold.parse_cli(Some("Bold")).unwrap();
        cfg.font_family_italic.parse_cli(Some("Italic")).unwrap();
        cfg.font_family_bold_italic
            .parse_cli(Some("Bold Italic"))
            .unwrap();
        cfg.set("font-variation", Some("wght=200")).unwrap();
        cfg.set("font-variation-bold", Some("wght=700")).unwrap();
        cfg.set("font-variation-italic", Some("slnt=-10")).unwrap();
        cfg.set("font-variation-bold-italic", Some("wdth=95"))
            .unwrap();
        cfg.finalize();

        let derived = DerivedConfig::from_config(&cfg);
        let key = Key::new(&derived, 13.0);

        assert_eq!(
            key.descriptors_for_style(Style::Regular)[0].variations,
            vec![variation(b"wght", 200.0)]
        );
        assert_eq!(
            key.descriptors_for_style(Style::Bold)[0].variations,
            vec![variation(b"wght", 700.0)]
        );
        assert_eq!(
            key.descriptors_for_style(Style::Italic)[0].variations,
            vec![variation(b"slnt", -10.0)]
        );
        assert_eq!(
            key.descriptors_for_style(Style::BoldItalic)[0].variations,
            vec![variation(b"wdth", 95.0)]
        );
    }

    #[test]
    fn font_variation_runtime_key_hash_changes_with_variation_value() {
        let mut base = Config::default();
        base.font_family.parse_cli(Some("Menlo")).unwrap();
        base.set("font-variation", Some("wght=200")).unwrap();

        let mut changed = base.clone();
        changed.set("font-variation", Some("")).unwrap();
        changed.set("font-variation", Some("wght=700")).unwrap();

        let base_key = Key::new(&DerivedConfig::from_config(&base), 13.0);
        let changed_key = Key::new(&DerivedConfig::from_config(&changed), 13.0);

        assert_ne!(base_key, changed_key);
        assert_ne!(base_key.hashcode(), changed_key.hashcode());
    }

    #[test]
    fn font_variation_runtime_key_preserves_style_offsets() {
        let mut cfg = Config::default();
        cfg.font_family.parse_cli(Some("Regular One")).unwrap();
        cfg.font_family.parse_cli(Some("Regular Two")).unwrap();
        cfg.font_family_bold.parse_cli(Some("Bold One")).unwrap();
        cfg.font_family_italic
            .parse_cli(Some("Italic One"))
            .unwrap();
        cfg.font_family_bold_italic
            .parse_cli(Some("Bold Italic One"))
            .unwrap();
        cfg.set("font-variation", Some("wght=200")).unwrap();
        cfg.set("font-variation-bold", Some("wght=700")).unwrap();
        cfg.set("font-variation-italic", Some("slnt=-10")).unwrap();
        cfg.set("font-variation-bold-italic", Some("wdth=95"))
            .unwrap();

        let key = Key::new(&DerivedConfig::from_config(&cfg), 13.0);

        assert_eq!(key.descriptors_for_style(Style::Regular).len(), 2);
        assert_eq!(key.descriptors_for_style(Style::Bold).len(), 1);
        assert_eq!(key.descriptors_for_style(Style::Italic).len(), 1);
        assert_eq!(key.descriptors_for_style(Style::BoldItalic).len(), 1);
        assert_eq!(
            key.descriptors_for_style(Style::Regular)[1].variations,
            vec![variation(b"wght", 200.0)]
        );
        assert_eq!(
            key.descriptors_for_style(Style::BoldItalic)[0]
                .family
                .as_deref(),
            Some("Bold Italic One")
        );
    }

    #[test]
    fn font_variation_runtime_default_key_has_no_variations() {
        let mut cfg = Config::default();
        cfg.font_family.parse_cli(Some("Menlo")).unwrap();

        let key = Key::new(&DerivedConfig::from_config(&cfg), 13.0);

        assert!(
            key.descriptors_for_style(Style::Regular)[0]
                .variations
                .is_empty(),
            "no-variation config keeps descriptor variations empty"
        );
    }

    #[test]
    fn font_variation_runtime_build_grid_with_configured_variations() {
        let mut cfg = Config::default();
        cfg.font_family.parse_cli(Some("Menlo")).unwrap();
        cfg.set("font-variation", Some("wght=700")).unwrap();

        let mut grid = build_grid_from_config(&cfg, 13.0).expect("grid builds");
        let index = grid
            .get_index('A' as u32, Style::Regular, None)
            .unwrap()
            .expect("configured variation grid resolves ASCII");

        assert!(grid.has_codepoint(index, 'A' as u32, None));
    }

    #[test]
    fn font_metric_modifier_runtime_key_maps_all_adjust_fields() {
        let mut cfg = Config::default();
        cfg.set("adjust-cell-width", Some("1")).unwrap();
        cfg.set("adjust-cell-height", Some("2")).unwrap();
        cfg.set("adjust-font-baseline", Some("3")).unwrap();
        cfg.set("adjust-underline-position", Some("4")).unwrap();
        cfg.set("adjust-underline-thickness", Some("5")).unwrap();
        cfg.set("adjust-strikethrough-position", Some("6")).unwrap();
        cfg.set("adjust-strikethrough-thickness", Some("7"))
            .unwrap();
        cfg.set("adjust-overline-position", Some("8")).unwrap();
        cfg.set("adjust-overline-thickness", Some("9")).unwrap();
        cfg.set("adjust-cursor-thickness", Some("10")).unwrap();
        cfg.set("adjust-cursor-height", Some("11")).unwrap();
        cfg.set("adjust-box-thickness", Some("12")).unwrap();
        cfg.set("adjust-icon-height", Some("25%")).unwrap();

        let key = Key::new(&DerivedConfig::from_config(&cfg), 13.0);
        let modifiers = key.metric_modifiers();

        assert_eq!(modifiers.len(), 13);
        assert_eq!(
            modifiers.get(&MetricKey::CellWidth),
            Some(&Modifier::Absolute(1))
        );
        assert_eq!(
            modifiers.get(&MetricKey::CellHeight),
            Some(&Modifier::Absolute(2))
        );
        assert_eq!(
            modifiers.get(&MetricKey::CellBaseline),
            Some(&Modifier::Absolute(3))
        );
        assert_eq!(
            modifiers.get(&MetricKey::UnderlinePosition),
            Some(&Modifier::Absolute(4))
        );
        assert_eq!(
            modifiers.get(&MetricKey::UnderlineThickness),
            Some(&Modifier::Absolute(5))
        );
        assert_eq!(
            modifiers.get(&MetricKey::StrikethroughPosition),
            Some(&Modifier::Absolute(6))
        );
        assert_eq!(
            modifiers.get(&MetricKey::StrikethroughThickness),
            Some(&Modifier::Absolute(7))
        );
        assert_eq!(
            modifiers.get(&MetricKey::OverlinePosition),
            Some(&Modifier::Absolute(8))
        );
        assert_eq!(
            modifiers.get(&MetricKey::OverlineThickness),
            Some(&Modifier::Absolute(9))
        );
        assert_eq!(
            modifiers.get(&MetricKey::CursorThickness),
            Some(&Modifier::Absolute(10))
        );
        assert_eq!(
            modifiers.get(&MetricKey::CursorHeight),
            Some(&Modifier::Absolute(11))
        );
        assert_eq!(
            modifiers.get(&MetricKey::BoxThickness),
            Some(&Modifier::Absolute(12))
        );
        assert_eq!(
            modifiers.get(&MetricKey::IconHeight),
            Some(&Modifier::Percent(1.25))
        );
    }

    #[test]
    fn font_metric_modifier_runtime_key_hash_changes_with_modifiers() {
        let mut base = Config::default();
        base.font_family.parse_cli(Some("Menlo")).unwrap();

        let mut adjusted = base.clone();
        adjusted.set("adjust-cell-width", Some("2")).unwrap();

        let base_key = Key::new(&DerivedConfig::from_config(&base), 13.0);
        let adjusted_key = Key::new(&DerivedConfig::from_config(&adjusted), 13.0);

        assert_ne!(base_key, adjusted_key);
        assert_ne!(base_key.hashcode(), adjusted_key.hashcode());
    }

    #[test]
    fn font_metric_modifier_runtime_build_grid_applies_config_modifiers() {
        let mut cfg = Config::default();
        cfg.font_family.parse_cli(Some("Menlo")).unwrap();
        let default_grid = build_grid_from_config(&cfg, 13.0).expect("default grid");

        cfg.set("adjust-cell-width", Some("3")).unwrap();
        cfg.set("adjust-cursor-thickness", Some("2")).unwrap();
        let adjusted_grid = build_grid_from_config(&cfg, 13.0).expect("adjusted grid");

        assert_eq!(
            adjusted_grid.metrics.cell_width,
            default_grid.metrics.cell_width + 3
        );
        assert_eq!(
            adjusted_grid.metrics.cursor_thickness,
            default_grid.metrics.cursor_thickness + 2
        );
    }

    #[test]
    fn font_metric_modifier_runtime_build_grid_recenters_cell_height() {
        let mut cfg = Config::default();
        cfg.font_family.parse_cli(Some("Menlo")).unwrap();
        let default_grid = build_grid_from_config(&cfg, 13.0).expect("default grid");

        cfg.set("adjust-cell-height", Some("5")).unwrap();
        let adjusted_grid = build_grid_from_config(&cfg, 13.0).expect("adjusted grid");

        assert_eq!(
            adjusted_grid.metrics.cell_height,
            default_grid.metrics.cell_height + 5
        );
        assert_ne!(
            adjusted_grid.metrics.cell_baseline,
            default_grid.metrics.cell_baseline
        );
        assert_ne!(
            adjusted_grid.metrics.underline_position,
            default_grid.metrics.underline_position
        );
    }

    #[test]
    fn shared_grid_set_key_preserves_multiple_family_order() {
        let mut cfg = Config::default();
        cfg.font_family.parse_cli(Some("Regular First")).unwrap();
        cfg.font_family.parse_cli(Some("Regular Second")).unwrap();
        cfg.font_family_bold.parse_cli(Some("Bold First")).unwrap();
        cfg.font_family_bold.parse_cli(Some("Bold Second")).unwrap();
        cfg.font_style = FontStyle::Name("Book".to_string());
        cfg.finalize();

        let derived = DerivedConfig::from_config(&cfg);
        let key = Key::new(&derived, 13.0);
        let regular = key.descriptors_for_style(Style::Regular);
        let bold = key.descriptors_for_style(Style::Bold);

        assert_eq!(regular.len(), 2);
        assert_eq!(regular[0].family.as_deref(), Some("Regular First"));
        assert_eq!(regular[1].family.as_deref(), Some("Regular Second"));
        assert_eq!(regular[0].style.as_deref(), Some("Book"));
        assert_eq!(regular[1].style.as_deref(), Some("Book"));
        assert!(!regular[0].bold);
        assert!(!regular[1].bold);

        assert_eq!(bold.len(), 2);
        assert_eq!(bold[0].family.as_deref(), Some("Bold First"));
        assert_eq!(bold[1].family.as_deref(), Some("Bold Second"));
        assert!(bold[0].bold);
        assert!(bold[1].bold);
    }

    #[test]
    fn shared_grid_set_key_includes_codepoint_map() {
        let mut cfg = Config::default();
        let base = Key::new(&DerivedConfig::from_config(&cfg), 13.0);
        cfg.font_codepoint_map
            .parse_cli(Some("U+0041-U+005A=Helvetica"))
            .unwrap();
        let mapped = Key::new(&DerivedConfig::from_config(&cfg), 13.0);

        assert_ne!(base, mapped);
        assert_ne!(base.hashcode(), mapped.hashcode());
        assert_eq!(mapped.codepoint_map().len(), 1);
    }

    #[test]
    fn shared_grid_set_build_grid_from_default_config() {
        let cfg = Config::default();
        let mut grid = build_grid_from_config(&cfg, 13.0).expect("grid builds");
        let index = grid
            .get_index('A' as u32, Style::Regular, None)
            .unwrap()
            .expect("default config resolves ASCII");
        assert!(grid.has_codepoint(index, 'A' as u32, None));
    }

    #[test]
    fn shared_grid_set_build_grid_honors_codepoint_override() {
        let mut cfg = Config::default();
        cfg.font_codepoint_map
            .parse_cli(Some("U+0041=Helvetica"))
            .unwrap();

        let mut grid = build_grid_from_config(&cfg, 13.0).expect("grid builds");
        let overridden = grid
            .get_index('A' as u32, Style::Regular, None)
            .unwrap()
            .expect("override resolves A");
        let regular = build_grid_from_config(&Config::default(), 13.0)
            .expect("plain grid builds")
            .get_index('A' as u32, Style::Regular, None)
            .unwrap()
            .expect("plain grid resolves A");

        assert_ne!(
            overridden.int(),
            regular.int(),
            "font-codepoint-map should change the resolved face"
        );
    }

    #[test]
    fn shared_grid_set_build_grid_honors_disabled_synthetic_styles() {
        let mut cfg = Config::default();
        cfg.font_synthetic_style = FontSyntheticStyle {
            bold: false,
            italic: false,
            bold_italic: false,
        };

        let mut grid = build_grid_from_config(&cfg, 13.0).expect("grid builds");
        let collection = grid.resolver.collection_mut();
        let bold_is_synthetic = collection
            .get_face(Index::new(Style::Bold, 0))
            .unwrap()
            .synthetic_bold_width()
            .is_some();
        let italic_is_synthetic = collection
            .get_face(Index::new(Style::Italic, 0))
            .unwrap()
            .is_skewed();
        let bold_italic = collection
            .get_face(Index::new(Style::BoldItalic, 0))
            .unwrap();
        let bold_italic_is_bold = bold_italic.synthetic_bold_width().is_some();
        let bold_italic_is_italic = bold_italic.is_skewed();

        assert!(!bold_is_synthetic);
        assert!(!italic_is_synthetic);
        assert!(!bold_italic_is_bold);
        assert!(!bold_italic_is_italic);
    }
}
