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
//! - `shortcuts.rs` (this file, plural) is the TABLE, plus the three commands
//!   that hand the main window what it needs to talk about shortcuts: the table
//!   itself, what the operating system ANSWERED when `shortcut::install` offered
//!   it the capture combination, and - since 6 September 2026 - the one command
//!   that CHANGES that combination. All three are webview-facing, so each
//!   carries a capability (`allow-describe-shortcuts`,
//!   `allow-describe-shortcut-status`, `allow-set-capture-shortcut`) and an
//!   `ipc::ensure_from` check. The status TYPE lives in `shortcut.rs`, next to
//!   the only function that can produce one; this file is where it crosses to a
//!   webview.
//!
//! They were kept apart rather than merged for that last difference: declaring
//! a command inside `shortcut.rs` would falsify the paragraph its header spends
//! on "no command is declared here, so there is no `invoke` frontier for an ACL
//! to sit on" - which is the whole point that paragraph makes.
//!
//! # THE TABLE IS A PROMISE; THE CAPTURE ROW IS WHAT WAS ACTUALLY OFFERED
//!
//! Since the combination became settable, [`REGISTRY`] states what this
//! application ships with and no longer what it holds on this machine.
//! [`describe_shortcuts`] therefore does not hand the table over as it stands:
//! it builds [`ShortcutRow`]s and puts the combination that was actually offered
//! to the system in the capture row - see [`rows`]. Everything downstream reads
//! that, so the help page and the launcher's reminder go on naming the
//! combination the user can really press, with not one file under `src/`
//! knowing that a shortcut is settable at all.

use std::str::FromStr;

use serde::Serialize;
use tauri::{AppHandle, Manager, Webview};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

use crate::ipc;
use crate::shortcut::{CaptureShortcut, ShortcutChange, ShortcutStatus};

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

/// Parses a combination with the plugin's own parser.
///
/// Split out because it is the one part of the registry that needs no event
/// loop: a unit test can hold every entry against the parser that will have to
/// register it.
///
/// Takes the ACCELERATOR and not an entry, since 6 September 2026: a settable
/// shortcut arrives as a bare string from the page, and it has to go through the
/// very parser the table's own rows go through. One door, not two.
pub fn parse(accelerator: &str) -> Result<Shortcut, String> {
    Shortcut::from_str(accelerator)
        .map_err(|error| format!("`{accelerator}` is not a valid shortcut: {error}"))
}

/// A combination this application is willing to register, and the caps it draws.
///
/// The two travel together on purpose. An accelerator alone would leave every
/// reader to guess what to put on screen for it, and guessing is what the
/// registry's `keys` field exists to refuse - see its own comment. For a
/// combination the USER chose there is no hand-written row to read, so the caps
/// are derived once, here, by [`accept`], and carried everywhere afterwards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Combination {
    /// The combination in the PLUGIN's syntax, canonical - see [`accept`].
    pub accelerator: String,
    /// The same combination as the interface draws it, one chip per key.
    pub keys: Vec<String>,
}

/// Every modifier this application will register, in the order it draws them.
///
/// Three columns, and the third is the reason this is a table rather than two
/// `match` arms: the plugin's TOKEN (`Shift`) and the French CAP (`Maj`) are not
/// the same string, and the ORDER is shared by both. « Maj + Ctrl + 2 » names
/// the same combination and is not what this application publishes.
///
/// The list is also the closed set of modifiers [`accept`] will take: a
/// combination carrying a bit that is not here would be drawn without it, which
/// is the one way a cap row can lie about what was registered.
const MODIFIERS: [(Modifiers, &str, &str); 4] = [
    (Modifiers::CONTROL, "Ctrl", "Ctrl"),
    (Modifiers::SHIFT, "Shift", "Maj"),
    (Modifiers::ALT, "Alt", "Alt"),
    (Modifiers::SUPER, "Super", "Windows"),
];

