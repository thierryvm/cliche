//! The shortcut registry: every combination this application announces, once.
//!
//! # Why the table is in RUST, and the TypeScript is only its reader
//!
//! Because Rust is what actually takes the combination from the operating
//! system. `shortcut::install` runs inside `setup`, before any webview exists,
//! and a shortcut this table did not describe is a shortcut nothing registers.
//! A second table on the JavaScript side would therefore not be a copy of the
//! truth - it would be a copy of a PROMISE, free to drift from what Windows was
//! actually asked for, and drift that no compiler and no type can see.
//! `src/shortcuts.ts` invokes [`describe_shortcuts`] and holds no data of its
//! own for exactly that reason.
//!
//! # `shortcuts.rs` and `shortcut.rs` are two different files
//!
//! One letter apart, said here because that is a trap and not a subtlety:
//!
//! - `shortcut.rs` (singular) INSTALLS the capture shortcut. It loads the
//!   plugin, binds one handler on the plugin's hotkey thread, and must never
//!   panic. Nothing there is reachable from a webview, and its header explains
//!   at length why that matters.
//! - `shortcuts.rs` (this file, plural) is the TABLE, plus the one command that
//!   hands it to the main window. It is webview-facing, so it carries a
//!   capability (`allow-describe-shortcuts`) and an `ipc::ensure_from` check.
//!
//! They were kept apart rather than merged for that last difference: declaring
//! a command inside `shortcut.rs` would falsify the paragraph its header spends
//! on "no command is declared here, so there is no `invoke` frontier for an ACL
//! to sit on" - which is the whole point that paragraph makes.

use std::str::FromStr;

use serde::Serialize;
use tauri::Webview;
use tauri_plugin_global_shortcut::Shortcut;

use crate::ipc;

/// Where a shortcut belongs in the in-app help.
///
/// A closed list rather than a free string: a category invented at the call
/// site is a heading the help page has to guess a place for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShortcutCategory {
    /// Taking a picture of the screen: the only thing this application does yet.
    Capture,
}

/// One shortcut, as the registry states it.
///
/// Serialised in camelCase to match `ShortcutEntry` in `src/shortcuts.ts`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutEntry {
    /// Stable identifier. Never shown; it is what code refers to an entry by,
    /// so that renaming a label or changing a combination breaks nothing.
    pub id: &'static str,
    /// The combination in the PLUGIN's syntax, the string
    /// `tauri_plugin_global_shortcut` is asked to register.
    pub accelerator: &'static str,
    /// The same combination as the interface DRAWS it, one chip per key.
    ///
    /// Not derived from `accelerator` at runtime, and that is a deliberate
    /// second statement of one fact: the plugin's syntax names physical keys
    /// (`Digit2`), while a user reads the character their own keyboard prints
    /// (`2` on this Belgian AZERTY, with Shift). No general rule turns one into
    /// the other. What keeps the two honest is
    /// `the_displayed_keys_are_the_combination_that_is_actually_registered`,
    /// which rebuilds this list from the parsed shortcut and refuses to guess.
    pub keys: &'static [&'static str],
    /// Key into `src/strings.ts`: what this shortcut DOES, in French.
    ///
    /// A key and not the sentence itself. The sentence is interface text, and
    /// interface text lives in one file - see the header of `src/strings.ts`.
    pub description_key: &'static str,
    /// The help page heading this entry files itself under.
    pub category: ShortcutCategory,
}

/// Identifier of the shortcut that starts a region capture.
///
/// A constant rather than the literal at the call site: `shortcut::install`
/// looks this entry up, and a typo there would leave the application with no
/// capture shortcut at all.
pub const CAPTURE_REGION: &str = "capture-region";

