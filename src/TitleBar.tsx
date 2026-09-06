/*
 * The window's own chrome, drawn by this application.
 *
 * `decorations` is `false` in tauri.conf.json since the night of 5-6 September
 * 2026, so what follows is not decoration: without it the window has no way to
 * be moved, minimised or closed.
 *
 * # THE ACL IS THE TRAP HERE
 *
 * None of the calls below is a command of this crate. They are commands of
 * Tauri's own `window` plugin, and the ACL gates every `plugin:`-prefixed
 * command whatever else is configured (`tauri-2.11.5/src/webview/mod.rs:1823`).
 * Missing a permission does not fail the build and does not fail `tsc`: the
 * button simply does nothing, in silence, in a release build. The four
 * permissions this file needs are named in `src-tauri/capabilities/default.json`
 * and are held against the shipped RuntimeAuthority by
 * `the_acl_grants_the_main_window_the_core_commands_its_title_bar_calls` in
 * `src-tauri/src/ipc.rs`.
 *
 * # THE DRAG REGION TAKES `deep`, AND THE MAQUETTE'S BARE ATTRIBUTE DOES NOT
 *
 * Read in `tauri-2.11.5/src/window/scripts/drag.js:51-70`. The script walks the
 * composed path of the mousedown and, for a BARE `data-tauri-drag-region`,
 * ends with `return el === composedPath[0]` - the drag starts only when the
 * pressed element IS the one carrying the attribute. `.c-titlebar__title` is
 * `flex: 1 1 auto`, so it covers most of the bar; with the bare attribute of
 * `Showcase.tsx` a press on the application's name would land on the `<p>` and
 * move nothing. `deep` (`:64`) makes the whole subtree a handle, and the
 * controls stay safe without a second attribute because the same walk returns
 * `false` as soon as it meets a `BUTTON` that carries none (`:57-58`) - which
 * is the markup below. That is the same rule `.c-titlebar__controls`'s
 * `app-region: no-drag` states on the CSS side, so the two halves agree.
 *
 * NOT ONE LINE OF THE PARAGRAPH ABOVE HAS BEEN OBSERVED IN A RUNNING WINDOW. It
 * was read in the vendored script. Whether WebView2 honours `-webkit-app-region`
 * at all - the other half, in components.css - is still unknown, and it does not
 * need to be: the attribute is Tauri's own answer and does not depend on it.
 */

import { useEffect, useState } from 'react';

import { getCurrentWindow } from '@tauri-apps/api/window';
import type { Window as TauriWindow } from '@tauri-apps/api/window';
import type { UnlistenFn } from '@tauri-apps/api/event';

import { Glyph, ICON } from './design/Glyph';
import { UI_STRINGS } from './strings';
import { maximiseControl } from './window-controls';

import './design/components.css';

/**
 * Runs something on the window this document is drawn in.
 *
 * `getCurrentWindow()` reads `window.__TAURI_INTERNALS__` and THROWS when the
 * document is opened outside the Tauri webview - which is exactly `pnpm dev` in
 * a plain browser tab, and that tab is how these screens get looked at. Opening
 * the chain with an already-resolved promise turns that throw into a rejection,
 * so it is reported like any other failure instead of tearing the React tree
 * down. Nothing is swallowed: every caller passes a rejection handler.
 */
function onWindow<T>(action: (appWindow: TauriWindow) => Promise<T>): Promise<T> {
  return Promise.resolve().then(() => action(getCurrentWindow()));
}

/**
 * What a failed window call does.
 *
 * The console, and deliberately nothing on screen: these are the window's own
 * controls, and a message explaining that « Réduire » did not work would be a
 * worse interface than the button that did not work. This is also where a
 * missing `core:window:*` permission announces itself, which is the failure
 * this whole file is written around.
 */
function report(what: string): (error: unknown) => void {
  return (error: unknown) => {
    console.error(`[cliche] title bar: ${what} failed`, error);
  };
}

type TitleBarProps = {
  readonly title: string;
  /** Whether the help screen is the one on show. Drawn as `aria-pressed`. */
  readonly helpOpen: boolean;
  /** Asked to swap between the launcher and the help. */
  readonly onToggleHelp: () => void;
  /** Whether the settings screen is the one on show. Drawn as `aria-pressed`. */
  readonly settingsOpen: boolean;
  /** Asked to swap between the launcher and the settings. */
  readonly onToggleSettings: () => void;
};

