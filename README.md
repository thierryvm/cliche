# Cliché

[![CI](https://github.com/thierryvm/cliche/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/thierryvm/cliche/actions/workflows/ci.yml)
[![Version](https://img.shields.io/github/package-json/v/thierryvm/cliche?label=version)](https://github.com/thierryvm/cliche/blob/main/package.json)
[![Platform](https://img.shields.io/badge/platform-Windows%2011-0b7285)](#prerequisites)

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
pnpm test        # version coherence check + cargo test
pnpm tauri build # release build + NSIS installer
pnpm build       # frontend bundle only
```

`pnpm test` runs `scripts/check-version.mjs`, then
`cargo test --manifest-path src-tauri/Cargo.toml`. At this stage the automated
tests are Rust unit tests plus that coherence check — which fails the build if
`package.json`, `tauri.conf.json` and `Cargo.toml` ever disagree on the version
number. A frontend test runner arrives with the shortcut registry, together
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
index.html            frontend entry
vite.config.ts
tsconfig.json         strict, plus noUncheckedIndexedAccess & friends
scripts/
  check-version.mjs   one version number, three files, one check
src/                  React frontend
  displays.ts         typed binding for the describe_displays command
src-tauri/
  build.rs            embeds the custom Windows manifest
  windows-app-manifest.xml   per-monitor DPI aware v2 — read the comments
  tauri.conf.json
  capabilities/       permission grants, empty of plugins on purpose
  src/
    main.rs           binary entry point
    lib.rs            builder, startup logging
    displays.rs       display enumeration + unit tests
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

**Not chosen yet.** Until a `LICENSE` file is added, default copyright applies:
all rights reserved. GitHub's Terms of Service still let anyone view and fork a
public repository, but not reproduce, distribute, or build derivative works
from it. (docs.github.com, *Licensing a repository*, consulted 2 September
2026.)

This is a pending decision, not a permanent stance. Adding a license later is a
one-file change; taking one back is not, because a version published under a
permissive license stays permissively licensed forever.

## Contributing

Not open to contributions at this stage — the license question above is exactly
why. Issues reporting a bug or a Windows configuration where something breaks
are welcome and useful.

## Known limits at this stage

- **Multi-monitor and mixed DPI are written correctly but unproven.** The
  development machine has a single 1920×1080 display at 100 %. Anything
  concerning a second screen is untested, not merely untried.
- **Windows 10 is untested.** The code targets APIs available there, but no
  Windows 10 machine has run this build.
- **The installer is unsigned.** SmartScreen warns; there is no certificate.
- Diagnostics go to stdout, visible under `pnpm tauri dev`. A release build
  hides its console, so a real logging backend is needed before shipping.
