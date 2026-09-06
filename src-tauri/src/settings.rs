//! Where the user's choice of capture combination is kept between runs.
//!
//! # One file, written and read by THIS crate, with nothing bought for it
//!
//! Decided by Thierry on 5 September 2026: a plain JSON file, no plugin, no new
//! crate. The whole of what is stored is one string, and this crate holds no
//! JSON parser - `ipc.rs` already declined to take on `serde_json` to look for a
//! single key, and the same trade decides this one. So the reader and the writer
//! below are string handling, and every way they can be wrong is a test in this
//! file.
//!
//! What that costs, stated rather than discovered: this reader understands the
//! shape THIS writer produces and refuses everything else. It does not decode
//! escapes, it does not walk nested objects, and it does not care what else the
//! file holds. A hand-edited file with a `\"` in it is reported as unusable
//! instead of being guessed at - see [`parse`].
//!
//! # A file that is missing, empty or damaged MUST NOT stop the application
//!
//! That is the rule the three [`SettingsRead`] answers exist for. None of them
//! is an error the caller has to handle: `shortcut::startup_choice` turns each
//! into a combination to offer the system, plus - for the two that are not
//! ordinary - one line for the terminal. A settings file is a convenience, and
//! an application that refused to start because of one would be trading its
//! whole purpose for a preference.
//!
//! # Nothing here is reachable from a webview
//!
//! No command is declared in this module. `read` runs in `setup`, and `write` is
//! called from `shortcut::change_capture_shortcut`, behind the
//! `set_capture_shortcut` command - which carries its own permission and its own
//! `ipc::ensure_from` check, in `shortcuts.rs`. The path is chosen by Tauri from
//! the bundle identifier and is never taken from the page: what crosses the IPC
//! frontier is an accelerator, and it is validated by `shortcuts::accept` before
//! it reaches [`write`].

use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use tauri::{AppHandle, Manager};

/// The file this application reads and writes, inside its configuration folder.
///
/// On Windows that folder is `%APPDATA%\<bundle identifier>`, which for this
/// application is `dev.thierryvm.cliche` - the `identifier` of
/// `tauri.conf.json`. The full path is never written here: it is asked of
/// Tauri, in [`path`], because the same code has to be right on a machine whose
/// profile lives somewhere else.
pub const SETTINGS_FILE: &str = "settings.json";

/// The one key this application stores. Named in camelCase to match the wire
/// shape everything else in this project crosses IPC with.
const CAPTURE_SHORTCUT_KEY: &str = "captureShortcut";

/// What a read of the settings file came back with.
///
/// Three answers and not two, for the reason `ShortcutStatus` has three: the
/// difference decides what the TERMINAL is told. A file that is not there is an
/// ordinary first launch and deserves no line at all; a file that is there and
/// cannot be used is worth a sentence naming what is wrong with it, or the user
/// would set a shortcut, restart, and find the old one back with nothing said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsRead {
    /// No file on disk. A first launch, or a user who never changed anything.
    Absent,
    /// A combination was read out of the file. NOT yet validated: whether it is
    /// one this application can register is `shortcuts::accept`'s answer.
    Chosen(String),
    /// There is something on disk and it cannot be used, with what is wrong.
    Unusable(String),
}