/// Every shortcut this application announces AND registers.
///
/// The two go together, and [`the_registry_holds_only_shortcuts_something_binds`]
/// is what keeps them together: `shortcut::install` binds ONE entry, by id, and
/// an entry added here without a handler would be a combination the help page
/// promises and nothing listens for. That is worse than a missing feature - the
/// user presses the keys and blames their keyboard.
///
/// `Digit2` rather than `2`: the parser accepts both, but a W3C code names a
/// PHYSICAL key rather than the character it produces. On the Belgian AZERTY
/// keyboard this project is used on, that key types `e` with an accent
/// unshifted and `2` with Shift - so `Ctrl+Shift+Digit2` is exactly the
/// "Ctrl + Maj + 2" the interface promises, and stays that key whatever the
/// active layout.
pub static REGISTRY: &[ShortcutEntry] = &[ShortcutEntry {
    id: CAPTURE_REGION,
    accelerator: "Ctrl+Shift+Digit2",
    keys: &["Ctrl", "Maj", "2"],
    description_key: "captureRegion",
    category: ShortcutCategory::Capture,
}];

/// The entry with this id, if the registry has one.
pub fn entry(id: &str) -> Option<&'static ShortcutEntry> {
    REGISTRY.iter().find(|candidate| candidate.id == id)
}

/// Parses an entry's combination with the plugin's own parser.
///
/// Split out because it is the one part of the registry that needs no event
/// loop: a unit test can hold every entry against the parser that will have to
/// register it.
pub fn parse(entry: &ShortcutEntry) -> Result<Shortcut, String> {
    Shortcut::from_str(entry.accelerator).map_err(|error| {
        format!(
            "`{}` (registry entry `{}`) is not a valid shortcut: {error}",
            entry.accelerator, entry.id
        )
    })
}

