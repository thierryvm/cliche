//! Which window is allowed to call which command.
//!
//! # Why this module exists: Tauri's ACL does NOT guard our own commands
//!
//! It is natural to assume that `capabilities/default.json` - which lists
//! `"windows": ["main"]` and grants `core:default` only - decides who may
//! `invoke` what. For the CORE and PLUGIN commands, it does. For the four
//! commands declared in this crate's `generate_handler!`, it does not, and the
//! difference is not a subtlety: it is the whole guard.
//!
//! Read in the vendored `tauri` 2.11.5 source, `src/webview/mod.rs:1823`:
//!
//! ```text
//! if (plugin_command.is_some() || has_app_acl_manifest || !is_local) && invoke.acl.is_none()
//! ```
//!
//! A request is rejected for want of a capability only when one of those three
//! holds. For a call to one of OUR commands, from a local origin:
//!
//! - `plugin_command` is `None` - the command name carries no `plugin:` prefix;
//! - `has_app_acl_manifest` is `false`. Verified on 4 September 2026, twice:
//!   `src-tauri/permissions/` does not exist, and `gen/schemas/acl-manifests.json`
//!   holds `core`, `core:*`, `clipboard-manager` and `global-shortcut` and no
//!   application key at all;
//! - `is_local` is `true` - both windows load bundled documents.
//!
//! So the condition is false and **no ACL check runs**. Any local webview of
//! this process can invoke any of our commands. The `main` window runs React
//! and its dependency tree; one compromised package there could call
//! `veil_selected` with a huge rectangle and put the frozen screen on the
//! system clipboard without the user drawing anything, or call `veil_dismissed`
//! to cancel every capture.
//!
//! Hence: each command checks the webview it came from, here, in Rust. That is
//! the guard - not the capability file.
//!
//! # What this does NOT protect against
//!
//! Said plainly, because a guard whose limits are unwritten gets trusted past
//! them. This refuses a call from the WRONG WINDOW. It does nothing against
//! code executing inside the veil window itself: whatever runs there is, as far
//! as this check can tell, the veil. `veil.html` loading no third-party script
//! is what makes that acceptable, and it is a separate fact from this one.

/// Label of the main window, as declared in `tauri.conf.json`.
///
/// One constant so the config and the guard cannot drift apart.
pub const MAIN_WINDOW_LABEL: &str = "main";

/// The line a refused call produces.
///
/// Pure, so its wording is under test. It names the command, the window that is
/// allowed and the window that called: a refusal nobody can attribute is a
/// refusal nobody can act on.
pub fn wrong_window_line(command: &str, expected: &str, actual: &str) -> String {
    format!(
        "`{command}` is served to the `{expected}` window only; this call came from `{actual}` \
         and was REFUSED"
    )
}

/// Whether a webview may call a command reserved to `expected`.
///
/// An exact string comparison, deliberately: no prefix, no case folding, no
/// "starts with". Window labels are chosen by this application, in
/// `tauri.conf.json` and in `veil::VEIL_WINDOW_LABEL`, and a loose match is how
/// a guard like this one quietly stops guarding.
pub fn is_from(actual: &str, expected: &str) -> bool {
    actual == expected
}

/// Refuses a command that did not come from the window it belongs to.
///
/// Takes the label rather than the `Webview` so the decision stays testable:
/// constructing a `Webview` needs a running event loop, a `&str` does not. The
/// callers pass `webview.label()`, which Tauri fills in from the webview that
/// actually sent the message - it is not a value the page can choose.
pub fn ensure_from(actual: &str, expected: &str, command: &str) -> Result<(), String> {
    if is_from(actual, expected) {
        Ok(())
    } else {
        Err(wrong_window_line(command, expected, actual))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::veil::VEIL_WINDOW_LABEL;

    #[test]
    fn the_two_window_labels_are_the_ones_the_application_actually_creates() {
        // `main` is declared in tauri.conf.json, `veil` is the label
        // `WebviewWindowBuilder` is given in `veil::create`. If either is
        // renamed there and not here, every command starts refusing everything
        // - loudly, which is the failure this guard is allowed to have.
        assert_eq!(MAIN_WINDOW_LABEL, "main");
        assert_eq!(VEIL_WINDOW_LABEL, "veil");
        assert_ne!(
            MAIN_WINDOW_LABEL, VEIL_WINDOW_LABEL,
            "two windows sharing a label would make this guard a no-op"
        );
    }

    #[test]
    fn the_veil_commands_refuse_the_main_window() {
        // THE case this module exists for: React, and its 150-plus packages,
        // asking Rust to cut the frozen screen into the clipboard.
        let refusal = ensure_from(MAIN_WINDOW_LABEL, VEIL_WINDOW_LABEL, "veil_selected")
            .expect_err("a call from `main` to a veil command must be refused");

        assert!(refusal.contains("veil_selected"), "{refusal}");
        assert!(refusal.contains("main"), "{refusal}");
        assert!(refusal.contains("veil"), "{refusal}");
        assert!(
            refusal.contains("REFUSED"),
            "the line must say the call did not happen, not merely that it was \
             unusual: {refusal}"
        );
    }

    #[test]
    fn the_window_a_command_belongs_to_is_accepted() {
        assert!(ensure_from(VEIL_WINDOW_LABEL, VEIL_WINDOW_LABEL, "veil_painted").is_ok());
        assert!(ensure_from(MAIN_WINDOW_LABEL, MAIN_WINDOW_LABEL, "describe_displays").is_ok());
    }

    #[test]
    fn a_label_is_matched_exactly_and_never_loosely() {
        // Each of these would pass a `starts_with`, a `contains` or a
        // case-insensitive comparison. None of them is the veil window.
        for impostor in ["veil2", "Veil", "veil ", " veil", "veil-decoy", ""] {
            assert!(
                !is_from(impostor, VEIL_WINDOW_LABEL),
                "`{impostor}` must not pass for the veil window"
            );
        }
    }
}
