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

## One-time setup — the build fails without this

`src-tauri/icons/` is empty. `tauri-build` refuses to produce the Windows
resource file without `icons/icon.ico`, so **`cargo build` fails until you
generate the icon set** from any squared PNG:

```powershell
work perso -NoCd; pnpm tauri icon path\to\app-icon.png
```

See `src-tauri/icons/README.md` for the exact error and why no placeholder was
committed.

## Commands

```powershell
work perso -NoCd; pnpm install     # dependencies
work perso -NoCd; pnpm tauri dev   # run the app
work perso -NoCd; pnpm typecheck   # tsc --noEmit, strict, zero `any`
work perso -NoCd; pnpm test        # cargo test on the Rust side
work perso -NoCd; pnpm build       # frontend bundle only
```

`pnpm test` currently maps to `cargo test --manifest-path src-tauri/Cargo.toml`:
at this stage the only automated tests are Rust unit tests. A frontend test
runner arrives with lot 2, together with `tauri-driver` for end-to-end runs
(Playwright is not usable here — it drives a browser, not a native window).

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

## Known limits at this stage

- **Multi-monitor and mixed DPI are written correctly but unproven.** The
  development machine has a single 1920×1080 display at 100 %. Anything
  concerning a second screen is untested, not merely untried.
- Diagnostics go to stdout, visible under `pnpm tauri dev`. A release build
  hides its console, so a real logging backend is needed before shipping.
