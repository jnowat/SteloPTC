// WP-74: Profile-pluggable compliance rule engine.
//
// Historically the compliance auto-flag rules in `commands/compliance.rs` were a
// flat list of hardcoded SQL blocks. Several of them encoded PTC/citrus
// assumptions — most notably the "Citrus missing HLB test" rule, which matches
// species codes `LIKE 'CIT-%'` — yet ran for EVERY active lab profile. So a
// mycology or cell-culture lab that happened to code a species `CIT-*` (e.g. a
// *Citrobacter* isolate, or any lab-local code starting with those letters) got
// a spurious plant-quarantine flag it could never clear. That is the long-open
// "compliance rule engine is PTC-only / not profile-gated" item in `skills.md`
// §8 and the ROADMAP's WP-25 deviation.
//
// This module makes the catalog of auto-flag rules first-class and profile
// scoped: each rule declares which profiles it applies to, and the command layer
// evaluates a rule only when it is active for the current profile. The mycology
// and cell-culture rule groups were already profile-gated at the call site; this
// centralizes that decision in one place, closes the citrus-HLB cross-profile
// leak, and gives the UI a single source of truth for "which rules are active
// here".
//
// Everything here is pure (no DB, no I/O), so it is unit-testable under
// `--no-default-features` and can never fail a command.

/// The three built-in lab profiles. Kept as `&str` constants (mirroring
/// `app_settings.lab_profile` values) rather than an enum so the module stays
/// decoupled from the frontend `LabProfile` type and forward-compatible with
/// plugin-supplied profiles.
pub const PLANT_TISSUE_CULTURE: &str = "plant_tissue_culture";
pub const CELL_CULTURE: &str = "cell_culture";
pub const MYCOLOGY: &str = "mycology";

/// Which lab profiles a rule applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleScope {
    /// Applies in every profile (general regulatory hygiene: permits, quarantine).
    AllProfiles,
    /// Applies only in the named profiles (domain-specific QC).
    Profiles(&'static [&'static str]),
}

/// A built-in compliance auto-flag rule.
#[derive(Debug, Clone, Copy)]
pub struct RuleDef {
    /// Stable identifier — also the `flag_type` emitted on each `ComplianceFlag`.
    pub flag_type: &'static str,
    /// Human-readable rule name, for the rule-catalog UI.
    pub title: &'static str,
    /// Default severity: `"critical"` | `"high"` | `"normal"`.
    pub severity: &'static str,
    /// Profiles this rule is active in.
    pub scope: RuleScope,
}

impl RuleDef {
    /// True if this rule should be evaluated for the given active profile.
    pub fn applies_to(&self, profile: &str) -> bool {
        match self.scope {
            RuleScope::AllProfiles => true,
            RuleScope::Profiles(ps) => ps.contains(&profile),
        }
    }
}

/// The built-in rule catalog. Order here is the order flag groups are evaluated
/// and grouped in the UI. Adding a rule = one entry here plus its SQL block in
/// `get_compliance_flags`, gated by `is_rule_active(flag_type, &profile)`.
pub const RULES: &[RuleDef] = &[
    // ── General regulatory hygiene — every profile ──────────────────────────
    RuleDef {
        flag_type: "expired_permit",
        title: "Expired regulatory permit",
        severity: "critical",
        scope: RuleScope::AllProfiles,
    },
    RuleDef {
        flag_type: "quarantine_no_release",
        title: "Quarantined without scheduled release",
        severity: "high",
        scope: RuleScope::AllProfiles,
    },
    RuleDef {
        flag_type: "positive_not_quarantined",
        title: "Positive disease test but not quarantined",
        severity: "critical",
        scope: RuleScope::AllProfiles,
    },
    // ── Plant Tissue Culture — citrus HLB (Huanglongbing / CLas) screening ───
    RuleDef {
        flag_type: "missing_hlb_test",
        title: "Citrus specimen missing HLB test",
        severity: "critical",
        scope: RuleScope::Profiles(&[PLANT_TISSUE_CULTURE]),
    },
    // ── Mycology QC (WP-44) ─────────────────────────────────────────────────
    RuleDef {
        flag_type: "myco_open_contamination",
        title: "Open contamination — culture not discarded",
        severity: "high",
        scope: RuleScope::Profiles(&[MYCOLOGY]),
    },
    RuleDef {
        flag_type: "myco_overdue_transfer",
        title: "Overdue for transfer",
        severity: "normal",
        scope: RuleScope::Profiles(&[MYCOLOGY]),
    },
    RuleDef {
        flag_type: "myco_slow_colonization",
        title: "Slow colonization",
        severity: "normal",
        scope: RuleScope::Profiles(&[MYCOLOGY]),
    },
    // ── Cell Culture QC (WP-33) ─────────────────────────────────────────────
    RuleDef {
        flag_type: "missing_mycoplasma_test",
        title: "Missing / overdue mycoplasma test",
        severity: "high",
        scope: RuleScope::Profiles(&[CELL_CULTURE]),
    },
    // ── Environmental monitoring (WP-78) — every profile ────────────────────
    RuleDef {
        flag_type: "environmental_out_of_range",
        title: "Latest environmental reading out of range",
        severity: "high",
        scope: RuleScope::AllProfiles,
    },
];

