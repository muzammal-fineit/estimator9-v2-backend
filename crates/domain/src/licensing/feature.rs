use std::collections::BTreeSet;

/// A switchable capability.
///
/// Wraps `&'static str` for the same reason [`crate::audit::AuditAction`] does:
/// the catalogue is fixed at compile time, so a caller can only name a feature
/// that exists, and the whole list is readable in one screen. It also means the
/// catalogue genuinely ships *in the binary* rather than in data the client can
/// add rows to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeatureName(&'static str);

impl FeatureName {
    pub fn as_str(self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for FeatureName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// A feature and what enabling it means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Feature {
    pub name: FeatureName,
    pub description: &'static str,

    /// What a fresh installation gets before anything is toggled. Off for
    /// anything chargeable; on for capabilities every installation has.
    pub enabled_by_default: bool,
}

impl Feature {
    const fn new(name: &'static str, description: &'static str, default: bool) -> Self {
        Self {
            name: FeatureName(name),
            description,
            enabled_by_default: default,
        }
    }
}

/// The catalogue. Adding a capability means adding a line here and shipping a
/// release — which is the intent: a feature the code never checks would be a
/// label that grants nothing.
pub const CATALOGUE: &[Feature] = &[
    Feature::new(
        "mev.fitting",
        "Fit MEV regression models from macroeconomic series.",
        false,
    ),
    Feature::new(
        "provision.runs",
        "Calculate and approve IFRS 9 provision runs.",
        false,
    ),
    Feature::new(
        "audit.export",
        "Export the audit trail for a compliance request.",
        true,
    ),
];

pub fn lookup(name: &str) -> Option<Feature> {
    CATALOGUE.iter().copied().find(|f| f.name.as_str() == name)
}

/// Which features are on. Ordered so two equal sets serialise identically,
/// which keeps the encrypted file stable when nothing has changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeatureSet(BTreeSet<&'static str>);

impl FeatureSet {
    /// What a fresh installation starts with.
    pub fn defaults() -> Self {
        Self(
            CATALOGUE
                .iter()
                .filter(|f| f.enabled_by_default)
                .map(|f| f.name.as_str())
                .collect(),
        )
    }

    /// Rebuilds from stored names, **discarding any it does not recognise**.
    ///
    /// A name that is not in the catalogue cannot switch anything on, so a
    /// hand-written state file granting `everything` achieves nothing. It also
    /// means a downgrade quietly drops features the older binary lacks rather
    /// than failing to start.
    pub fn rehydrate<'a>(names: impl IntoIterator<Item = &'a str>) -> Self {
        Self(
            names
                .into_iter()
                .filter_map(|name| lookup(name).map(|f| f.name.as_str()))
                .collect(),
        )
    }

    pub fn is_enabled(&self, name: FeatureName) -> bool {
        self.0.contains(name.as_str())
    }

    pub fn set(&mut self, name: FeatureName, enabled: bool) {
        if enabled {
            self.0.insert(name.as_str());
        } else {
            self.0.remove(name.as_str());
        }
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.0.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogue_has_no_duplicate_names() {
        let mut names: Vec<&str> = CATALOGUE.iter().map(|f| f.name.as_str()).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();

        assert_eq!(names.len(), before, "two features share a name");
    }

    /// The property the whole design rests on: a stored name that is not in the
    /// binary's catalogue grants nothing, so an invented state file is inert.
    #[test]
    fn unknown_names_are_discarded_rather_than_honoured() {
        let set = FeatureSet::rehydrate(["mev.fitting", "everything", "admin.god_mode"]);

        assert_eq!(set.names().collect::<Vec<_>>(), ["mev.fitting"]);
    }

    #[test]
    fn defaults_are_the_free_capabilities_only() {
        let defaults = FeatureSet::defaults();

        assert!(defaults.is_enabled(FeatureName("audit.export")));
        assert!(!defaults.is_enabled(FeatureName("mev.fitting")));
    }

    #[test]
    fn toggling_is_idempotent() {
        let mut set = FeatureSet::defaults();
        let mev = FeatureName("mev.fitting");

        set.set(mev, true);
        set.set(mev, true);
        assert!(set.is_enabled(mev));

        set.set(mev, false);
        set.set(mev, false);
        assert!(!set.is_enabled(mev));
    }
}