/// Reads the stored combination out of the file's text.
///
/// # What it accepts, exactly
///
/// `"captureShortcut"`, optional layout, `:`, optional layout, then a double
/// quoted run of characters holding no backslash. That is the shape [`render`]
/// produces and it is the only shape accepted - a reader that guessed at
/// anything wider would be inventing a parser nobody asked for, on data this
/// application wrote itself.
///
/// A backslash is refused rather than decoded, and that is deliberate: nothing
/// here ever writes one, so meeting one means the file was edited by hand or by
/// something else. Reporting it beats guessing which escape was meant and
/// handing the operating system a combination the user never chose.
///
/// KNOWN BLIND SPOT, said so nobody trusts this past its reach: the key is
/// found by substring, so a file whose FIRST occurrence of `"captureShortcut"`
/// sits inside some other value would be read from there. Nothing this
/// application writes has a second key at all, and the whole answer is
/// validated again by `shortcuts::accept` before it is offered to Windows.
pub fn parse(text: &str) -> Result<String, String> {
    let needle = format!("\"{CAPTURE_SHORTCUT_KEY}\"");

    let Some(at) = text.find(&needle) else {
        return Err(format!(
            "it holds no `{CAPTURE_SHORTCUT_KEY}` key, so there is no combination in it to read"
        ));
    };

    let after_key = text[at + needle.len()..].trim_start();
    let Some(after_colon) = after_key.strip_prefix(':') else {
        return Err(format!(
            "`{CAPTURE_SHORTCUT_KEY}` is not followed by `:`, so this is not the file this \
             application writes"
        ));
    };

    let Some(inside) = after_colon.trim_start().strip_prefix('"') else {
        return Err(format!(
            "the value of `{CAPTURE_SHORTCUT_KEY}` is not a quoted string"
        ));
    };

    let Some(end) = inside.find('"') else {
        return Err(format!(
            "the value of `{CAPTURE_SHORTCUT_KEY}` opens a quote that is never closed"
        ));
    };

    let value = &inside[..end];

    if value.contains('\\') {
        return Err(format!(
            "the value of `{CAPTURE_SHORTCUT_KEY}` holds a backslash. This reader does not decode \
             escapes and never writes one, so the file was edited by something else"
        ));
    }
    if value.is_empty() {
        return Err(format!(
            "`{CAPTURE_SHORTCUT_KEY}` is empty, which names no combination at all"
        ));
    }

    Ok(value.to_owned())
}

/// The text written to disk for one combination.
///
/// Refuses a quote, a backslash or a line break rather than escaping them. This
/// is not a general JSON writer and does not pretend to be one: what it is ever
/// handed is a canonical accelerator from `shortcuts::accept`, made of ASCII
/// letters, digits and `+`. The guard is here so that the day something else
/// calls this, it fails loudly instead of producing a file its own reader would
/// then report as damaged.
pub fn render(accelerator: &str) -> Result<String, String> {
    if accelerator.contains(['"', '\\', '\n', '\r']) {
        return Err(format!(
            "`{accelerator}` holds a quote, a backslash or a line break. This writer escapes \
             nothing, and an accelerator never needs it"
        ));
    }

    Ok(format!(
        "{{\n  \"{CAPTURE_SHORTCUT_KEY}\": \"{accelerator}\"\n}}\n"
    ))
}

/// Where the settings file lives, as Tauri resolves it.
///
/// Asked of `app.path()` rather than composed here: the configuration folder is
/// the platform's answer, not a constant, and the one thing this application may
/// state about it is the file NAME.
pub fn path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join(SETTINGS_FILE))
        .map_err(|error| {
            format!("the application configuration directory could not be resolved: {error}")
        })
}

/// Reads the settings file, turning every failure into an answer.
///
/// Never returns an error, on purpose: see this module's header. A path that
/// cannot even be resolved is reported as [`SettingsRead::Unusable`] rather
/// than as something apart, because the consequence is identical - the registry
/// combination is used and the terminal is told why.
pub fn read(app: &AppHandle) -> SettingsRead {
    let path = match path(app) {
        Ok(path) => path,
        Err(reason) => return SettingsRead::Unusable(reason),
    };

    match fs::read_to_string(&path) {
        Ok(text) => match parse(&text) {
            Ok(value) => SettingsRead::Chosen(value),
            Err(why) => SettingsRead::Unusable(format!("{} is unusable: {why}", path.display())),
        },
        Err(error) if error.kind() == ErrorKind::NotFound => SettingsRead::Absent,
        Err(error) => {
            SettingsRead::Unusable(format!("{} could not be read: {error}", path.display()))
        }
    }
}

