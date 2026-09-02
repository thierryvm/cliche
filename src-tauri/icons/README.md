# Icons are missing on purpose — and they block `cargo build`

This directory is empty apart from this note. **The build will fail until it is
filled.** That is not an oversight left silently in place: it is recorded here
because the agent that scaffolded lot 0 (2 September 2026) can only write text
files, and `icon.ico` is binary.

## The exact failure to expect

`tauri-build` refuses to generate the Windows resource file without an icon.
Verified on 2 September 2026 by reading `crates/tauri-build/src/lib.rs` on the
`dev` branch of `tauri-apps/tauri`:

```rust
if window_icon_path.exists() {
  res.set_icon_with_id(&window_icon_path.display().to_string(), "32512");
} else {
  return Err(anyhow!(format!(
    "`{}` not found; required for generating a Windows Resource file during tauri-build",
    window_icon_path.display()
  )));
}
```

So `cargo build --manifest-path src-tauri/Cargo.toml` will stop with:

```
`icons/icon.ico` not found; required for generating a Windows Resource file during tauri-build
```

The default path is `icons/icon.ico`, relative to `src-tauri/`.

## How to fill it

From the repository root, with any squared PNG (1024×1024 with transparency is
the usual source):

```powershell
work perso -NoCd; pnpm tauri icon path\to\app-icon.png
```

`tauri icon [OPTIONS] [INPUT]` defaults its input to `./app-icon.png` and its
output to the `icons` directory next to `tauri.conf.json` — that is this
directory. It generates `icon.ico`, `icon.icns` and the PNG sizes that
`tauri.conf.json` lists under `bundle.icon`.

Cross-checked against <https://v2.tauri.app/reference/cli/> on 2 September 2026.

## Do not "unblock" this with a placeholder

Writing a text file named `icon.ico` makes the path test pass and then fails
later inside the resource compiler, with a message that points nowhere near the
cause. An honest missing file is cheaper than a fake present one.