/// Look up a rule by its `flag_type`.
pub fn rule(flag_type: &str) -> Option<&'static RuleDef> {
    RULES.iter().find(|r| r.flag_type == flag_type)
}

/// True if the named rule is active for the given profile.
///
/// An unknown `flag_type` is treated as active (fail-open) so a future rule
/// wired at the call site before it is catalogued here still runs — but every
/// built-in rule is catalogued, so in practice this is exact.
pub fn is_rule_active(flag_type: &str, profile: &str) -> bool {
    match rule(flag_type) {
        Some(r) => r.applies_to(profile),
        None => true,
    }
}

/// Every rule active for a profile — the single source of truth for the command
/// layer and the rule-catalog UI.
pub fn rules_for_profile(profile: &str) -> Vec<&'static RuleDef> {
    RULES.iter().filter(|r| r.applies_to(profile)).collect()
}

// ── WP-77: compliance flag waivers ───────────────────────────────────────────
//
// A compliance flag is a *computed* signal, so it re-appears every time
// `get_compliance_flags` runs. Some flags are known-and-accepted (a documented
// exception, a permit renewal in progress) and should stop nagging without the
// operator having to fix the underlying condition. A waiver suppresses one
// specific `(flag_type, specimen_id)` pair, optionally until an expiry date,
// with a required reason recorded in the audit log. The decision logic below is
// pure so it is unit-tested here; the command layer supplies the waivers from
// `compliance_flag_waivers` and the filtering just reuses `is_waived`.

/// A stored waiver, reduced to the fields the suppression decision needs.
#[derive(Debug, Clone)]
pub struct Waiver {
    pub flag_type: String,
    pub specimen_id: String,
    /// ISO `YYYY-MM-DD`. `None` = permanent (until revoked).
    pub expires_at: Option<String>,
}

/// True if `waiver` is still in force as of `today` (ISO `YYYY-MM-DD`). ISO dates
/// compare correctly lexicographically, so a waiver is active while it has no
/// expiry or its expiry is on/after today.
pub fn waiver_is_active(waiver: &Waiver, today: &str) -> bool {
    match &waiver.expires_at {
        None => true,
        Some(exp) => exp.as_str() >= today,
    }
}