/**
 * The window's chrome: its name, the way into the help, and the three controls
 * a window has.
 *
 * # THE HELP CONTROL IS A TOGGLE, and that is what gives the screen a way BACK
 *
 * Added 6 September 2026. It is `aria-pressed`, not a link: pressed while the
 * help is up, and pressing it again returns to the launcher. A one-way button
 * would have needed a second control - « retour » - and the design system
 * publishes no wording for one. This shape needs a single word for both
 * directions, and the state is carried by a mechanism the material layer
 * already dresses (`.c-btn[aria-pressed='true']`, the inset accent bar, which
 * `Showcase.tsx` publishes as « outil actif »).
 *
 * It sits inside `.c-titlebar__controls` because that is the one part of the bar
 * that is `app-region: no-drag`; anywhere else the press would move the window.
 * It carries no `data-tauri-drag-region`, which is what stops Tauri's own drag
 * script from treating it as a handle - the same rule the three window controls
 * rely on, read in `tauri-2.11.5/src/window/scripts/drag.js:57-58`.
 *
 * # AND SO IS THE SETTINGS CONTROL, added the same day
 *
 * Same shape, same toggle, and the same reason: `#/reglages` is a screen with no
 * other way in. The two application controls come FIRST and the three window
 * controls last, which is also the tab order - what this window DOES before
 * what is done TO this window - and it leaves the close button at the end,
 * where a hand looking for it already goes.
 */
export default function TitleBar({
  title,
  helpOpen,
  onToggleHelp,
  settingsOpen,
  onToggleSettings,
}: TitleBarProps) {
  // Starts at `false` and is corrected by the first probe below. The starting
  // value is a guess and is treated as one - it is why the effect probes on
  // mount rather than only on the first resize.
  const [maximised, setMaximised] = useState(false);

  useEffect(() => {
    let abandoned = false;
    let stopListening: UnlistenFn | undefined;

    const sync = () => {
      void onWindow((appWindow) => appWindow.isMaximized()).then((value) => {
        if (!abandoned) {
          setMaximised(value);
        }
      }, report('reading the maximised state'));
    };

    // ASKED, NEVER ASSUMED. A window can be maximised before this component
    // mounts - restored by the session, or snapped by the user during startup -
    // and a bar that only listened would draw « Agrandir » over a full screen.
    sync();

    void onWindow((appWindow) => appWindow.onResized(sync)).then((stop) => {
      // The listener may arrive after the effect was torn down: React's
      // StrictMode mounts twice in development, and this promise does not
      // cancel. Dropping it on the floor would leave a handler calling
      // setState on an unmounted tree, once per mount, for the life of the
      // window.
      if (abandoned) {
        stop();
      } else {
        stopListening = stop;
      }
    }, report('listening for window resizes'));

    return () => {
      abandoned = true;
      stopListening?.();
    };
  }, []);

  const control = maximiseControl(maximised);

  return (
    <div className="c-titlebar" data-tauri-drag-region="deep">
      <p className="c-titlebar__title">{title}</p>
      <div className="c-titlebar__controls">
        <button
          type="button"
          className="c-btn c-btn--ghost c-btn--icon c-winbtn"
          aria-label={UI_STRINGS.helpTitle}
          aria-pressed={helpOpen}
          onClick={onToggleHelp}
        >
          <Glyph d={ICON.info} />
        </button>
        <button
          type="button"
          className="c-btn c-btn--ghost c-btn--icon c-winbtn"
          aria-label={UI_STRINGS.settingsTitle}
          aria-pressed={settingsOpen}
          onClick={onToggleSettings}
        >
          <Glyph d={ICON.settings} />
        </button>
        <button
          type="button"
          className="c-btn c-btn--ghost c-btn--icon c-winbtn"
          aria-label={UI_STRINGS.windowMinimize}
          onClick={() => {
            void onWindow((appWindow) => appWindow.minimize()).catch(report('minimising'));
          }}
        >
          <Glyph d={ICON.minimize} />
        </button>
        <button
          type="button"
          className="c-btn c-btn--ghost c-btn--icon c-winbtn"
          aria-label={UI_STRINGS[control.labelKey]}
          onClick={() => {
            // `toggleMaximize` and not `maximize()`/`unmaximize()` picked from
            // our own `maximised`: that flag is one IPC round trip old, and the
            // window is the only thing that knows which of the two it is now.
            void onWindow((appWindow) => appWindow.toggleMaximize()).catch(
              report('toggling maximised'),
            );
          }}
        >
          <Glyph d={ICON[control.icon]} />
        </button>
        <button
          type="button"
          className="c-btn c-btn--ghost c-btn--icon c-winbtn c-winbtn--close"
          aria-label={UI_STRINGS.windowClose}
          onClick={() => {
            void onWindow((appWindow) => appWindow.close()).catch(report('closing'));
          }}
        >
          <Glyph d={ICON.close} />
        </button>
      </div>
    </div>
  );
}
