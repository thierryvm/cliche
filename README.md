# Cliché

[![CI](https://github.com/thierryvm/cliche/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/thierryvm/cliche/actions/workflows/ci.yml)
[![Version](https://img.shields.io/github/package-json/v/thierryvm/cliche?label=version)](https://github.com/thierryvm/cliche/blob/main/package.json)
[![Platform](https://img.shields.io/badge/platform-Windows%2011-0b7285)](#prerequisites)
[![License](https://img.shields.io/badge/license-PolyForm%20Noncommercial%201.0.0-7a5200)](LICENSE.md)

Local screenshot utility for Windows. Capture a region, a window, a whole
screen, or a web page longer than the screen; annotate it; copy or save it;
find it again later. Keyboard-driven, and styled like frosted glass.

**It talks to no network.** No account, no share link, no telemetry, no update
check. There is no HTTP client in the dependency tree, and the webview CSP
limits `connect-src` to Tauri's own IPC channel.

> ### Status: early — not usable as a screenshot tool yet
>
> The window opens and enumerates displays. Nothing captures anything so far.
> This section is a fact table, not a roadmap in disguise: it says what has
> been *run*, not what has been planned.

| Capability | State |
| --- | --- |
| Window starts, per-monitor DPI aware v2 | ✅ done, verified |
| Display enumeration (physical pixels, scale) | ✅ done, verified on 1 display |
| Design system: tokens, components, showcase at `#/systeme` | ✅ done, looked at in a browser — not yet in WebView2 |
| Global shortcut → frozen overlay → region → clipboard | ⏳ next |
| Window and full-screen capture | ⏳ planned |
| Annotation (arrow, text, **destructive** blur) | ⏳ planned |
| Local library, save, search | ⏳ planned |
| Scrolling web page capture | ⏳ planned |
| In-app help, derived from the shortcut registry | ⏳ planned |

Plan and reasoning live in [`docs/`](docs/): [`PRD.md`](docs/PRD.md) for the
product, [`STACK.md`](docs/STACK.md) for why Tauri, [`PLAN.md`](docs/PLAN.md)
for the implementation order. They are written in French — the working
language of this project. Everything else, including code and commits, is
English.

## Screenshots

None yet, on purpose: there is nothing worth showing until the capture overlay
lands. A screenshot of a screenshot tool that cannot take screenshots would be
a picture of an empty window.

## Install

### From a release

**No release published yet.** The first one gets cut when the capture path
works end to end; the procedure and what a release contains are written in
[`docs/RELEASES.md`](docs/RELEASES.md).

There is **no auto-updater**, and that is a decision rather than an omission —
see [Updates](#updates).

### From source

Requires the [prerequisites](#prerequisites) below.

```powershell
git clone https://github.com/thierryvm/cliche.git
cd cliche
pnpm install
pnpm tauri build
```

The NSIS installer lands in `src-tauri/target/release/bundle/nsis/`. It is
unsigned: Windows SmartScreen will warn on first run. Signing costs a
certificate, and this project has no budget line for one yet.

## Prerequisites

- **Windows 11** (Windows 10 should work; untested — see [Known limits](#known-limits-at-this-stage))
- **Rust**, MSVC toolchain
- **Node** 22 or later
- **pnpm** 11 — never npm; the lockfile is pnpm's
- **WebView2** — already present on Windows 11

## Commands

```powershell
pnpm install     # dependencies
pnpm tauri dev   # run the app
pnpm typecheck   # tsc --noEmit, strict, zero `any`
pnpm test        # version check + contrast check + cargo test
pnpm tauri build # release build + NSIS installer
pnpm build       # frontend bundle only
```

`pnpm test` runs three things in order: `scripts/check-version.mjs`, then
`scripts/check-contrast.mjs`, then `cargo test --manifest-path
src-tauri/Cargo.toml`. The two scripts check different things, by the same
method: recompute from the source, then fail rather than warn.

`check-version.mjs` fails the build if `package.json`, `tauri.conf.json` and
`Cargo.toml` ever disagree on the version number — one fact copied by hand into
three files. `check-contrast.mjs` recomputes 139 colour pairings from
`src/design/tokens.css` and fails if any falls under the WCAG ratio its job
requires; the ratios quoted in that file's comments are quoted *from* this
computation, which is what stops them becoming decoration. Both are
dependency-free and are separate CI steps too. A frontend test runner arrives with the shortcut registry, together
with `tauri-driver` for end-to-end runs (Playwright is not usable here: it
drives a browser, not a native window).

> **Maintainer note.** On the author's machine every outgoing command is
> prefixed with a DevContext identity guard (`work perso -NoCd;`). It isolates
> git, `gh`, Vercel and Supabase credentials per project root. It is **not**
> needed to build this project, and it is deliberately absent from the commands
> above.

### Why `pnpm-workspace.yaml` says `allowBuilds: esbuild: true`

pnpm 11 refuses to run a dependency's install script until it is explicitly
allowed, and **it writes the block itself** with the placeholder
`set this to true or false`. That placeholder is neither, so the build stays
ignored and `pnpm install` exits 1 — which in turn fails `pnpm tauri build`,
since the Tauri CLI runs an install check first. Deleting the block does not
help: pnpm puts it back. It has to be answered. Measured 2 September 2026.

## Measuring the capture path

The question the skeleton exists to answer: from the entry of the shortcut
handler to a full-screen veil showing the **frozen** screen — is the median
under 150 ms?

Two transports are compiled in and chosen at **startup**, by environment
variable, so switching between them is a restart and not a rebuild:

| `CLICHE_TRANSPORT` | Route | Encoding cost |
| --- | --- | --- |
| `bmp` *(default)* | Rust serves a BMP from memory on the `cliche:` scheme, the page loads it with an `<img>` | a 66-byte header and one `memcpy` |
| `png` | PNG, base64, pushed in as a `data:` URL | **69.6 ms** median, measured 3 September 2026 — 46 % of the budget |

```powershell
# Twenty automated runs, no keyboard, transport A then transport B.
$env:CLICHE_BENCH = '20'; $env:CLICHE_TRANSPORT = 'bmp'; pnpm tauri dev
$env:CLICHE_BENCH = '20'; $env:CLICHE_TRANSPORT = 'png'; pnpm tauri dev

# Or press Ctrl+Shift+2 twenty times; the report prints itself either way.
```

`CLICHE_BENCH` calls **the same function** the shortcut calls, which measures
the same interval — `t0` is the entry of the handler, so everything the
keyboard does before that is already outside every figure printed. The three
ways it is nevertheless gentler than a real press are written out above
`veil::spawn_bench`, and they all flatter the result. Treat it as a floor and
the keyboard as the arbiter.

Reading the report: `capture` · `transport` · `shown` · `painted`, then a
TOTAL. **`painted` is an approximation with a known error in each direction** —
it carries the acknowledgement's own trip back to Rust (too large) and stops at
a `requestAnimationFrame` callback rather than at compositor presentation (too
small). Both are spelled out at the top of `src-tauri/src/veil.rs`; neither is
hidden behind the word "measured".

### The one CSP entry this needs

`img-src` gains exactly one origin, in both `csp` and `devCsp`:

```
http://cliche.localhost
```

That is where WebView2 serves the custom `cliche:` scheme on Windows — the same
`http://<scheme>.localhost` rule the pre-existing `http://asset.localhost`
entry follows. Nothing else changes: `connect-src` still allows only Tauri's
IPC channel, `script-src` is still `'self'`, and the veil page fetches nothing
of its own. `tauri.conf.json` is strict JSON and cannot carry a comment, so the
reasoning lives in `veil::VEIL_ORIGIN` — with a unit test that pins the exact
string, so the constant and the policy cannot drift apart in silence.

## Stack

| Piece | Version | Note |
| --- | --- | --- |
| Tauri | 2.11.5 | pinned in `src-tauri/Cargo.toml` |
| `tauri-build` | resolved to 2.6.3 | from `tauri-build = "2"` |
| `@tauri-apps/cli` | 2.11.4 | pinned exactly in `package.json` |
| React | 18 | |
| Vite | 6 | dev server on port 1420, `strictPort` |
| Package manager | **pnpm** 11.7.0 | matches the version CI installs |

The icon set is committed (`src-tauri/icons/`), which matters: `tauri-build`
refuses to produce the Windows resource file without `icons/icon.ico`, so
`cargo build` fails on a fresh clone without it. Only the icons the Windows
NSIS bundle references are kept — the Android, iOS and Microsoft Store icons
that `tauri icon` also generates were removed, since this project ships Windows
only and unused binaries in a repository are files nobody re-checks.

To regenerate them from a squared PNG: `pnpm tauri icon ./app-icon.png`.

## Layout

```
index.html            frontend entry — the application window
veil.html             SECOND entry — the full-screen veil, no React, no tokens
vite.config.ts        two rollup inputs, one per entry above
tsconfig.json         strict, plus noUncheckedIndexedAccess & friends
scripts/
  check-version.mjs   one version number, three files, one check
  check-contrast.mjs  139 colour pairings, recomputed from the tokens
src/                  React frontend
  displays.ts         typed binding for the describe_displays command
  App.tsx             home screen, plus a hash switch to the showcase
  design/
    tokens.css        THE source of truth for the visual system
    components.css    buttons, fields, glass, rows, grid — no raw values
    Showcase.tsx      the openable showcase, at #/systeme
  veil/
    main.ts           bare TypeScript: show the frozen frame, acknowledge, Escape
src-tauri/
  build.rs            embeds the custom Windows manifest
  windows-app-manifest.xml   per-monitor DPI aware v2 — read the comments
  tauri.conf.json
  capabilities/       permission grants, empty of plugins on purpose
  src/
    main.rs           binary entry point
    lib.rs            builder, startup logging
    displays.rs       display enumeration + unit tests
    timing.rs         the instrument the whole verdict rests on
    capture.rs        screen grab, PNG and BMP encoders, measured separately
    shortcut.rs       the global shortcut, registered from Rust
    veil.rs           preheated veil window, both transports, the benchmark
```

## DPI awareness

The process is declared **per-monitor DPI aware v2** through a custom
application manifest. This is not decoration: every screenshot coordinate is a
physical pixel, and a process that is not per-monitor aware receives
coordinates that Windows has silently rescaled.

Tauri's own default manifest declares *no* DPI setting at all — only the
Common-Controls dependency. Supplying a custom manifest replaces the default
wholesale, which is why `windows-app-manifest.xml` repeats that dependency
block. The reasoning, and how it relates to `tao`'s runtime
`SetProcessDpiAwarenessContext` call, is written out in that file.

**That manifest must stay pure ASCII.** A single accented character in a
comment is enough to stop the application from starting with
`os error 14001 — side-by-side configuration is incorrect`, and neither
`cargo build` nor `cargo test` reports it. The resource chain transcodes the
file to the ANSI code page, so a UTF-8 `é` (`C3 A9`) becomes a lone `E9`,
which is invalid UTF-8 in a document declared `encoding="UTF-8"`. Windows then
rejects the whole manifest. Measured 2 September 2026.

## Updates

**There is no auto-updater, and that is a decision, not an omission.** The
reasoning — what an update check sends off the machine, what the repetition of
that check builds over time, and what would have to change to add one — is in
[`docs/UPDATES.md`](docs/UPDATES.md).

The short version: an updater only helps with machines you do not touch, and
today there is exactly one machine, touched daily by the person who compiles
it. The trigger for revisiting is written down: **before the first copy handed
to someone else**, not "when there are users" — an updater can only update a
build that already contains it.

## License

[**PolyForm Noncommercial License 1.0.0**](LICENSE.md) — SPDX identifier
`PolyForm-Noncommercial-1.0.0`.

Read it, fork it, change it, build it, run it: all of that is granted, for any
**noncommercial** purpose. Personal use, study, hobby projects, and use by
charities, schools, public research and government bodies are all named as
permitted purposes in the text. Commercial use is not granted here — ask.

It is **not** an OSI-approved open-source license, and that is a deliberate
trade rather than an oversight. The reason is asymmetry: a version published
under a permissive license stays permissively licensed forever, so moving from
this license to MIT later costs one file, while the reverse is impossible. This
one keeps that door open.

## Contributing

Not open to pull requests yet — a noncommercial license makes the terms under
which outside patches would be accepted a question that has not been thought
through. Issues reporting a bug, or a Windows configuration where something
breaks, are welcome and useful.

## Known limits at this stage

- **Multi-monitor and mixed DPI are written correctly but unproven.** The
  development machine has a single 1920×1080 display at 100 %. Anything
  concerning a second screen is untested, not merely untried.
- **Windows 10 is untested.** The code targets APIs available there, but no
  Windows 10 machine has run this build.
- **The installer is unsigned.** SmartScreen warns; there is no certificate.
- Diagnostics go to stdout, visible under `pnpm tauri dev`. A release build
  hides its console, so a real logging backend is needed before shipping.
