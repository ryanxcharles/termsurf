use super::formatter::EntryFormatter;
use super::ConfigSetError;
use crate::input::key;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeybindsParseError {
    ValueRequired,
    InvalidValue,
}

impl From<KeybindsParseError> for ConfigSetError {
    fn from(error: KeybindsParseError) -> Self {
        match error {
            KeybindsParseError::ValueRequired => ConfigSetError::ValueRequired,
            KeybindsParseError::InvalidValue => ConfigSetError::InvalidValue,
        }
    }
}

/// Config-owned keybinding set (upstream `Config.Keybinds`).
#[derive(Clone)]
pub(crate) struct Keybinds {
    pub(crate) has_global_keybinds: bool,
    pub(crate) triggers: Vec<crate::ConfigKeybind>,
    pub(crate) sequences: crate::ConfigKeybindSet,
    pub(crate) tables: Vec<crate::ConfigKeybindTable>,
    pub(crate) chain_parent: Option<crate::ConfigKeybindChainParent>,
}

impl fmt::Debug for Keybinds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Keybinds")
            .field("has_global_keybinds", &self.has_global_keybinds)
            .field("triggers_len", &self.triggers.len())
            .field("tables_len", &self.tables.len())
            .finish()
    }
}

impl PartialEq for Keybinds {
    fn eq(&self, other: &Self) -> bool {
        self.has_global_keybinds == other.has_global_keybinds
            && keybinds_eq(&self.triggers, &other.triggers)
            && keybind_sets_eq(&self.sequences, &other.sequences)
            && keybind_tables_eq(&self.tables, &other.tables)
    }
}

impl Default for Keybinds {
    fn default() -> Self {
        let mut keybinds = Self::empty();
        keybinds.reset_to_defaults();
        keybinds
    }
}

impl Keybinds {
    fn empty() -> Self {
        Self {
            has_global_keybinds: false,
            triggers: Vec::new(),
            sequences: crate::ConfigKeybindSet::default(),
            tables: Vec::new(),
            chain_parent: None,
        }
    }

    fn reset_to_defaults(&mut self) {
        self.clear();
        for entry in crate::DEFAULT_BINDINGS {
            self.store_keybind(crate::ConfigKeybind {
                actions: vec![entry.action.to_vec()],
                flags: entry.flags,
                trigger: crate::default_binding_entry_trigger(entry),
            });
        }
        self.chain_parent = None;
    }

    fn clear(&mut self) {
        self.has_global_keybinds = false;
        self.triggers.clear();
        self.sequences.clear();
        self.tables.clear();
        self.chain_parent = None;
    }

    pub(crate) fn parse_cli(&mut self, input: Option<&str>) -> Result<(), KeybindsParseError> {
        let value = input.ok_or(KeybindsParseError::ValueRequired)?;
        if value.is_empty() {
            self.reset_to_defaults();
            return Ok(());
        }
        if value == "clear" {
            self.clear();
            return Ok(());
        }

        let entry = crate::parse_config_keybind_entry(value.as_bytes())
            .map_err(|_| KeybindsParseError::InvalidValue)?;
        match self.store_keybind_entry(entry) {
            crate::ConfigKeybindStoreResult::Ok => Ok(()),
            crate::ConfigKeybindStoreResult::Err(_) => Err(KeybindsParseError::InvalidValue),
        }
    }

