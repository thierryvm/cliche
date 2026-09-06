/**
 * The help page's content, DERIVED from the shortcut registry.
 *
 * # This file is what « lot 2 » means by « sans qu'aucun fichier d'aide ait été
 * # touché »
 *
 * `docs/PLAN.md` states the finish line for that lot: add an entry to the Rust
 * table, restart, and the shortcut appears in the help WITHOUT any help file
 * being edited. Nothing here knows a combination, a key cap or a sentence. It
 * takes what `describe_shortcuts` handed over and turns it into rows; the
 * component below it draws rows and decides nothing.
 *
 * That is also why this is a module of its own rather than three `map` calls
 * inside `Help.tsx`: the derivation is the CLAIM, and a claim that lives inside
 * a component needs jsdom to be put to the test. Same rule as
 * `shortcut-hint.ts` and `window-controls.ts`.
 *
 * # WHAT ARRIVES HERE IS NOT TRUSTED
 *
 * `descriptionKey` and `category` cross the IPC frontier at run time. Their
 * TypeScript types are a promise the compiler never gets to check - see the
 * comment on `ShortcutEntry.descriptionKey` in `src/shortcuts.ts` - so both are
 * looked up rather than indexed, and both can come back `undefined`. What holds
 * them honest in practice is a RUST test that reads `src/strings.ts`
 * (`every_description_key_exists_in_the_string_catalogue`); the `undefined`
 * branches are what happens on the day that test is wrong, and they are written
 * so the page still lists the shortcut instead of quietly dropping it.
 */

import type { ShortcutCategory, ShortcutEntry } from './shortcuts';
import { UI_STRINGS } from './strings';
import type { StringKey } from './strings';

/** One row of the key map: a combination, and what it does. */
export interface HelpLine {
  /** The registry's own identifier. Never shown; it is the React key. */
  readonly id: string;
  /** The combination as it is DRAWN, one chip per key. Straight from Rust. */
  readonly keys: readonly string[];
  /**
   * The sentence from the catalogue, or `undefined` when the registry points at
   * a key `src/strings.ts` does not define.
   *
   * `undefined` and not a fallback of our own: this file may not invent French,
   * and printing the raw `descriptionKey` would put an engineering identifier
   * in front of a user. The LINE still exists, because the combination is real
   * and registered - a shortcut the help silently omitted would be one the user
   * presses with nothing to explain it.
   */
  readonly description: string | undefined;
  /** The heading this row files itself under. */
  readonly category: ShortcutCategory;
}

/** One heading of the help page, and the rows under it. */
export interface HelpSection {
  /** The category every row here shares. Used as the React key. */
  readonly category: ShortcutCategory;
  /**
   * The heading from the catalogue, or `undefined` for a category nothing has
   * named yet - in which case the rows are drawn without one rather than under
   * a word invented here.
   */
  readonly heading: string | undefined;
  readonly lines: readonly HelpLine[];
}

/**
 * Which catalogue entry names each heading.
 *
 * The ONE place the two closed lists meet - `ShortcutCategory` in
 * `src/shortcuts.ts`, mirroring the Rust enum, and `UI_STRINGS`. `satisfies`
 * rather than an annotation, so adding a category to the union fails the
 * type-check HERE, on the line that has to decide what it is called.
 */
const CATEGORY_HEADING = {
  capture: 'helpCategoryCapture',
} as const satisfies Record<ShortcutCategory, StringKey>;

/**
 * Whether a string that arrived at run time is a key of the catalogue.
 *
 * `Object.prototype.hasOwnProperty.call` and not `key in UI_STRINGS`: `in`
 * walks the prototype chain, so `'toString'` would pass and the lookup would
 * hand back a function where a sentence was expected.
 */
function isStringKey(key: string): key is StringKey {
  return Object.prototype.hasOwnProperty.call(UI_STRINGS, key);
}

/** The catalogue's sentence for a key that came over the wire, if it has one. */
function labelFor(key: string): string | undefined {
  return isStringKey(key) ? UI_STRINGS[key] : undefined;
}

/** The catalogue's heading for a category that came over the wire, if it has one. */
function headingFor(category: ShortcutCategory): string | undefined {
  if (!Object.prototype.hasOwnProperty.call(CATEGORY_HEADING, category)) {
    return undefined;
  }
  return UI_STRINGS[CATEGORY_HEADING[category]];
}

/**
 * One line per registry entry, in the order the registry states them.
 *
 * THE function `docs/PLAN.md` asks for a test on: as many lines as entries,
 * every combination present, and one more entry means one more line. It maps
 * and does nothing else - no filter, no sort, no dedupe - because each of those
 * would be a way for a registered shortcut to go missing from the help.
 */
export function helpLines(entries: readonly ShortcutEntry[]): readonly HelpLine[] {
  return entries.map((entry) => ({
    id: entry.id,
    keys: entry.keys,
    description: labelFor(entry.descriptionKey),
    category: entry.category,
  }));
}

/**
 * The same lines, gathered under one heading per category.
 *
 * First appearance decides the order of the headings, and the order inside a
 * heading is the registry's: the page is a reference, and a reference whose
 * order changes between two reads is one nobody can scan twice.
 */
export function helpSections(entries: readonly ShortcutEntry[]): readonly HelpSection[] {
  const sections: { category: ShortcutCategory; heading: string | undefined; lines: HelpLine[] }[] =
    [];

  for (const line of helpLines(entries)) {
    const opened = sections.find((section) => section.category === line.category);

    if (opened === undefined) {
      sections.push({
        category: line.category,
        heading: headingFor(line.category),
        lines: [line],
      });
    } else {
      opened.lines.push(line);
    }
  }

  return sections;
}
