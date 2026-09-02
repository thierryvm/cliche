# Cliché

Local screenshot utility for Windows. Capture, annotate, copy, find again.

**It talks to no network.** No account, no share link, no telemetry, no update
check. There is no HTTP client in the dependency tree, and the webview CSP has
`connect-src` limited to Tauri's own IPC channel.

The implementation plan is `docs/PLAN.md` (in French, deliberately — it is the
working language; everything else here is English).

## Stack

| Piece | Version | Note |
| --- | --- | --- |
| Tauri | 2.11.5 | pinned in `src-tauri/Cargo.toml` |
| `tauri-build` | resolved to 2.6.3 | from `tauri-build = "2"` |
| `@tauri-apps/cli` | 2.11.4 | pinned exactly in `package.json` |
| React | 18 | |
| Vite | 6 | dev server on port 1420, `strictPort` |
| Package manager | **pnpm** | never npm |

## Prerequisites

Rust (MSVC toolchain), Node, pnpm, and WebView2 (shipped with Windows 11).

## Setup

`pnpm install` is enough. The icon set is committed (`src-tauri/icons/`), which
matters because `tauri-build` refuses to produce the Windows resource file
without `icons/icon.ico` — without it, `cargo build` fails.

Only the icons the Windows NSIS bundle actually references are kept. The Android,
iOS and Microsoft Store icons that `tauri icon` also generates were removed: this
project ships Windows only, and unused binaries in a repository are files nobody
will ever re-check.

To regenerate them from a squared PNG:

```powershell
work perso -NoCd; pnpm tauri icon .\app-icon.png
```

## Commands

```powershell
work perso -NoCd; pnpm install     # dependencies
work perso -NoCd; pnpm tauri dev   # run the app
work perso -NoCd; pnpm typecheck   # tsc --noEmit, strict, zero `any`
work perso -NoCd; pnpm test        # version coherence + cargo test
work perso -NoCd; pnpm tauri build # release build + NSIS installer
work perso -NoCd; pnpm build       # frontend bundle only
```

`pnpm test` runs `scripts/check-version.mjs` then
`cargo test --manifest-path src-tauri/Cargo.toml`: at this stage the only
automated tests are Rust unit tests plus that coherence check. A frontend test
runner arrives with lot 2, together with `tauri-driver` for end-to-end runs
(Playwright is not usable here — it drives a browser, not a native window).

### Why `pnpm-workspace.yaml` says `allowBuilds: esbuild: true`

pnpm 11 refuses to run a dependency's install script until it is explicitly
allowed, and **it writes the block itself** with the placeholder
`set this to true or false`. That placeholder is neither, so the build stays
ignored and `pnpm install` exits 1 — which in turn fails `pnpm tauri build`,
since the Tauri CLI runs an install check first. Deleting the block does not
help: pnpm puts it back. It has to be answered. Measured 2 September 2026.

The `work perso -NoCd` prefix is the DevContext identity guard; it does not
survive from one shell invocation to the next, so it is repeated on every
outgoing command.

## Layout

```
index.html            frontend entry
vite.config.ts
tsconfig.json         strict, plus noUncheckedIndexedAccess & friends
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
physical pixel, and a process that is not per-monitor aware receives coordinates
that Windows has silently rescaled.

Tauri's own default manifest declares *no* DPI setting at all — only the
Common-Controls dependency. Supplying a custom manifest replaces the default
wholesale, which is why `windows-app-manifest.xml` repeats that dependency
block. The reasoning, and how it relates to `tao`'s runtime
`SetProcessDpiAwarenessContext` call, is written out in that file.

## Updates

**There is no auto-updater, and that is a decision, not an omission.** The
reasoning — what an update check sends off the machine, what the repetition of
that check builds over time, and what would have to change to add one — is in
[`docs/UPDATES.md`](docs/UPDATES.md). The publishing procedure is
[`docs/RELEASES.md`](docs/RELEASES.md).

The short version: an updater only helps with machines you do not touch, and
today there is exactly one machine, touched daily by the person who compiles it.
The trigger for revisiting is written down: **before the first copy handed to
someone else**, not "when there are users" — an updater can only update a build
that already contains it.

## License

None, deliberately. Without a license file, default copyright applies: all
rights reserved. GitHub's Terms of Service still let anyone view and fork a
public repository, but not reproduce, distribute or build derivative works from
it. (docs.github.com, *Licensing a repository*, consulted 2 September 2026.)

This keeps the source readable — which is worth something for a tool that reads
your screen — while leaving the option of selling binaries open. Adding an MIT
or Apache licence later is a one-file change; removing one is not.

## Known limits at this stage

- **Multi-monitor and mixed DPI are written correctly but unproven.** The
  development machine has a single 1920×1080 display at 100 %. Anything
  concerning a second screen is untested, not merely untried.
- Diagnostics go to stdout, visible under `pnpm tauri dev`. A release build
  hides its console, so a real logging backend is needed before shipping.
