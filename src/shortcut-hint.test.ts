/**
 * The launcher's shortcut reminder, under test.
 *
 * Same rule as `window-controls.test.ts`: no jsdom, no testing-library, so the
 * decision is a function of what the IPC call gave back and the component only
 * renders its answer.
 *
 * WHAT THIS CANNOT REACH: that `describe_shortcuts` is actually called, and
 * that the promise's three outcomes are wired to the three inputs below. That
 * is `Launcher.tsx`, it needs the Tauri webview, and nothing here observes it.
 */

import { describe, expect, it } from 'vitest';

import { hintFor } from './shortcut-hint';
import type { RegistryRead } from './shortcut-hint';
import type { ShortcutEntry } from './shortcuts';

/**
 * An entry shaped exactly as `describe_shortcuts` serialises one.
 *
 * The base is annotated rather than left to inference: without it the literal
 * `'capture'` would widen to `string` inside the spread and stop being a
 * `ShortcutCategory`.
 */
function entry(overrides: Partial<ShortcutEntry> = {}): ShortcutEntry {
  const base: ShortcutEntry = {
    id: 'capture-region',
    accelerator: 'Ctrl+Shift+Digit2',
    keys: ['Ctrl', 'Maj', '2'],
    descriptionKey: 'captureRegion',
    category: 'capture',
  };

  return { ...base, ...overrides };
}

describe('hintFor', () => {
  it('waits while the registry is being read', () => {
    expect(hintFor({ status: 'reading' })).toEqual({ state: 'loading' });
  });

  it('shows the keys the registry states, and never a combination of its own', () => {
    const read: RegistryRead = {
      status: 'read',
      entries: [entry({ keys: ['Ctrl', 'Alt', 'F9'] })],
    };

    expect(hintFor(read)).toEqual({ state: 'ready', keys: ['Ctrl', 'Alt', 'F9'] });
  });

  it('finds the capture entry by what it describes, not by where it sits', () => {
    // A registry that grows a second entry must not make the launcher announce
    // the wrong combination. `[0]` would pass every other test in this file.
    const read: RegistryRead = {
      status: 'read',
      entries: [
        entry({ id: 'dismiss-veil', keys: ['Échap'], descriptionKey: 'dismissVeil' }),
        entry({ keys: ['Ctrl', 'Maj', '2'] }),
      ],
    };

    expect(hintFor(read)).toEqual({ state: 'ready', keys: ['Ctrl', 'Maj', '2'] });
  });

  it('refuses to announce anything when the registry describes no region capture', () => {
    const read: RegistryRead = {
      status: 'read',
      entries: [entry({ id: 'dismiss-veil', descriptionKey: 'dismissVeil' })],
    };

    expect(hintFor(read)).toEqual({ state: 'refused' });
  });

  it('refuses an entry that carries no key at all', () => {
    // An empty list would draw a reminder with no keycap in it: a sentence
    // claiming a shortcut exists, with nothing to press. Worse than no line.
    expect(hintFor({ status: 'read', entries: [entry({ keys: [] })] })).toEqual({
      state: 'refused',
    });
  });

  it('says so when the registry could not be read', () => {
    expect(hintFor({ status: 'unreadable' })).toEqual({ state: 'refused' });
  });
});