    pub(crate) fn format_entry(&self, formatter: &mut EntryFormatter) {
        if self.triggers.is_empty() && self.tables.is_empty() {
            formatter.entry_void();
            return;
        }

        for entry in format_set(&self.sequences, None) {
            formatter.entry_str(&entry);
        }
        for table in &self.tables {
            let table_name = String::from_utf8_lossy(&table.name);
            if table.bindings.is_empty()
                && table.sequences.bindings.is_empty()
                && table.sequences.leaders.is_empty()
            {
                continue;
            }
            for entry in format_set(&table.sequences, Some(&table_name)) {
                formatter.entry_str(&entry);
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn format_entry_count(&self) -> usize {
        if self.triggers.is_empty() && self.tables.is_empty() {
            return 1;
        }
        let root = format_set(&self.sequences, None).len();
        root + self
            .tables
            .iter()
            .map(|table| {
                if table.bindings.is_empty()
                    && table.sequences.bindings.is_empty()
                    && table.sequences.leaders.is_empty()
                {
                    0
                } else {
                    format_set(
                        &table.sequences,
                        Some(&String::from_utf8_lossy(&table.name)),
                    )
                    .len()
                }
            })
            .sum::<usize>()
    }

    pub(crate) fn store_keybind_entry(
        &mut self,
        entry: crate::ParsedConfigKeybind,
    ) -> crate::ConfigKeybindStoreResult {
        match entry {
            crate::ParsedConfigKeybind::Root(binding) => match binding {
                crate::ParsedConfigKeybindBinding::Direct(binding) => self.store_keybind(binding),
                crate::ParsedConfigKeybindBinding::Sequence(sequence) => {
                    self.store_keybind_sequence(sequence);
                }
            },
            crate::ParsedConfigKeybind::Chain(action) => {
                if !self.append_keybind_chain(action) {
                    self.chain_parent = None;
                    return crate::ConfigKeybindStoreResult::Err(
                        crate::ConfigKeybindParseError::InvalidFormat,
                    );
                }
            }
            crate::ParsedConfigKeybind::Table { name, binding } => {
                let parent = match &binding {
                    crate::ParsedConfigKeybindBinding::Direct(binding) => {
                        crate::ConfigKeybindChainParent::TableDirect {
                            name: name.clone(),
                            trigger: binding.trigger,
                        }
                    }
                    crate::ParsedConfigKeybindBinding::Sequence(sequence) => {
                        crate::ConfigKeybindChainParent::TableSequence {
                            name: name.clone(),
                            triggers: sequence.triggers.clone(),
                        }
                    }
                };
                if let Some(table) = self.tables.iter_mut().find(|table| table.name == name) {
                    table.store(binding);
                } else {
                    let mut table = crate::ConfigKeybindTable {
                        name,
                        bindings: Vec::new(),
                        sequences: crate::ConfigKeybindSet::default(),
                    };
                    table.store(binding);
                    self.tables.push(table);
                }
                self.chain_parent = Some(parent);
            }
            crate::ParsedConfigKeybind::TableClear { name } => {
                if let Some(table) = self.tables.iter_mut().find(|table| table.name == name) {
                    table.bindings.clear();
                    table.sequences.clear();
                } else {
                    self.tables.push(crate::ConfigKeybindTable {
                        name,
                        bindings: Vec::new(),
                        sequences: crate::ConfigKeybindSet::default(),
                    });
                }
                self.chain_parent = None;
            }
        }
        self.refresh_has_global_keybinds();
        crate::ConfigKeybindStoreResult::Ok
    }

    fn store_keybind(&mut self, binding: crate::ConfigKeybind) {
        if binding.flags & crate::ROASTTY_KEYBIND_FLAG_GLOBAL != 0 {
            self.has_global_keybinds = true;
        }
        let trigger = binding.trigger;
        self.sequences.store_direct(binding.clone());
        self.triggers
            .retain(|existing| !crate::config_trigger_matches_candidate(existing.trigger, trigger));
        self.triggers.push(binding);
        self.chain_parent = Some(crate::ConfigKeybindChainParent::RootDirect(trigger));
    }

    fn store_keybind_sequence(&mut self, sequence: crate::ConfigKeybindSequence) {
        if sequence.binding.flags & crate::ROASTTY_KEYBIND_FLAG_GLOBAL != 0 {
            self.has_global_keybinds = true;
        }
        if let Some(first) = sequence.triggers.first().copied() {
            self.triggers
                .retain(|binding| !crate::config_trigger_matches_candidate(binding.trigger, first));
        }
        let triggers = sequence.triggers.clone();
        self.sequences.store_sequence(sequence);
        self.chain_parent = Some(crate::ConfigKeybindChainParent::RootSequence(triggers));
    }

    fn append_keybind_chain(&mut self, action: Vec<u8>) -> bool {
        let Some(parent) = self.chain_parent.clone() else {
            return false;
        };

        match parent {
            crate::ConfigKeybindChainParent::RootDirect(trigger) => {
                let mut appended = false;
                if let Some(binding) = self.triggers.iter_mut().rev().find(|binding| {
                    crate::config_trigger_matches_candidate(binding.trigger, trigger)
                }) {
                    binding.append_action(action.clone());
                    appended = true;
                }
                if self.sequences.append_chain(&[trigger], action) {
                    appended = true;
                }
                appended
            }
            crate::ConfigKeybindChainParent::RootSequence(triggers) => {
                self.sequences.append_chain(&triggers, action)
            }
            crate::ConfigKeybindChainParent::TableDirect { name, trigger } => {
                let Some(table) = self.tables.iter_mut().find(|table| table.name == name) else {
                    return false;
                };
                let mut appended = false;
                if let Some(binding) = table.bindings.iter_mut().rev().find(|binding| {
                    crate::config_trigger_matches_candidate(binding.trigger, trigger)
                }) {
                    binding.append_action(action.clone());
                    appended = true;
                }
                if table.sequences.append_chain(&[trigger], action) {
                    appended = true;
                }
                appended
            }
            crate::ConfigKeybindChainParent::TableSequence { name, triggers } => {
                let Some(table) = self.tables.iter_mut().find(|table| table.name == name) else {
                    return false;
                };
                table.sequences.append_chain(&triggers, action)
            }
        }
    }

    fn refresh_has_global_keybinds(&mut self) {
        self.has_global_keybinds = self.sequences.has_global_keybinds()
            || self.tables.iter().any(|table| {
                table.sequences.has_global_keybinds()
                    || table
                        .bindings
                        .iter()
                        .any(|binding| binding.flags & crate::ROASTTY_KEYBIND_FLAG_GLOBAL != 0)
            });
    }
}

fn keybinds_eq(left: &[crate::ConfigKeybind], right: &[crate::ConfigKeybind]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.flags == right.flags
                && left.actions == right.actions
                && crate::config_trigger_matches_candidate(left.trigger, right.trigger)
        })
}

fn keybind_sets_eq(left: &crate::ConfigKeybindSet, right: &crate::ConfigKeybindSet) -> bool {
    keybinds_eq(&left.bindings, &right.bindings)
        && left.leaders.len() == right.leaders.len()
        && left
            .leaders
            .iter()
            .zip(&right.leaders)
            .all(|(left, right)| {
                crate::config_trigger_matches_candidate(left.trigger, right.trigger)
                    && keybind_sets_eq(&left.set, &right.set)
            })
}

fn keybind_tables_eq(
    left: &[crate::ConfigKeybindTable],
    right: &[crate::ConfigKeybindTable],
) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.name == right.name
                && keybinds_eq(&left.bindings, &right.bindings)
                && keybind_sets_eq(&left.sequences, &right.sequences)
        })
}

