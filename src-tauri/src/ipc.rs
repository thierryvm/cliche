//! Which window is allowed to call which command.
//!
//! # Tauri's ACL now guards our own commands too
//!
//! Until 4 September 2026 it did not, and that is worth spelling out because the
//! condition that decides it is easy to read the wrong way round. In the
//! vendored `tauri` 2.11.5 source, `src/webview/mod.rs:1823`:
//!
//! ```text
//! if (plugin_command.is_some() || has_app_acl_manifest || !is_local) && invoke.acl.is_none()
//! ```
//!
//! A request is rejected for want of a capability only when one of those three
//! holds. For a call to one of OUR commands, from a local origin, `plugin_command`
//! is `None` - the name carries no `plugin:` prefix - and `is_local` is `true`,
//! since both windows load bundled documents. Everything therefore hangs on the
//! middle term.
//!
//! `has_app_acl_manifest` is the `__app-acl__` key being present in the resolved
//! ACL (`tauri-utils` 2.9.3, `src/acl/mod.rs:348` for the lookup and `:50` for
//! the constant), and `tauri-build` inserts that key exactly when the
//! application declares at least one permission of its own (`tauri-build` 2.6.3,
//! `src/acl.rs:408-413`). This application now does: `src-tauri/permissions/`
//! holds one permission per command, and `capabilities/default.json` and
//! `capabilities/veil.json` grant each of them to ONE named window. So the
//! condition holds, the check runs, and a `veil_selected` sent from `main` is
//! refused by Tauri before this module is reached.
//!
//! That claim is not taken on trust: the test
//! `the_acl_grants_each_command_to_its_own_window_and_to_no_other` puts the
//! question to the shipped `RuntimeAuthority` itself, and
//! `every_registered_command_is_granted_to_a_window` fails the suite the day a
//! command is added and left ungranted.
//!
//! # Why the Rust guard below stays, all the same
//!
//! Three reasons. The third is the one that would be missed.
//!
//! 1. **Defence in depth.** The capability files and this check state the same
//!    rule in two independent places, and both are under test. A capability
//!    edited by hand - a `windows` array widened to `["main", "veil"]` for a
//!    quick trial, say - does not silently open the veil commands to `main`.
//! 2. **When the guard does fire, its refusal can be acted on.** Subordinate to
//!    the other two, and stated that way on purpose: on an ordinary IPC call
//!    from the wrong window, this guard is never reached at all, because Tauri
//!    rejects and returns first (`webview/mod.rs:1827-1852`). It fires in case 1
//!    (a capability widened by hand) and on the `serve` path of case 3. There,
//!    [`wrong_window_line`] names the command and both windows on the terminal.
//!    Tauri's own refusal carries that detail only in a debug build, since
//!    `resolve_access_message` is `#[cfg(debug_assertions)]`
//!    (`ipc/authority.rs:228`); in release it is the single line
//!    `Command <name> not allowed by ACL` (`webview/mod.rs:1847-1850`), sent to
//!    the PAGE. The veil's page does catch it (`src/veil/main.ts:415`), into
//!    `console.error` - and the veil window holds no `core:webview` permission,
//!    so its devtools cannot be opened. Nobody would ever read that line.
//! 3. **The `cliche:` URI scheme is not IPC, and NO ACL covers it.**
//!    [`crate::veil::serve`] is wired with `register_uri_scheme_protocol` in
//!    `lib.rs`, not with `invoke_handler`. A webview reaches it with an ordinary
//!    `<img>` or `fetch`, which never crosses the `invoke` frontier the condition
//!    above sits on. The label check inside `serve` - the same shape as
//!    [`ensure_from`], applied to `ctx.webview_label()` - is the ONLY thing
//!    between the `main` window and the frozen frame.
//!
//! # What this does NOT protect against
//!
//! Said plainly, because a guard whose limits are unwritten gets trusted past
//! them. Both the capability and this check refuse a call from the WRONG WINDOW.
//! Neither does anything against code executing inside the veil window itself:
//! whatever runs there is, as far as an ACL capability and a label comparison
//! can tell, the veil. What makes that acceptable is that `veil.html` loads no
//! dependency beyond `@tauri-apps/api` (`src/veil/main.ts:106`) - above all not
//! React and its tree, which is a separate entry point (`vite.config.ts`). That
//! is a separate fact from this one.
//!
//! Everything above was read in the vendored `tauri` 2.11.5 source and checked
//! against the generated ACL by the tests below. NONE of it has been confirmed
//! by a run of the application: the rejection at `webview/mod.rs:1827-1852` has
//! been read, never observed.

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
    use std::fs;
    use std::path::Path;
    use tauri::ipc::Origin;

    /// Asks the application's REAL ACL whether a window may call a command.
    ///
    /// No event loop and no window: `generate_context!` embeds the capability
    /// files and the manifests `tauri-build` generated, and
    /// `Context::runtime_authority_mut` (`tauri-2.11.5/src/lib.rs:480` - `pub`,
    /// though `#[doc(hidden)]`) hands back the very `RuntimeAuthority` a real
    /// webview is checked against. So this is the shipped decision, not a
    /// model of it.
    ///
    /// The window and the webview are given the same label because each of this
    /// application's two windows holds a single webview of that name;
    /// `resolve_access` accepts a match on either
    /// (`tauri-2.11.5/src/ipc/authority.rs:459-460`).
    ///
    /// The runtime type is named rather than inferred: `generate_context!`
    /// expands to a generic function whose only mention of `R` is its return
    /// type (`tauri-codegen-2.6.3/src/context.rs:474`), so nothing else would
    /// pin it.
    ///
    /// RISK TAKEN KNOWINGLY: that method is `#[doc(hidden)]` and its own
    /// documentation says "This API is unstable" (`tauri-2.11.5/src/lib.rs:473-482`).
    /// A version bump may remove it or change its signature. The failure would
    /// then be a COMPILATION ERROR of this suite - loud, on the pull request,
    /// in `ci.yml`'s Rust unit tests - and never a test left green for the
    /// wrong reason. Worth saying too: what actually holds 2.11.5 here is
    /// `Cargo.lock`; `Cargo.toml` only asks for `>=2.11.5, <3.0.0`.
    fn granted(context: &mut tauri::Context<tauri::Wry>, command: &str, window: &str) -> bool {
        context
            .runtime_authority_mut()
            .resolve_access(command, window, window, &Origin::Local)
            .is_some()
    }

    /// The commands `lib.rs` actually registers, read from its own source.
    ///
    /// Deliberately not a list kept here: a hand-kept list is exactly what
    /// [`every_registered_command_is_granted_to_a_window`] exists to make
    /// unnecessary. The reading is literal - what sits between
    /// `generate_handler![` and the next `]`, one entry per comma, last path
    /// segment kept - and the test checks the shape of what came out before it
    /// trusts it.
    fn commands_registered_in_lib() -> Vec<String> {
        let source = include_str!("lib.rs");

        let Some((_, after)) = source.split_once("generate_handler![") else {
            return Vec::new();
        };
        let Some((block, _)) = after.split_once(']') else {
            return Vec::new();
        };

        block
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(|entry| entry.rsplit("::").next().unwrap_or(entry).to_owned())
            .collect()
    }

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
        //
        // `veil_decoded` joined the list on 4 September 2026 and is the second
        // worst of the five to leave open: it is what makes the veil window
        // VISIBLE. Called from `main` it would raise a full-screen, always-on-top
        // sheet over the user's desktop with no capture behind it - and, being
        // the only route to `show()` outside the fallback timer, with nothing
        // else to take it back down but Escape.
        //
        // `veil_ready` joined it later the same day and is the least dangerous
        // of the five: it prints one line and touches nothing. It is guarded all
        // the same, because that line is the whole evidence the cold-start
        // diagnosis rests on, and evidence any window may write into the report
        // is not evidence.
        for command in [
            "veil_painted",
            "veil_selected",
            "veil_dismissed",
            "veil_decoded",
            "veil_ready",
        ] {
            let refusal = ensure_from(MAIN_WINDOW_LABEL, VEIL_WINDOW_LABEL, command)
                .expect_err("a call from `main` to a veil command must be refused");

            assert!(refusal.contains(command), "{refusal}");
            assert!(refusal.contains("main"), "{refusal}");
            assert!(refusal.contains("veil"), "{refusal}");
            assert!(
                refusal.contains("REFUSED"),
                "the line must say the call did not happen, not merely that it was \
                 unusual: {refusal}"
            );
        }
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

    #[test]
    fn the_acl_grants_each_command_to_its_own_window_and_to_no_other() {
        // THE matrix, taken from the shipped ACL rather than from the files it
        // was built out of.
        //
        // It also proves the application manifest is armed at all, and that is
        // the reason to read this test first. `veil_selected` carries no
        // `plugin:` prefix, so it can only ever be granted by an APPLICATION
        // permission - the resolver files those under the `__app-acl__` key
        // (`tauri-utils-2.9.3/src/acl/resolved.rs:131`), and `has_app_manifest`
        // is that key being present (`tauri-utils-2.9.3/src/acl/mod.rs:348`,
        // constant at `:50`). If the assertion below holds, the key is there,
        // so `has_app_acl_manifest` is true in
        // `tauri-2.11.5/src/webview/mod.rs:1823` and the check now runs on this
        // crate's own commands.
        let mut context = tauri::generate_context!();

        for command in [
            "veil_painted",
            "veil_selected",
            "veil_dismissed",
            "veil_decoded",
            "veil_ready",
        ] {
            assert!(
                granted(&mut context, command, VEIL_WINDOW_LABEL),
                "`{command}` must be granted to `{VEIL_WINDOW_LABEL}`; without it the veil can \
                 no longer end a capture and the application stops working - except for \
                 `veil_ready`, which is a diagnostic: ungranted, it costs the cold-start \
                 evidence and nothing else"
            );
            assert!(
                !granted(&mut context, command, MAIN_WINDOW_LABEL),
                "`{command}` must NOT be granted to `{MAIN_WINDOW_LABEL}`: that window runs \
                 React and its dependency tree, and these are the calls that reach the clipboard, \
                 that make a full-screen always-on-top window appear, and - for `veil_ready` - \
                 that write the lines a cold-start diagnosis is read from"
            );
        }

        assert!(
            granted(&mut context, "describe_displays", MAIN_WINDOW_LABEL),
            "`describe_displays` is invoked from src/displays.ts, in the `{MAIN_WINDOW_LABEL}` \
             window"
        );
        assert!(
            !granted(&mut context, "describe_displays", VEIL_WINDOW_LABEL),
            "the veil never asks for the monitor list; granting it would widen the window that \
             is up while the screen is frozen"
        );

        // Without this pair the whole test could be green because the resolver
        // says yes to everything.
        for window in [MAIN_WINDOW_LABEL, VEIL_WINDOW_LABEL] {
            assert!(
                !granted(&mut context, "no_such_command", window),
                "the resolver said yes to a command that does not exist, from `{window}`: it is \
                 the instrument that is broken, and every other assertion here is worthless"
            );
        }
    }

    #[test]
    fn every_registered_command_is_granted_to_a_window() {
        // The net for the day somebody adds a command. Since this application
        // declares an ACL manifest, a command nobody granted is refused at
        // runtime by a generic message, in a release build, on a machine that
        // is not this one. Here it is a red `cargo test` naming the command.
        //
        // WHAT THIS NET DOES NOT CATCH, so that nobody trusts it past its
        // reach: a new WINDOW. It asks whether each command is granted to
        // `main` or to `veil`, the only two labels this application creates. A
        // third `WebviewWindowBuilder` would see every one of its `invoke`
        // calls refused, and this test would stay green throughout.
        let commands = commands_registered_in_lib();

        assert!(
            !commands.is_empty(),
            "no command was read out of `generate_handler![` in lib.rs. This PARSER is what is \
             broken, not the application - and a parser that finds nothing would keep this test \
             green for ever, whatever the application registers"
        );
        for command in &commands {
            assert!(
                !command.is_empty()
                    && command
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric() || character == '_'),
                "`{command}` is not a command name, so the parser above read something other \
                 than the handler list: it is the parser that needs fixing, not the application"
            );
        }

        let mut context = tauri::generate_context!();

        for command in &commands {
            assert!(
                granted(&mut context, command, MAIN_WINDOW_LABEL)
                    || granted(&mut context, command, VEIL_WINDOW_LABEL),
                "`{command}` is registered in `generate_handler!` and no capability grants it, \
                 to `{MAIN_WINDOW_LABEL}` or to `{VEIL_WINDOW_LABEL}`. Every call to it would be \
                 REFUSED at runtime. Add a permission `allow-{}` under src-tauri/permissions/ \
                 and name it in the capability file of the window that calls it.",
                command.replace('_', "-")
            );
        }
    }

    #[test]
    fn the_generated_manifest_carries_the_application_key() {
        // Small and direct, next to the matrix above which proves the same
        // thing by its effect: `has_app_manifest` is a plain key lookup for
        // `APP_ACL_KEY` (`tauri-utils-2.9.3/src/acl/mod.rs:348` and `:50`).
        //
        // A substring search, not a parse: this crate does not depend on
        // `serde_json`, and taking on a dependency to look for one key is not a
        // trade worth making. The blind spot is that the key would also be
        // "found" inside a string value - nothing writes one, and the test above
        // is what actually holds the claim.
        let manifests = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("gen")
            .join("schemas")
            .join("acl-manifests.json");

        let text = fs::read_to_string(&manifests)
            .unwrap_or_else(|error| panic!("{} must be readable: {error}", manifests.display()));

        assert!(
            text.contains("__app-acl__"),
            "{} carries no `__app-acl__` key, so tauri-build generated no application manifest \
             and NO ACL check runs on this crate's own commands. src-tauri/permissions/ must \
             hold at least one permission file (tauri-build-2.6.3/src/acl.rs:408-413).",
            manifests.display()
        );
    }
}