/// The cap a physical key code is drawn with, or `None` for one nobody chose.
///
/// `Code` prints its W3C name, so the two families that matter are derived from
/// it rather than listed one by one: `Digit2` -> `2`, `KeyA` -> `A`. A physical
/// code names a KEY and not the character it produces; on the Belgian AZERTY
/// this application is used on, the `Digit2` key types `2` with Shift, which is
/// exactly what the registry's hand-written row already says.
///
/// `None` rather than a guess, and that is the whole design: an unknown key is
/// refused by [`accept`] instead of being published under a cap invented here.
/// A user cannot press a key whose name the screen got wrong.
fn cap(code: Code) -> Option<String> {
    let name = code.to_string();

    for prefix in ["Digit", "Key"] {
        if let Some(rest) = name.strip_prefix(prefix) {
            let mut characters = rest.chars();
            return match (characters.next(), characters.next()) {
                (Some(single), None) if single.is_ascii_alphanumeric() => Some(single.to_string()),
                _ => None,
            };
        }
    }

    // A function key is drawn as it is named. `Escape` is not: the interface
    // writes the French key cap, which is what is engraved on the keyboard this
    // application is used on.
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

/// Whether this application will offer a combination to the operating system.
///
/// # THE frontier for anything that did not come from [`REGISTRY`]
///
/// Pure, and pure on purpose: everything a user can type reaches the system
/// through here, and a decision that needs an event loop is a decision no test
/// can put a bad combination to. The page validates too (`src/shortcut-recorder.ts`),
/// and that is not a duplicate - what arrives over IPC is never trusted, and a
/// second webview, a devtools console or a widened capability would all bypass
/// the first check.
///
/// Four refusals, and the second is the one that matters most:
///
/// 1. What the plugin's own parser will not read.
/// 2. A combination with NO MODIFIER. `Digit2` registered globally would take
///    that key away from every other application on the machine - the user could
///    no longer type a `2` anywhere - and Windows would happily allow it.
/// 3. A modifier outside [`MODIFIERS`], which would be registered and never
///    drawn.
/// 4. A key this application has no cap for, which would be registered and drawn
///    under a name invented on the spot.
///
/// The accelerator that comes back is CANONICAL: rebuilt from the parsed
/// combination in the order of [`MODIFIERS`], never the caller's own spelling.
/// That is what makes `Shift+Ctrl+2` and `Ctrl+Shift+Digit2` one value rather
/// than two, and it is also what lets `settings::render` write the file without
/// escaping anything - a canonical accelerator is ASCII letters, digits and `+`.
pub fn accept(input: &str) -> Result<Combination, String> {
    let shortcut = parse(input)?;

    if shortcut.mods.is_empty() {
        return Err(format!(
            "`{input}` carries no modifier. Registered globally, that key would be taken from \
             every other application on this machine"
        ));
    }

    let known = MODIFIERS
        .iter()
        .fold(Modifiers::empty(), |all, (modifier, _, _)| all | *modifier);
    if !shortcut.mods.difference(known).is_empty() {
        return Err(format!(
            "`{input}` carries a modifier this application does not draw, so it would be \
             registered and never shown"
        ));
    }

    let Some(cap) = cap(shortcut.key) else {
        return Err(format!(
            "`{input}` ends on `{}`, a key this application has no cap for. It would be \
             registered and drawn under a name invented on the spot",
            shortcut.key
        ));
    };

    let mut accelerator = String::new();
    let mut keys = Vec::new();

    for (modifier, token, label) in MODIFIERS {
        if shortcut.mods.contains(modifier) {
            accelerator.push_str(token);
            accelerator.push('+');
            keys.push(label.to_owned());
        }
    }

    accelerator.push_str(&shortcut.key.to_string());
    keys.push(cap);

    Ok(Combination { accelerator, keys })
}

/// One row of the table as the frontend receives it.
///
/// The owned twin of [`ShortcutEntry`], and it exists because the capture
/// combination is no longer a constant: `&'static str` cannot hold a string a
/// user chose at run time. Field names and their meaning are unchanged, so
/// `ShortcutEntry` in `src/shortcuts.ts` reads both without knowing which it
/// got.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutRow {
    pub id: &'static str,
    pub accelerator: String,
    pub keys: Vec<String>,
    pub description_key: &'static str,
    pub category: ShortcutCategory,
}

