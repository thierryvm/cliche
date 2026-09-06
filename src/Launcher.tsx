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
 * WHAT THE PRIMARY TILE DOES, SINCE 6 SEPTEMBER 2026: it starts a capture, by
 * the same `veil::perform_capture` the global shortcut runs - `capture_region`
 * in `src-tauri/src/launch.rs`, granted to this window by
 * `capabilities/default.json`. It used to reach nothing at all, which this
 * header called a defect rather than a detail; it is one no longer.
 *
 * TWO THINGS THAT ARE STILL NOT DONE, said here because a screen that looks
 * finished is where an unfinished thing hides:
 *
 *   - The capture the tile starts freezes the screen WITH THIS WINDOW IN IT.
 *     Nothing hides Cliche first, and `launch.rs` explains why nothing does it
 *     behind a decision that is not the code's to take.
 *   - A `capture_region` that REJECTS - only ever a misconfigured ACL, since a
 *     failure inside the pipeline never comes back this way - is written to the
 *     console and to nothing else. The maquette has no wording for it, and this
 *     file may not invent any: every sentence on this screen comes from
 *     `src/strings.ts`, and every value there is the showcase's.
 */

import { useEffect, useState } from 'react';

import Keys from './Keys';
import { Glyph, ICON } from './design/Glyph';
import { captureRegion } from './launch';
import { hintFor } from './shortcut-hint';
import type { RegistryRead } from './shortcut-hint';
import { describeShortcuts, describeShortcutStatus } from './shortcuts';
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
 *
 * It is the LITERAL character below, and it is invisible in this file. Rewriting
 * this file on 6 September 2026 replaced it with an ordinary space in passing,
 * and nothing in the suite could have said so - the chip would simply have lost
 * its line box on a screen nobody was looking at. Checked from the outside, and
 * this is the check to repeat after any edit of this line:
 *   rg "RESERVED_CAP = '\x{00A0}';" src/Launcher.tsx
 */
const RESERVED_CAP = ' ';

/**
 * The combination as the refusal note writes it: one string, not chips.
 *
 * The maquette draws it inside `.c-num` rather than with `Keys` there
 * (`Showcase.tsx`, the `refused` branch) - a run of keycaps inside a sentence
 * would break the line where the sentence should not break.
 */
function drawn(keys: readonly string[]): string {
  return keys.join(' + ');
}

export default function Launcher() {
  const [read, setRead] = useState<RegistryRead>({ status: 'reading' });

  useEffect(() => {
    // StrictMode runs effects twice in development, so both commands are asked
    // twice there. That is the dev double-render, not a bug.
    let abandoned = false;

    // Both or neither: the reminder is drawn from the two together - what this
    // application asked for, and what the system answered - and a screen that
    // had one of them would have to guess the other.
    Promise.all([describeShortcuts(), describeShortcutStatus()]).then(
      ([entries, registration]) => {
        if (abandoned) return;

        if (registration.status !== 'accepted') {
          // The reason is the operating system's own words, or this
          // application's: English, technical, and useful to exactly one
          // reader. The terminal has the same line from Rust; this puts it
          // where a developer with only the webview open can see it too.
          console.warn(
            `[cliche] launcher: the capture shortcut is ${registration.status}`,
            registration.reason,
          );
        }
        setRead({ status: 'read', entries, registration });
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
        <button
          type="button"
          className="c-launch__item c-launch__item--primary"
          onClick={() => {
            // Not awaited: resolving means a capture was STARTED, and the veil
            // takes the screen from here. Nothing on this screen changes on the
            // way back, so there is no state to guard against a stale answer.
            captureRegion().catch((error: unknown) => {
              console.error('[cliche] launcher: the capture could not be started', error);
            });
          }}
        >
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

      {/* The maquette's own note, word for word: the combination is NAMED, and
          what the user can still do is said (PRD R4). It is drawn only when
          Windows really refused - `shortcut-hint.ts` is what holds that line. */}
      {hint.state === 'refused' && (
        <div className="c-note c-note--danger" role="alert" style={BLOCK_GAP}>
          <Glyph d={ICON.alert} />
          <span>
            <strong>{UI_STRINGS.shortcutRefused}</strong>
            {' — '}
            <span className="c-num">{drawn(hint.keys)}</span> {UI_STRINGS.shortcutHeldByAnother}{' '}
            {UI_STRINGS.shortcutMouseStillWorks}
          </span>
        </div>
      )}

      {/* No shortcut, and no combination anyone may be told was refused. The
          second half of the note above is the half that is still true. */}
      {hint.state === 'unavailable' && (
        <div className="c-note c-note--danger" role="alert" style={BLOCK_GAP}>
          <Glyph d={ICON.alert} />
          <span>
            <strong>{UI_STRINGS.shortcutUnavailable}</strong>
            {' — '}
            {UI_STRINGS.shortcutMouseStillWorks}
          </span>
        </div>
      )}

      {hint.state === 'unreadable' && (
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