/// Hands the registry to the frontend.
///
/// **Main window only.** Not because a table of key names is dangerous - it is
/// not, and nothing here reads the screen or the clipboard. The rule this obeys
/// is `displays.rs`'s: every command declared in `generate_handler!` names the
/// window it serves, so that an unguarded command reads as an oversight rather
/// than as a considered exception. The veil has no help page and never asks.
///
/// Nothing is printed, unlike `describe_displays`. That command logs because
/// the monitor list is discovered at run time and a wrong one explains a wrong
/// capture; this one would only echo a constant back into the terminal.
#[tauri::command]
pub fn describe_shortcuts(webview: Webview) -> Result<Vec<ShortcutEntry>, String> {
    ipc::ensure_from(
        webview.label(),
        ipc::MAIN_WINDOW_LABEL,
        "describe_shortcuts",
    )?;

    Ok(REGISTRY.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use tauri_plugin_global_shortcut::{Code, Modifiers};

    /// The label a key CODE is drawn with, rebuilt from the parsed shortcut.
    ///
    /// This is the checker, not a second source of truth: it exists so that
    /// [`ShortcutEntry::keys`] can be compared against something written
    /// independently of it. Hence the return of `None` for anything it has not
    /// been taught - a checker that guesses would pass a wrong label through
    /// silently, which is the exact failure it is here to catch.
    ///
    /// `Code` prints its W3C name (`keyboard-types-0.7.0/src/code.rs:465`), so
    /// the two families that matter can be derived from it rather than listed
    /// one by one: `Digit2` -> `2`, `KeyA` -> `A`.
    fn expected_label(code: Code) -> Option<String> {
        let name = code.to_string();

        for prefix in ["Digit", "Key"] {
            if let Some(rest) = name.strip_prefix(prefix) {
                let mut characters = rest.chars();
                return match (characters.next(), characters.next()) {
                    (Some(single), None) if single.is_ascii_alphanumeric() => {
                        Some(single.to_string())
                    }
                    _ => None,
                };
            }
        }

        // A function key is drawn as it is named. `Escape` is not: the
        // interface writes the French key cap, which is what is engraved on the
        // keyboard this application is used on.
        let is_function_key = name.starts_with('F')
            && name.len() > 1
            && name[1..]
                .chars()
                .all(|character| character.is_ascii_digit());

        match name.as_str() {
            "Escape" => Some("Échap".to_owned()),
            _ if is_function_key => Some(name),
            _ => None,
        }
    }

    /// The full list of chips an entry should draw, rebuilt from its parsed
    /// combination.
    ///
    /// Modifier order is Ctrl, Maj, Alt, Windows - the order the design system
    /// draws them in (`src/design/Showcase.tsx`, the five-key specimen). Order
    /// is part of what is checked: "Maj + Ctrl + 2" names the same combination
    /// and is not what the system publishes.
    fn expected_keys(shortcut: &Shortcut) -> Option<Vec<String>> {
        let mut keys = Vec::new();

        for (modifier, label) in [
            (Modifiers::CONTROL, "Ctrl"),
            (Modifiers::SHIFT, "Maj"),
            (Modifiers::ALT, "Alt"),
            (Modifiers::SUPER, "Windows"),
        ] {
            if shortcut.mods.contains(modifier) {
                keys.push(label.to_owned());
            }
        }

        keys.push(expected_label(shortcut.key)?);
        Some(keys)
    }

    /// The keys declared by a TypeScript object literal, one per line.
    ///
    /// Deliberately naive, like `scripts/check-version.mjs`'s reading of
    /// Cargo.toml: a key is a line's leading identifier, followed by a colon.
    /// Anything else - prose, a comment, a type declaration - fails the shape
    /// test and is dropped. `every_description_key_exists_in_the_string_catalogue`
    /// runs it over a sample first, so a parser that has quietly stopped
    /// finding keys is caught rather than read as a clean catalogue.
    fn keys_in(source: &str) -> HashSet<&str> {
        source
            .lines()
            .filter_map(|line| line.trim().split_once(':'))
            .map(|(key, _)| key.trim())
            .filter(|key| {
                key.chars().next().is_some_and(char::is_alphabetic)
                    && key
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric())
            })
            .collect()
    }

    #[test]
    fn the_registry_is_not_empty_so_the_tests_below_assert_something() {
        // Every other test here iterates the table. On an empty table they
        // would all pass, for ever, over an application with no shortcut at
        // all.
        assert!(
            !REGISTRY.is_empty(),
            "an empty registry makes every assertion in this file vacuous"
        );
        assert!(
            entry(CAPTURE_REGION).is_some(),
            "`{CAPTURE_REGION}` is the entry `shortcut::install` binds; without it the \
             application starts with no capture shortcut"
        );
    }

    #[test]
    fn no_two_entries_claim_the_same_combination() {
        // Compared on the PARSED shortcut, not on the accelerator string:
        // `Ctrl+Shift+Digit2` and `Shift+Ctrl+2` are two spellings of one
        // combination, and the operating system would refuse the second
        // registration - at run time, on somebody else's machine.
        let mut seen = HashSet::new();

        for candidate in REGISTRY {
            let shortcut = parse(candidate).expect("every entry must parse");

            assert!(
                seen.insert(shortcut),
                "`{}` (`{}`) claims a combination another entry already holds; only one of \
                 the two could ever be registered",
                candidate.id,
                candidate.accelerator
            );
        }
    }

    #[test]
    fn no_two_entries_share_an_identifier() {
        // Ids are how code reaches an entry: `entry()` returns the FIRST match,
        // so a duplicate would make one of the two unreachable and silently
        // unbindable.
        let mut seen = HashSet::new();

        for candidate in REGISTRY {
            assert!(
                seen.insert(candidate.id),
                "`{}` is used by two entries; `entry()` would only ever return the first",
                candidate.id
            );
        }
    }

    #[test]
    fn every_combination_parses_with_the_plugins_real_parser() {
        // Held against the parser that will have to register them. A typo fails
        // here rather than at run time, in a message nobody is watching for.
        for candidate in REGISTRY {
            if let Err(reason) = parse(candidate) {
                panic!("`{}` does not parse: {reason}", candidate.id);
            }
        }
    }

    #[test]
    fn the_parser_rejects_nonsense_so_the_test_above_is_not_vacuous() {
        // The test above is a blanket "they all parse" over a table that holds
        // one entry today. Without this, it could read as green because the
        // parser accepts anything at all. (`shortcut.rs` states the same fact
        // for its own neighbour, which pins exact values instead; both are kept
        // because each guards a different assertion.)
        for nonsense in ["Ctrl+Shift+NotAKey", "", "Ctrl+Shift", "Ctrl++Digit2"] {
            assert!(
                Shortcut::from_str(nonsense).is_err(),
                "`{nonsense}` was accepted by the parser, so `every_combination_parses...` \
                 proves nothing"
            );
        }
    }

    #[test]
    fn the_displayed_keys_are_the_combination_that_is_actually_registered() {
        // THE lie this registry makes possible: publish "Ctrl + Maj + 2" in the
        // help while asking Windows for something else. Nothing about the two
        // fields forces them to agree, so they are compared - the drawn chips
        // against a list rebuilt from the parsed combination.
        for candidate in REGISTRY {
            let shortcut = parse(candidate).expect("every entry must parse");

            let expected = expected_keys(&shortcut).unwrap_or_else(|| {
                panic!(
                    "the checker in this test file has no label for `{}`, used by `{}`. Teach \
                     `expected_label` that key rather than removing this assertion: an unchecked \
                     entry is one the help page may draw wrongly.",
                    shortcut.key, candidate.id
                )
            });

            let drawn = candidate.keys.to_vec();

            assert_eq!(
                drawn, expected,
                "`{}` draws {:?} but registers `{}`, which is {:?}",
                candidate.id, candidate.keys, candidate.accelerator, expected
            );
        }
    }

    #[test]
    fn the_checker_refuses_a_key_it_was_never_taught() {
        // Without this, the test above could be green because `expected_keys`
        // returns something for everything - and it would then compare the
        // table against a guess.
        assert!(
            expected_label(Code::MediaPlayPause).is_none(),
            "the checker invented a label for a key nobody chose a French cap for"
        );
        assert_eq!(expected_label(Code::Digit2).as_deref(), Some("2"));
        assert_eq!(expected_label(Code::KeyA).as_deref(), Some("A"));
    }

    #[test]
    fn every_description_key_exists_in_the_string_catalogue() {
        // The registry names its description by KEY; the sentence lives in
        // `src/strings.ts`. A key with no entry there is a help page with a
        // blank line on it - and nothing in either language would say so.
        //
        // Read at compile time, so the file is a build input: editing the
        // catalogue re-runs this test. The reading is deliberately naive, like
        // `scripts/check-version.mjs`'s of Cargo.toml - a key is a line's
        // leading identifier followed by a colon.
        //
        // So the parser is put to a sample whose answer is known BEFORE it is
        // pointed at the real catalogue. Without that, a parser that finds
        // nothing - or one that starts reading prose out of the comments -
        // would keep this test green whatever src/strings.ts holds.
        let sample = "export const UI_STRINGS = {\n  \
                      sampleKey: 'Fermer',\n  \
                      // note: this line is not a key\n\
                      } as const;\n";
        let sampled = keys_in(sample);

        assert!(
            sampled.contains("sampleKey"),
            "the parser missed a key of its own sample: {sampled:?}"
        );
        assert_eq!(
            sampled.len(),
            1,
            "the parser read something other than keys out of its own sample: {sampled:?}"
        );

        let keys = keys_in(include_str!("../../src/strings.ts"));

        for candidate in REGISTRY {
            assert!(
                keys.contains(candidate.description_key),
                "`{}` points at `{}`, which src/strings.ts does not define",
                candidate.id,
                candidate.description_key
            );
        }
    }

    #[test]
    fn the_registry_holds_only_shortcuts_something_binds() {
        // A TRIPWIRE, and it is meant to be tripped: `shortcut::install` binds
        // exactly one entry, by id. Adding a second entry here without giving
        // it a handler would publish a combination in the help page that
        // nothing registers - the user presses the keys, nothing happens, and
        // the application looks broken rather than unfinished.
        //
        // The fix when this fails is not to raise the number: it is to decide,
        // in `shortcut::install`, what the new entry does.
        assert_eq!(
            REGISTRY.len(),
            1,
            "the registry grew past what `shortcut::install` binds. Give the new entry a \
             handler there, then update this count."
        );
    }
}
