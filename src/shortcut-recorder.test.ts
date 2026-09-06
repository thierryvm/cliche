/**
 * The shortcut recorder's two decisions, under test.
 *
 * Same rule as `shortcut-hint.test.ts`: no jsdom, no testing-library, so the
 * decisions are functions of what a key press and an IPC answer held, and the
 * component only renders their result.
 *
 * WHAT THIS CANNOT REACH: that `Settings.tsx` really listens for `keydown`,
 * really calls `set_capture_shortcut`, and really draws what comes back. That
 * needs the Tauri webview, and nothing here observes it.
 */

import { describe, expect, it } from 'vitest';

import { drawn, outcomeOf, record } from './shortcut-recorder';
import type { RecorderPress } from './shortcut-recorder';
import type { Combination, ShortcutChange } from './shortcuts';

/** A press with nothing held. Overridden one flag at a time below. */
function press(overrides: Partial<RecorderPress> & Pick<RecorderPress, 'code'>): RecorderPress {
  return {
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    metaKey: false,
    ...overrides,
  };
}

/** A combination shaped exactly as Rust serialises one. */
function combination(accelerator: string, keys: readonly string[]): Combination {
  return { accelerator, keys };
}

const CAPTURE = combination('Ctrl+Shift+Digit2', ['Ctrl', 'Maj', '2']);
const WANTED = combination('Ctrl+Shift+KeyA', ['Ctrl', 'Maj', 'A']);

describe('record', () => {
  it('waits while only modifiers are held', () => {
    // Everybody presses Ctrl before the key that follows it. A field that
    // answered on the first of the two would be unusable.
    for (const code of ['ControlLeft', 'ShiftRight', 'AltLeft', 'MetaLeft']) {
      expect(record(press({ code, ctrlKey: true }))).toEqual({ kind: 'holding' });
    }
  });

  it('reads Escape as the cancellation the maquette promises', () => {
    // « Échap annule. » is printed under the listening field. This is that
    // sentence being true.
    expect(record(press({ code: 'Escape' }))).toEqual({ kind: 'cancelled' });
  });

  it('refuses a key with no modifier, and says which is missing', () => {
    // THE dangerous press. `Digit2` registered globally is taken from every
    // other application on the machine: the user can no longer type a `2`
    // anywhere, in any program. Windows allows it without a word.
    expect(record(press({ code: 'Digit2' }))).toEqual({
      kind: 'refused',
      why: 'shortcutNeedsModifier',
    });
    expect(record(press({ code: 'KeyA' }))).toEqual({
      kind: 'refused',
      why: 'shortcutNeedsModifier',
    });
  });

  it('refuses a key this application cannot draw', () => {
    // Narrower than the plugin's own parser on purpose: Rust refuses every key
    // it has no cap for, and letting one through here would spend a round trip
    // to come back with a worse message.
    for (const code of ['Enter', 'Space', 'ArrowUp', 'NumpadAdd', 'F25']) {
      expect(record(press({ code, ctrlKey: true }))).toEqual({
        kind: 'refused',
        why: 'shortcutKeyUnsupported',
      });
    }
  });

  it('writes the modifiers in the order Rust writes them', () => {
    // Not cosmetic: `shortcuts::accept` rebuilds a canonical accelerator as
    // Ctrl, Shift, Alt, Super. Sending the same combination spelled another way
    // would have Rust answer with a string this screen never sent.
    expect(
      record(
        press({ code: 'KeyQ', altKey: true, ctrlKey: true, metaKey: true, shiftKey: true }),
      ),
    ).toEqual({ kind: 'combination', accelerator: 'Ctrl+Shift+Alt+Super+KeyQ' });
  });

  it('builds the accelerator out of the physical code and the modifiers held', () => {
    expect(record(press({ code: 'Digit2', ctrlKey: true, shiftKey: true }))).toEqual({
      kind: 'combination',
      accelerator: 'Ctrl+Shift+Digit2',
    });
    expect(record(press({ code: 'F5', altKey: true }))).toEqual({
      kind: 'combination',
      accelerator: 'Alt+F5',
    });
  });
});

describe('outcomeOf', () => {
  it('shows the new combination when the system took it', () => {
    const change: ShortcutChange = { outcome: 'changed', active: WANTED, saved: true };

    expect(outcomeOf(change)).toEqual({
      active: WANTED.keys,
      note: { state: 'settled' },
    });
  });

  it('says so when the shortcut works and the choice was not written down', () => {
    // The shortcut IS live, so refusing to show it would be a lie in the other
    // direction. What has to be said is that it will be gone after a restart.
    const change: ShortcutChange = { outcome: 'changed', active: WANTED, saved: false };

    expect(outcomeOf(change)).toEqual({
      active: WANTED.keys,
      note: { state: 'unsaved' },
    });
  });

  it('goes back to the PREVIOUS combination when the new one was refused', () => {
    // THE row this lot exists for. Two things have to be right at once: the
    // field shows the combination that is registered - the old one - and the
    // message names the one that was refused. Showing the refused combination
    // would put a shortcut on screen that nothing is listening for.
    const change: ShortcutChange = {
      outcome: 'kept',
      refused: WANTED,
      active: CAPTURE,
      reason: 'HotKey already registered',
    };

    expect(outcomeOf(change)).toEqual({
      active: CAPTURE.keys,
      note: { state: 'kept', refused: WANTED.keys },
    });
  });

  it('shows no combination at all when nothing is registered', () => {
    // The honest end of the rollback: the new one was refused and the old one
    // could not be taken back. Drawing either would promise keys that do
    // nothing.
    const change: ShortcutChange = {
      outcome: 'stranded',
      refused: WANTED,
      reason: 'HotKey already registered',
    };

    expect(outcomeOf(change)).toEqual({
      active: undefined,
      note: { state: 'stranded', refused: WANTED.keys },
    });
  });
});

describe('drawn', () => {
  it('joins a combination the way a sentence names one', () => {
    expect(drawn(CAPTURE.keys)).toBe('Ctrl + Maj + 2');
    expect(drawn(['Échap'])).toBe('Échap');
  });
});