/// True if some active waiver suppresses the given flag for the given specimen.
pub fn is_waived(waivers: &[Waiver], flag_type: &str, specimen_id: &str, today: &str) -> bool {
    waivers.iter().any(|w| {
        w.flag_type == flag_type && w.specimen_id == specimen_id && waiver_is_active(w, today)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_flag_type_is_unique() {
        let mut seen = std::collections::HashSet::new();
        for r in RULES {
            assert!(seen.insert(r.flag_type), "duplicate flag_type: {}", r.flag_type);
        }
    }

    #[test]
    fn severities_are_from_the_known_set() {
        for r in RULES {
            assert!(
                matches!(r.severity, "critical" | "high" | "normal"),
                "rule {} has unexpected severity {}",
                r.flag_type,
                r.severity
            );
        }
    }

    #[test]
    fn citrus_hlb_is_ptc_only_not_a_cross_profile_leak() {
        // The core WP-74 fix: the citrus HLB rule must NOT fire in a mycology or
        // cell-culture lab, even though a species there could be coded "CIT-*".
        assert!(is_rule_active("missing_hlb_test", PLANT_TISSUE_CULTURE));
        assert!(!is_rule_active("missing_hlb_test", MYCOLOGY));
        assert!(!is_rule_active("missing_hlb_test", CELL_CULTURE));
    }

    #[test]
    fn general_rules_apply_in_every_profile() {
        for profile in [PLANT_TISSUE_CULTURE, CELL_CULTURE, MYCOLOGY] {
            assert!(is_rule_active("expired_permit", profile));
            assert!(is_rule_active("quarantine_no_release", profile));
            assert!(is_rule_active("positive_not_quarantined", profile));
        }
    }

    #[test]
    fn mycology_rules_are_mycology_only() {
        for ft in ["myco_open_contamination", "myco_overdue_transfer", "myco_slow_colonization"] {
            assert!(is_rule_active(ft, MYCOLOGY), "{ft} should fire in mycology");
            assert!(!is_rule_active(ft, PLANT_TISSUE_CULTURE), "{ft} must not fire in PTC");
            assert!(!is_rule_active(ft, CELL_CULTURE), "{ft} must not fire in cell culture");
        }
    }

    #[test]
    fn mycoplasma_rule_is_cell_culture_only() {
        assert!(is_rule_active("missing_mycoplasma_test", CELL_CULTURE));
        assert!(!is_rule_active("missing_mycoplasma_test", PLANT_TISSUE_CULTURE));
        assert!(!is_rule_active("missing_mycoplasma_test", MYCOLOGY));
    }

    #[test]
    fn rules_for_profile_counts_are_correct() {
        // 4 general (permits, quarantine, positive-not-quarantined, environmental)
        // + 1 PTC-specific (citrus HLB).
        assert_eq!(rules_for_profile(PLANT_TISSUE_CULTURE).len(), 5);
        // 4 general + 3 mycology.
        assert_eq!(rules_for_profile(MYCOLOGY).len(), 7);
        // 4 general + 1 cell-culture.
        assert_eq!(rules_for_profile(CELL_CULTURE).len(), 5);
    }

    #[test]
    fn an_unknown_profile_gets_only_general_rules() {
        // A plugin-supplied profile we don't recognize still gets the universal
        // regulatory-hygiene rules and none of the domain-specific ones.
        let active = rules_for_profile("some_future_profile");
        assert_eq!(active.len(), 4);
        assert!(active.iter().all(|r| r.scope == RuleScope::AllProfiles));
    }

    #[test]
    fn unknown_rule_is_fail_open() {
        assert!(is_rule_active("not_a_real_rule", MYCOLOGY));
    }

    // ── Waiver logic (WP-77) ────────────────────────────────────────────────

    fn waiver(flag: &str, spec: &str, exp: Option<&str>) -> Waiver {
        Waiver { flag_type: flag.to_string(), specimen_id: spec.to_string(), expires_at: exp.map(str::to_string) }
    }

    #[test]
    fn permanent_waiver_is_always_active() {
        let w = waiver("missing_hlb_test", "s1", None);
        assert!(waiver_is_active(&w, "2026-07-18"));
        assert!(waiver_is_active(&w, "2099-01-01"));
    }

    #[test]
    fn dated_waiver_expires_after_its_date() {
        let w = waiver("missing_hlb_test", "s1", Some("2026-07-18"));
        assert!(waiver_is_active(&w, "2026-07-18"), "active on the expiry day itself");
        assert!(waiver_is_active(&w, "2026-07-01"));
        assert!(!waiver_is_active(&w, "2026-07-19"), "expired the day after");
    }

    #[test]
    fn is_waived_matches_only_same_flag_and_specimen() {
        let ws = vec![waiver("missing_hlb_test", "s1", None)];
        assert!(is_waived(&ws, "missing_hlb_test", "s1", "2026-07-18"));
        // Different specimen — not waived.
        assert!(!is_waived(&ws, "missing_hlb_test", "s2", "2026-07-18"));
        // Different flag — not waived.
        assert!(!is_waived(&ws, "quarantine_no_release", "s1", "2026-07-18"));
    }

    #[test]
    fn expired_waiver_does_not_suppress() {
        let ws = vec![waiver("missing_hlb_test", "s1", Some("2026-06-30"))];
        assert!(!is_waived(&ws, "missing_hlb_test", "s1", "2026-07-18"));
    }
}