fn format_set(set: &crate::ConfigKeybindSet, table: Option<&str>) -> Vec<String> {
    let mut entries = Vec::new();
    for binding in &set.bindings {
        entries.extend(format_binding(binding, &[], table));
    }
    for leader in &set.leaders {
        let mut prefix = vec![leader.trigger];
        format_leader(&leader.set, &mut prefix, table, &mut entries);
    }
    entries
}

fn format_leader(
    set: &crate::ConfigKeybindSet,
    prefix: &mut Vec<crate::RoasttyInputTrigger>,
    table: Option<&str>,
    entries: &mut Vec<String>,
) {
    for binding in &set.bindings {
        entries.extend(format_binding(binding, prefix, table));
    }
    for leader in &set.leaders {
        prefix.push(leader.trigger);
        format_leader(&leader.set, prefix, table, entries);
        prefix.pop();
    }
}

fn format_binding(
    binding: &crate::ConfigKeybind,
    prefix: &[crate::RoasttyInputTrigger],
    table: Option<&str>,
) -> Vec<String> {
    let mut trigger = String::new();
    if let Some(table) = table {
        trigger.push_str(table);
        trigger.push('/');
    }

    let mut trigger_parts = prefix
        .iter()
        .copied()
        .chain(std::iter::once(binding.trigger))
        .map(format_trigger)
        .collect::<Vec<_>>();
    trigger.push_str(&trigger_parts.remove(0));
    for part in trigger_parts {
        trigger.push('>');
        trigger.push_str(&part);
    }

    let mut entries = Vec::new();
    for (index, action) in binding.actions.iter().enumerate() {
        let action = format_action(action);
        if index == 0 {
            entries.push(format!("{trigger}={action}"));
        } else {
            entries.push(format!("chain={action}"));
        }
    }
    entries
}

fn format_action(action: &[u8]) -> String {
    if matches!(action, b"previous_tab" | b"next_tab" | b"last_tab") {
        return String::from_utf8_lossy(action).into_owned();
    }
    crate::canonical_config_binding_action(action)
        .unwrap_or_else(|| String::from_utf8_lossy(action).into_owned())
}

fn format_trigger(trigger: crate::RoasttyInputTrigger) -> String {
    let mut parts = Vec::new();
    if trigger.mods & crate::ROASTTY_MODS_SUPER != 0 {
        parts.push("super".to_string());
    }
    if trigger.mods & crate::ROASTTY_MODS_CTRL != 0 {
        parts.push("ctrl".to_string());
    }
    if trigger.mods & crate::ROASTTY_MODS_ALT != 0 {
        parts.push("alt".to_string());
    }
    if trigger.mods & crate::ROASTTY_MODS_SHIFT != 0 {
        parts.push("shift".to_string());
    }
    parts.push(match trigger.tag {
        crate::ROASTTY_TRIGGER_PHYSICAL => {
            let raw = unsafe { trigger.key.physical };
            key::ALL_KEYS
                .iter()
                .copied()
                .find(|key| *key as i32 == raw)
                .map_or("unidentified".to_string(), format_physical_key)
        }
        crate::ROASTTY_TRIGGER_UNICODE => {
            let codepoint = unsafe { trigger.key.unicode };
            char::from_u32(codepoint)
                .map(|ch| ch.to_string())
                .unwrap_or_else(|| "Unidentified".to_string())
        }
        crate::ROASTTY_TRIGGER_CATCH_ALL => "catch_all".to_string(),
        _ => "Unidentified".to_string(),
    });
    parts.join("+")
}

fn format_physical_key(key: key::Key) -> String {
    key.snake().to_string()
}
