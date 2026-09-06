/**
 * The help page's derivation, under test.
 *
 * # THE POINT OF THIS FILE, in one sentence
 *
 * `docs/PLAN.md`, lot 2: « j'ajoute une entrée au tableau, je relance, et le
 * raccourci apparaît dans l'Aide sans qu'aucun fichier d'aide ait été touché ».
 * What proves that is not a screenshot - it is that the number of rows is a
 * function of the registry and of nothing else.
 *
 * # WHY THE ENTRIES BELOW ARE FABRICATED, and never the real registry
 *
 * Two reasons, and the first is the one that matters:
 *
 * 1. A test that read the real table would only ever prove the derivation at
 *    the length that table happens to have. It holds ONE entry today, so
 *    « as many lines as entries » would be « one line », which a hard-coded
 *    `1` also satisfies. Fabricated lists of two, three and four are what make
 *    the assertion about the FUNCTION rather than about today's table.
 * 2. The real table cannot be grown to try: `shortcuts.rs` carries a tripwire,
 *    `the_registry_holds_only_shortcuts_something_binds`, which fails the suite
 *    the moment an entry is added without a handler in `shortcut::install`.
 *    That tripwire is right, and this file must not need it lifted.
 *
 * Same rule as `shortcut-hint.test.ts`: no jsdom, no testing-library. What
 * `Help.tsx` does with these rows - one `<dt>`, one `<dd>` - is not observed
 * here, and that is this file's blind spot, stated rather than left implied.
 */

import { describe, expect, it } from 'vitest';

import { helpLines, helpSections } from './help-rows';
import type { ShortcutEntry } from './shortcuts';
import { UI_STRINGS } from './strings';

/**
 * An entry shaped exactly as `describe_shortcuts` serialises one.
 *
 * The base is annotated rather than left to inference, for the reason
 * `shortcut-hint.test.ts` gives: without it the literal `'capture'` widens to
 * `string` inside the spread and stops being a `ShortcutCategory`.
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

/** Three entries that are NOT the registry's, so the count means something. */
const THREE: readonly ShortcutEntry[] = [
  entry(),
  entry({ id: 'dismiss-veil', accelerator: 'Escape', keys: ['Échap'], descriptionKey: 'dismissVeil' }),
  entry({
    id: 'capture-window',
    accelerator: 'Ctrl+Shift+Digit3',
    keys: ['Ctrl', 'Maj', '3'],
    descriptionKey: 'captureWindowUnderPointer',
  }),
];

describe('helpLines', () => {
  it('draws exactly as many rows as the registry states entries', () => {
    expect(helpLines(THREE)).toHaveLength(THREE.length);
  });

  it('carries every combination the registry states, in its order', () => {
    // The combinations are the thing a help page exists to publish, and the
    // ONLY place they come from is the entry. Compared as a whole list, so an
    // entry drawn with another entry's keys fails here rather than on screen.
    expect(helpLines(THREE).map((line) => line.keys)).toEqual([
      ['Ctrl', 'Maj', '2'],
      ['Échap'],
      ['Ctrl', 'Maj', '3'],
    ]);
  });

  it('grows by exactly one row when the registry grows by one entry', () => {
    // THE finish line of lot 2, as a property rather than as a screenshot. No
    // file under src/ was touched between these two calls.
    const added = entry({
      id: 'capture-screen',
      accelerator: 'Ctrl+Shift+Digit4',
      keys: ['Ctrl', 'Maj', '4'],
      descriptionKey: 'captureFullScreen',
    });

    const before = helpLines(THREE);
    const after = helpLines([...THREE, added]);

    expect(after).toHaveLength(before.length + 1);
    expect(after.map((line) => line.keys)).toContainEqual(['Ctrl', 'Maj', '4']);
    expect(after.map((line) => line.description)).toContain(UI_STRINGS.captureFullScreen);
  });

  it('holds at any length, including none and one', () => {
    // The two ends of the range. Zero because a `map` is not the only way to
    // write this function and some of the others - a `find`, a lookup by id -
    // are wrong at zero or at one, silently.
    expect(helpLines([])).toHaveLength(0);
    expect(helpLines([entry()])).toHaveLength(1);
  });

  it('names what each shortcut does from the catalogue, never from the entry', () => {
    // The entry carries a KEY, not a sentence. If this ever returned the key
    // itself, the help page would read « dismissVeil » in French prose.
    const lines = helpLines(THREE);

    expect(lines.map((line) => line.description)).toEqual([
      UI_STRINGS.captureRegion,
      UI_STRINGS.dismissVeil,
      UI_STRINGS.captureWindowUnderPointer,
    ]);
  });

  it('still lists a shortcut the catalogue cannot name', () => {
    // A registered combination the user can press. Dropping the row would hide
    // a working shortcut; inventing a sentence would put French in a file that
    // is not the catalogue. It is listed, unnamed, and that is deliberate.
    const lines = helpLines([entry({ descriptionKey: 'noSuchKeyInTheCatalogue' })]);

    expect(lines).toHaveLength(1);
    expect(lines[0]?.description).toBeUndefined();
  });

  it('does not mistake an inherited property for a catalogue entry', () => {
    // `'toString' in UI_STRINGS` is true, and a lookup written that way would
    // put a JavaScript function where a sentence belongs.
    expect(helpLines([entry({ descriptionKey: 'toString' })])[0]?.description).toBeUndefined();
  });
});

describe('helpSections', () => {
  it('gathers the rows under one heading, and loses none of them', () => {
    const sections = helpSections(THREE);

    expect(sections).toHaveLength(1);
    expect(sections[0]?.heading).toBe(UI_STRINGS.helpCategoryCapture);
    expect(sections[0]?.lines).toHaveLength(THREE.length);
  });

  it('files every entry somewhere, whatever the registry holds', () => {
    // The property that survives a second category being added to the union:
    // the sections, flattened, are the lines. A grouping that dropped a row
    // would be a shortcut missing from the help with nothing to say so.
    const flattened = helpSections(THREE).flatMap((section) => section.lines);

    expect(flattened).toEqual(helpLines(THREE));
  });

  it('has no heading to show for an empty registry', () => {
    expect(helpSections([])).toHaveLength(0);
  });
});