/// Writes one combination down, creating the configuration folder if needed.
///
/// Returns the path it wrote to, so the caller can name it on the terminal: a
/// settings file nobody can find is a settings file nobody can delete when it
/// goes wrong.
///
/// The file is REPLACED whole. There is one key in it and this application is
/// the only thing that writes it, so a merge would be reading back a document
/// only to write the same document out again.
pub fn write(app: &AppHandle, accelerator: &str) -> Result<PathBuf, String> {
    let path = path(app)?;
    let body = render(accelerator)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("{} could not be created: {error}", parent.display()))?;
    }

    fs::write(&path, body)
        .map_err(|error| format!("{} could not be written: {error}", path.display()))?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The combination the registry states today, in the plugin's syntax.
    const SAMPLE: &str = "Ctrl+Shift+Digit2";

    #[test]
    fn what_this_module_writes_is_what_it_reads_back() {
        // THE round trip. Written and read by two functions in this file, so a
        // change to either that leaves the other behind fails here rather than
        // on the next launch, silently, as a shortcut that reverted itself.
        let text = render(SAMPLE).expect("a canonical accelerator must be writable");

        assert_eq!(
            parse(&text).as_deref(),
            Ok(SAMPLE),
            "the reader cannot read the writer's own output: {text:?}"
        );
    }

    #[test]
    fn the_written_file_names_the_key_and_the_combination() {
        // Held on the TEXT and not only on the round trip: this file is meant to
        // be opened and deleted by a human when a shortcut goes wrong, so what
        // is in it has to be readable.
        let text = render(SAMPLE).expect("a canonical accelerator must be writable");

        assert!(text.contains(CAPTURE_SHORTCUT_KEY), "{text}");
        assert!(text.contains(SAMPLE), "{text}");
        assert!(
            text.starts_with('{') && text.trim_end().ends_with('}'),
            "the file must be a JSON object, or nothing else will ever read it: {text}"
        );
    }

    #[test]
    fn a_combination_that_could_not_be_written_honestly_is_refused() {
        // The writer escapes nothing. Without this guard it would happily
        // produce a file its own reader then reports as damaged - a corruption
        // this application would have caused itself.
        for hostile in ["Ctrl+\"", "Ctrl+\\", "Ctrl+A\nShift"] {
            assert!(
                render(hostile).is_err(),
                "`{hostile}` was written out unescaped, which produces a file this module cannot \
                 read back"
            );
        }
        assert!(
            render(SAMPLE).is_ok(),
            "the guard above must not refuse an ordinary accelerator, or nothing could ever be \
             saved"
        );
    }

    #[test]
    fn a_damaged_file_is_reported_and_never_guessed_at() {
        // THE case this parser exists for, one row per way a file can be wrong.
        // Every one of them must come back as an error naming the key, because
        // the line the terminal prints is built out of it.
        for (text, what) in [
            ("", "an empty file"),
            ("{}", "an object with nothing in it"),
            ("not json at all", "prose"),
            ("{\"captureShortcut\"}", "a key with no colon"),
            ("{\"captureShortcut\": 42}", "a value that is not a string"),
            (
                "{\"captureShortcut\": \"Ctrl+Shift",
                "an unterminated string",
            ),
            ("{\"captureShortcut\": \"\"}", "an empty combination"),
            (
                "{\"captureShortcut\": \"Ctrl+\\\"\"}",
                "a value holding an escape",
            ),
            ("{\"other\": \"Ctrl+Shift+Digit2\"}", "some other key"),
        ] {
            let refusal = parse(text)
                .expect_err(format!("{what} must not be read as a combination: {text:?}").as_str());

            assert!(
                refusal.contains(CAPTURE_SHORTCUT_KEY),
                "the refusal for {what} names nothing a reader could act on: {refusal}"
            );
        }
    }

    #[test]
    fn layout_around_the_value_is_not_part_of_it() {
        // A file this application wrote, then reformatted by an editor, is
        // still that file. Anything stricter would turn a pretty-printer into a
        // lost shortcut.
        for text in [
            "{\"captureShortcut\":\"Ctrl+Shift+Digit2\"}",
            "{\n  \"captureShortcut\" :  \"Ctrl+Shift+Digit2\"\n}",
            "{\"captureShortcut\"\n:\n\"Ctrl+Shift+Digit2\"}",
        ] {
            assert_eq!(
                parse(text).as_deref(),
                Ok(SAMPLE),
                "layout changed the answer for {text:?}"
            );
        }
    }

    #[test]
    fn the_reader_hands_back_the_value_and_not_the_quotes_around_it() {
        // Without this the round trip above could be green over a reader that
        // returns the whole line, and Windows would be offered a combination
        // with a quote in it.
        let value = parse("{\"captureShortcut\": \"Ctrl+Shift+KeyA\"}")
            .expect("a well formed file must be readable");

        assert_eq!(value, "Ctrl+Shift+KeyA");
        assert!(
            !value.contains('"') && !value.contains(':'),
            "the reader kept some of the syntax around the value: {value:?}"
        );
    }
}