/// The table as it should be PUBLISHED, given what was offered to the system.
///
/// Pure, so the substitution is under test without an event loop. The capture
/// row carries `capture` when there is one; every other row - and the capture
/// row on a launch where nothing was ever offered - carries the registry's own
/// hand-written values, because those are then the only honest thing to show.
///
/// Note which combination is passed in by [`describe_shortcuts`]: the one that
/// was ATTEMPTED, not the one that was accepted. A combination Windows refused
/// is still the one this application asked for, and the launcher's refusal note
/// names it - see `src/shortcut-hint.ts`, which compares the accelerator it was
/// given against this table's before it draws any cap at all.
pub fn rows(capture: Option<&Combination>) -> Vec<ShortcutRow> {
    REGISTRY
        .iter()
        .map(|entry| {
            let published = match capture {
                Some(combination) if entry.id == CAPTURE_REGION => combination.clone(),
                _ => Combination {
                    accelerator: entry.accelerator.to_owned(),
                    keys: entry.keys.iter().map(|key| (*key).to_owned()).collect(),
                },
            };

            ShortcutRow {
                id: entry.id,
                accelerator: published.accelerator,
                keys: published.keys,
                description_key: entry.description_key,
                category: entry.category,
            }
        })
        .collect()
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
/// Since 6 September 2026 it hands over [`rows`] rather than [`REGISTRY`]
/// itself, so that the capture row names the combination this launch really
/// offered the system instead of the one the source ships with. A state that is
/// not managed at all is not an error here: the table is still true of what this
/// application intends to hold, and refusing to list any shortcut because the
/// registration state is missing would take the help page down over a
/// diagnostic.
#[tauri::command]
pub fn describe_shortcuts(app: AppHandle, webview: Webview) -> Result<Vec<ShortcutRow>, String> {
    ipc::ensure_from(
        webview.label(),
        ipc::MAIN_WINDOW_LABEL,
        "describe_shortcuts",
    )?;

    let attempted = app
        .try_state::<CaptureShortcut>()
        .and_then(|state| state.attempted());

    Ok(rows(attempted.as_ref()))
}

/// Changes the combination that starts a capture, at once.
///
/// # What "at once" means, and why the alternative was refused
///
/// The old combination is released and the new one taken before this returns.
/// Thierry decided it on 5 September 2026, and the reason is not comfort: a
/// setting that only took effect after a restart would look, to the person who
/// just used it, exactly like a setting that did not work - they press the new
/// keys, nothing happens, and the honest conclusion is that the application is
/// broken.
///
/// # THE ROLLBACK IS THE PART THAT MATTERS
///
/// The sequence, its order and every way it can fail live in
/// [`crate::shortcut::change`], which is pure and takes the registrar as an
/// argument precisely so that the refusal path is a test rather than a hope. The
/// promise it keeps: a combination the system refuses leaves the PREVIOUS one
/// registered, and the answer below says so, naming both.
///
/// Nothing is written to disk unless the new combination was really taken. A
/// file holding a combination Windows refuses would hand the same failure to
/// every launch afterwards, and the user would have to find the file to get out
/// of it.
///
/// **Main window only**, by capability AND in Rust. This is the widest of the
/// three shortcut commands by a distance: it takes a global hotkey from the
/// operating system and writes a file. The veil has no settings screen.
#[tauri::command]
pub fn set_capture_shortcut(
    app: AppHandle,
    webview: Webview,
    accelerator: String,
) -> Result<ShortcutChange, String> {
    ipc::ensure_from(
        webview.label(),
        ipc::MAIN_WINDOW_LABEL,
        "set_capture_shortcut",
    )?;

    // Validated HERE, before anything is unregistered. What arrives from a
    // webview is a string and nothing more; `accept` is the only door onto the
    // operating system, and its refusals are the four listed on it.
    let wanted = accept(&accelerator)?;

    crate::shortcut::change_capture_shortcut(&app, &wanted)
}

/// Hands the frontend what the operating system ANSWERED about the capture
/// shortcut.
///
/// # This is the other half of [`describe_shortcuts`], and the two are not the
/// # same fact
///
/// The registry above is a PROMISE: the combinations this application intends to
/// hold. This command is what became of that promise on this machine, this
/// launch - see [`crate::shortcut::ShortcutStatus`]. A launcher that reads only
/// the registry can draw a combination nothing is listening for, which is the
/// state Cliche shipped in until 6 September 2026.
///
/// The status is read from managed state rather than recomputed: a registration
/// is something that HAPPENED - in `setup`, or in the last
/// [`set_capture_shortcut`] - and asking the plugin again would answer about a
/// registration nobody performed. The state is a mutex since the combination
/// became settable, so this now reports the LAST answer rather than the first;
/// `CaptureShortcut` in `shortcut.rs` is where that is kept.
///
/// `try_state` and not `state`, like everything else that runs on a webview IPC
/// thread: `state` panics when the type was never managed, and taking the
/// application down over a missing diagnostic is a bad trade.
///
/// # IT USED TO FAIL ON A LAUNCH THAT WAS SIMPLY STILL STARTING
///
/// Until 7 September 2026 an unmanaged state was the ONLY answer this command
/// had for a launcher that asked early, and asking early is the ordinary case:
/// Tauri builds the windows declared in `tauri.conf.json` before it calls our
/// `setup` (`tauri-2.11.5/src/app.rs:2521-2535`), so the launcher boots and
/// invokes while `setup` is still enumerating monitors and building the veil's
/// WebView2. The error crossed to the page, and the page drew a red "the
/// shortcut registry could not be read" over a shortcut that worked. `setup`
/// now manages a [`crate::shortcut::ShortcutStatus::Starting`] state on its
/// first instruction, so the honest answer exists at every instant.
///
/// The error path is KEPT, and [`status_answer`] is where it lives. It no longer
/// describes a race - it describes a `setup` that never ran at all, which is a
/// programming error, and swallowing it with a default would hide it.
///
/// **Main window only**, by capability AND in Rust, for the reason `displays.rs`
/// gives: every command names the window it serves.
#[tauri::command]
pub fn describe_shortcut_status(
    app: AppHandle,
    webview: Webview,
) -> Result<ShortcutStatus, String> {
    ipc::ensure_from(
        webview.label(),
        ipc::MAIN_WINDOW_LABEL,
        "describe_shortcut_status",
    )?;

    status_answer(
        app.try_state::<CaptureShortcut>()
            .map(|managed| managed.status()),
    )
}

/// What [`describe_shortcut_status`] answers, given what is managed.
///
/// Split out of the command because a command cannot be reached from a test: it
/// takes a `Webview`, and building one needs a running event loop - the same
/// constraint that put `shortcut::change` behind a trait and `ipc::ensure_from`
/// behind a `&str`. What is left in the command is one lookup; the DECISION is
/// here, where `an_undecided_status_is_answered_and_not_refused` holds it.
///
/// Whatever is managed is handed over UNCHANGED, every variant of it. That is
/// the rule, and it is worth stating because the tempting shortcut is the one
/// that caused the defect: filtering a state the frontend might not know about
/// turns a launch that is merely starting into a launch that failed.
fn status_answer(managed: Option<ShortcutStatus>) -> Result<ShortcutStatus, String> {
    managed.ok_or_else(|| {
        "no shortcut status is managed at all. Since 7 September 2026 `setup` manages one on its \
         FIRST instruction (src-tauri/src/lib.rs), before anything slow runs, so this is not a \
         call that arrived too early: it means `setup` never ran. That is a programming error in \
         this application, not a machine problem."
            .to_owned()
    })
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
    /// # DO NOT REPLACE THIS WITH A CALL TO [`cap`]
    ///
    /// Production grew its own copy of this rule on 6 September 2026, because a
    /// combination the USER chose has no hand-written row to read a cap from.
    /// The two are deliberately separate statements of one rule, and the
    /// registry's hand-written `keys` is what both are held against - this one
    /// by `the_displayed_keys_are_the_combination_that_is_actually_registered`,
    /// `cap` by `the_derived_caps_agree_with_the_hand_written_registry_row`.
    /// Calling `cap` here would make the first of those compare a derivation
    /// with itself, and it would pass for ever.
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
            let shortcut = parse(candidate.accelerator).expect("every entry must parse");

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
            if let Err(reason) = parse(candidate.accelerator) {
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
            let shortcut = parse(candidate.accelerator).expect("every entry must parse");

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

    // --- the settable combination -------------------------------------------

    #[test]
    fn the_derived_caps_agree_with_the_hand_written_registry_row() {
        // What ties `cap` to something nobody derived. The registry states its
        // caps BY HAND (`keys: &["Ctrl", "Maj", "2"]`), and production now has
        // to produce the same list for a combination a user typed. If the two
        // ever disagree, one of the two screens is lying about which keys to
        // press - and the one that lies is whichever the reader is looking at.
        for candidate in REGISTRY {
            let derived = accept(candidate.accelerator)
                .unwrap_or_else(|reason| panic!("`{}` must be acceptable: {reason}", candidate.id));

            assert_eq!(
                derived.keys, candidate.keys,
                "`{}` is drawn {:?} by hand and {:?} by `accept`",
                candidate.id, candidate.keys, derived.keys
            );
        }
    }

    #[test]
    fn an_accepted_combination_is_canonical_and_parses_back() {
        // THE property `settings.rs` leans on: what `accept` hands back is
        // written to disk as it stands, and read again at the next launch. A
        // canonical form the parser could not read back would lose the user's
        // shortcut on the FIRST restart, silently, with the registry's own
        // combination taking its place.
        for (typed, canonical) in [
            ("Shift+Ctrl+Digit2", "Ctrl+Shift+Digit2"),
            ("ctrl+shift+KeyA", "Ctrl+Shift+KeyA"),
            ("Alt+Ctrl+F5", "Ctrl+Alt+F5"),
        ] {
            let accepted = accept(typed).unwrap_or_else(|reason| panic!("`{typed}`: {reason}"));

            assert_eq!(
                accepted.accelerator, canonical,
                "`{typed}` was not brought to the one spelling this application stores"
            );
            assert!(
                parse(&accepted.accelerator).is_ok(),
                "`{}` is a spelling this application's own parser refuses",
                accepted.accelerator
            );
            assert_eq!(
                accept(&accepted.accelerator).map(|again| again.accelerator),
                Ok(accepted.accelerator.clone()),
                "a canonical accelerator must survive a second pass unchanged"
            );
        }
    }

    #[test]
    fn a_canonical_accelerator_can_be_written_to_the_settings_file_unescaped() {
        // `settings::render` escapes nothing and refuses a quote or a backslash.
        // This is the other half of that decision: what `accept` produces can
        // only ever be ASCII letters, digits and `+`.
        for typed in ["Ctrl+Shift+Digit2", "Ctrl+Alt+Shift+Super+F12", "Ctrl+KeyZ"] {
            let accepted = accept(typed).unwrap_or_else(|reason| panic!("`{typed}`: {reason}"));

            assert!(
                accepted
                    .accelerator
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '+'),
                "`{}` holds something `settings::render` would refuse to write",
                accepted.accelerator
            );
            assert!(
                crate::settings::render(&accepted.accelerator).is_ok(),
                "`{}` cannot be written down, so choosing it would not survive a restart",
                accepted.accelerator
            );
        }
    }

    #[test]
    fn a_combination_with_no_modifier_is_refused() {
        // THE dangerous one. `Digit2` registered globally takes that key from
        // every other application on the machine: the user can no longer type a
        // `2` anywhere, in any program, and the only way out is this
        // application's own settings screen. Windows does not stop it.
        for bare in ["Digit2", "KeyA", "F5", "Escape"] {
            let refusal = accept(bare)
                .expect_err("a combination with no modifier must never reach the operating system");

            assert!(
                refusal.contains(bare),
                "the refusal names no combination: {refusal}"
            );
            assert!(
                refusal.contains("modifier"),
                "the refusal must say WHAT is missing, or nobody can act on it: {refusal}"
            );
        }
    }

    #[test]
    fn a_key_with_no_cap_is_refused_rather_than_drawn_under_an_invented_name() {
        // The other half of `cap` returning `None`. Registering a key the
        // interface cannot name would put a shortcut in the help page under a
        // label somebody made up at the call site.
        let refusal = accept("Ctrl+Shift+MediaPlayPause")
            .expect_err("a key this application cannot draw must not be registered");

        assert!(refusal.contains("MediaPlayPause"), "{refusal}");
    }

    #[test]
    fn nonsense_never_reaches_the_operating_system() {
        // Without this the four refusals above could be green over an `accept`
        // that says no to everything, and the ones below could be green over an
        // `accept` that says yes to everything.
        for nonsense in ["", "Ctrl+", "Ctrl+Shift", "NotAKey", "Ctrl+Shift+NotAKey"] {
            assert!(
                accept(nonsense).is_err(),
                "`{nonsense}` was accepted as a combination"
            );
        }
        for real in ["Ctrl+Shift+Digit2", "Ctrl+Alt+KeyQ", "Ctrl+Shift+F1"] {
            assert!(
                accept(real).is_ok(),
                "`{real}` was refused, so this application can register nothing at all"
            );
        }
    }

    #[test]
    fn the_published_table_names_the_combination_that_was_offered() {
        // THE substitution the help page and the launcher both read. Without
        // it, changing the shortcut would leave every screen announcing the
        // combination the source ships with - the user presses the keys they
        // just chose and the application tells them to press other ones.
        let chosen = accept("Ctrl+Alt+KeyQ").expect("the combination under test must be valid");
        let published = rows(Some(&chosen));

        let capture = published
            .iter()
            .find(|row| row.id == CAPTURE_REGION)
            .expect("the capture row must survive the substitution");

        assert_eq!(capture.accelerator, chosen.accelerator);
        assert_eq!(capture.keys, chosen.keys);
        assert_eq!(
            published.len(),
            REGISTRY.len(),
            "the substitution dropped or invented a row"
        );

        // Everything that is NOT the combination stays the registry's: an entry
        // is more than its keys, and a row that lost its description key would
        // draw a blank line in the help page.
        let entry = entry(CAPTURE_REGION).expect("the registry must hold the capture entry");
        assert_eq!(capture.description_key, entry.description_key);
        assert_eq!(capture.category, entry.category);
    }

    #[test]
    fn the_veil_window_cannot_rebind_the_capture_shortcut() {
        // The Rust half of the guard, held at the only place a test can reach
        // it: constructing a `Webview` needs a running event loop, a label does
        // not. The ACL says the same thing a second time, and `ipc.rs` puts THAT
        // question to the shipped `RuntimeAuthority`.
        let refusal = ipc::ensure_from(
            crate::veil::VEIL_WINDOW_LABEL,
            ipc::MAIN_WINDOW_LABEL,
            "set_capture_shortcut",
        )
        .expect_err("the veil must not be able to rebind the shortcut that raised it");

        assert!(refusal.contains("set_capture_shortcut"), "{refusal}");
        assert!(
            refusal.contains(crate::veil::VEIL_WINDOW_LABEL),
            "{refusal}"
        );
        assert!(refusal.contains(ipc::MAIN_WINDOW_LABEL), "{refusal}");
        assert!(
            refusal.contains("REFUSED"),
            "the line must say the call did not happen: {refusal}"
        );
        assert!(
            ipc::ensure_from(
                ipc::MAIN_WINDOW_LABEL,
                ipc::MAIN_WINDOW_LABEL,
                "set_capture_shortcut"
            )
            .is_ok(),
            "the settings screen is in the main window; refusing it there would leave the \
             recorder as mute as it was before"
        );
    }

    // --- what the launcher is told while `setup` is still running ------------

    #[test]
    fn an_undecided_status_is_answered_and_not_refused() {
        // THE defect of 7 September 2026, in the one place a test can reach it.
        //
        // On the installed binary the launcher showed a red "the shortcut
        // registry could not be read" while the shortcut worked: it had asked
        // during `setup`, found no managed state, and got an error. The state is
        // managed from the first instruction of `setup` now, so what arrives
        // here for that launch is `Starting` - and `Starting` is an ANSWER.
        //
        // Held on every variant rather than on that one, because the rule is
        // general: this command hands over what is managed, whatever it is. A
        // filter added for a state the frontend "does not know about yet" is
        // exactly how a launch that is merely starting becomes a launch that
        // failed.
        let combination = accept("Ctrl+Shift+Digit2").expect("the capture combination is valid");

        for status in [
            ShortcutStatus::Starting,
            ShortcutStatus::Accepted {
                accelerator: combination.accelerator.clone(),
            },
            ShortcutStatus::RefusedBySystem {
                accelerator: combination.accelerator,
                reason: "HotKey already registered".to_owned(),
            },
            ShortcutStatus::NotAttempted {
                reason: "the global-shortcut plugin failed to load".to_owned(),
            },
        ] {
            assert_eq!(
                status_answer(Some(status.clone())),
                Ok(status.clone()),
                "{status:?} is managed and this command refused it or changed it. The launcher \
                 draws what comes back from here; anything but the state itself is a screen \
                 saying something that did not happen"
            );
        }
    }

    #[test]
    fn a_status_that_is_not_managed_at_all_is_still_an_error_and_no_longer_blames_a_race() {
        // The path that is DELIBERATELY kept. `unwrap_or(Starting)` here would
        // have been shorter and would have turned a `setup` that never ran - a
        // real defect, and one nothing else in the process reports - into a
        // launcher waiting politely for ever.
        //
        // The wording is the part that changed. It used to describe the race
        // above, which was wrong twice over: it was the ordinary case, and it is
        // no longer reachable that way at all.
        let refusal =
            status_answer(None).expect_err("a status nothing manages must not pass for a status");

        assert!(
            refusal.contains("never ran"),
            "the message must name what is now the ONLY cause - a `setup` that did not run - \
             rather than the startup race it used to describe: {refusal}"
        );
        assert!(
            refusal.contains("src-tauri/src/lib.rs"),
            "the message must name the file that manages the state, or nobody can act on it: \
             {refusal}"
        );
        // Case-insensitive, and deliberately: the message shouts FIRST, and a
        // test that pinned the capitalisation would be asking about typography
        // rather than about what the sentence says.
        assert!(
            refusal.to_lowercase().contains("first instruction"),
            "the message must say WHEN the state is managed. That is what tells the reader this \
             is not a call that came too early: {refusal}"
        );
    }

    #[test]
    fn a_launch_that_offered_nothing_publishes_the_registrys_own_row() {
        // The other row of the same rule. When nothing was ever offered to the
        // system there is no combination to substitute, and the table this
        // application SHIPS with is then the only honest thing to show.
        let published = rows(None);
        let capture = published
            .iter()
            .find(|row| row.id == CAPTURE_REGION)
            .expect("the capture row must exist whatever happened at startup");
        let entry = entry(CAPTURE_REGION).expect("the registry must hold the capture entry");

        assert_eq!(capture.accelerator, entry.accelerator);
        assert_eq!(capture.keys, entry.keys);
    }
}
