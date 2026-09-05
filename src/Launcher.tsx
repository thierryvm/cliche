/*
 * The home screen: three capture actions, and what the shortcut is doing.
 *
 * The markup is `LauncherBody` in `src/design/Showcase.tsx`, class for class.
 * Two things differ, and both are written down rather than left to be noticed:
 *
 *   1. The screen name is an `<h1>` and not an `<h3>`. In the showcase this
 *      block sits under that page's own `h1` and `h2`; here it is the first
 *      heading of a window, and a document whose outline starts at level 3 is
 *      a document a screen reader cannot walk. `.c-screen__name` is unchanged,
 *      so nothing about the look moves.
 *   2. The loading keycap holds a non-breaking space instead of the drawn
 *      combination. The showcase may print « Ctrl + Maj + 2 » because it is a
 *      DRAWING; this file may not, because the combination belongs to the Rust
 *      registry (`src-tauri/src/shortcuts.rs`) and is not known before the
 *      round trip that is still in flight. The box is therefore narrower than
 *      the one the maquette reserves - the honest cost of not inventing a
 *      shortcut, and the reasoning is in `src/shortcut-hint.ts`.
 *
 * WHAT THE PRIMARY TILE DOES TODAY: nothing. No command starts a capture from
 * a webview - `veil::perform_capture` is reached only from the global shortcut
 * handler (`src-tauri/src/shortcut.rs`), and `capabilities/default.json` grants
 * this window no capture command because none exists to grant. The tile is
 * drawn in its built state because it IS the built action; wiring it needs a
 * new Rust command, and that is the next lot. Said plainly here because a
 * button that looks pressable and is not is a defect, not a detail.
 */

import { useEffect, useState } from 'react';

import Keys from './Keys';
import { Glyph, ICON } from './design/Glyph';
import { hintFor } from './shortcut-hint';
import type { RegistryRead } from './shortcut-hint';
import { describeShortcuts } from './shortcuts';
import { UI_STRINGS } from './strings';

import './design/components.css';

/** The gap between the launcher's blocks, as the maquette spends it. */
const BLOCK_GAP = { marginBlockStart: 'var(--space-5)' } as const;

/**
 * What the reserved keycap holds while the registry is being read.
 *
 * U+00A0 and not a plain space: `.c-kbd.c-skeleton` paints its text out but
 * still measures it, and an ordinary whitespace child would collapse, leaving
 * a chip with no line box.
 */
const RESERVED_CAP = ' ';

export default function Launcher() {
  const [read, setRead] = useState<RegistryRead>({ status: 'reading' });

  useEffect(() => {
    // StrictMode runs effects twice in development, so `describe_shortcuts` is
    // asked twice there. That is the dev double-render, not a bug.
    let abandoned = false;

    describeShortcuts().then(
      (entries) => {
        if (!abandoned) {
          setRead({ status: 'read', entries });
        }
      },
      (error: unknown) => {
        if (!abandoned) {
          // The message is not shown: what the user needs to know is that the
          // shortcut cannot be announced, and `.c-note--danger` says exactly
          // that. The detail belongs in the console, where a refused ACL or a
          // wrong-window guard can actually be acted on.
          console.error('[cliche] launcher: the shortcut registry could not be read', error);
          setRead({ status: 'unreadable' });
        }
      },
    );

    return () => {
      abandoned = true;
    };
  }, []);

  const hint = hintFor(read);

  return (
    <>
      <h1 className="c-screen__name">{UI_STRINGS.capture}</h1>

      <div
        className="c-launch"
        role="group"
        aria-label={UI_STRINGS.captureActions}
        style={BLOCK_GAP}
      >
        <button type="button" className="c-launch__item c-launch__item--primary">
          <Glyph d={ICON.capture} />
          <span className="c-launch__name">{UI_STRINGS.captureRegion}</span>
        </button>

        {/* aria-disabled and not `disabled`: it announces, it does not block,
            and keeping the tile focusable is how a keyboard user learns the two
            actions exist at all (PRD A1). There is no handler to stop the click
            in, because there is nothing for it to reach. */}
        <button type="button" className="c-launch__item c-launch__item--soon" aria-disabled="true">
          <Glyph d={ICON.window} />
          <span className="c-launch__name">{UI_STRINGS.captureWindow}</span>
          <span className="c-badge">{UI_STRINGS.comingSoon}</span>
        </button>

        <button type="button" className="c-launch__item c-launch__item--soon" aria-disabled="true">
          <Glyph d={ICON.screen} />
          <span className="c-launch__name">{UI_STRINGS.captureFullScreen}</span>
          <span className="c-badge">{UI_STRINGS.comingSoon}</span>
        </button>
      </div>

      {hint.state === 'ready' && (
        <p className="c-hint" style={BLOCK_GAP}>
          <Keys keys={hint.keys} />
          <span>{UI_STRINGS.shortcutHint}</span>
        </p>
      )}

      {hint.state === 'loading' && (
        <p className="c-hint" style={BLOCK_GAP} aria-busy="true">
          <span className="c-kbd c-skeleton" aria-hidden="true">
            {RESERVED_CAP}
          </span>
          <span>{UI_STRINGS.shortcutLoading}</span>
        </p>
      )}

      {hint.state === 'refused' && (
        <div className="c-note c-note--danger" role="alert" style={BLOCK_GAP}>
          <Glyph d={ICON.alert} />
          <span>
            <strong>{UI_STRINGS.failure}</strong>
            {' — '}
            {UI_STRINGS.shortcutRegistryUnreadable}
          </span>
        </div>
      )}
    </>
  );
}
