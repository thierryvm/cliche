/**
 * Every label the interface shows, in ONE place.
 *
 * Keys are English, values are French. The product speaks French; the code that
 * moves it around does not, and a key named `captureRegion` stays readable to a
 * reader who cannot read the sentence it points at.
 *
 * # The wording is not decided here
 *
 * It is decided in `src/design/Showcase.tsx`, which is the system on one page
 * and the thing that gets LOOKED at. Every value below is copied from there,
 * character for character, and `scripts/check-strings.mjs` fails the suite the
 * day the two disagree. The showcase writes `&apos;`, which JSX decodes to a
 * plain `'` before it reaches the screen; that is the apostrophe used here.
 *
 * # What is deliberately NOT in this catalogue
 *
 * - `Cliché`. It is the product's NAME, not a translated label: it is spelled
 *   the same in every language, and it appears in places this catalogue must
 *   not reach - an English exception message in `src/main.tsx`, for one.
 * - Key caps (`Ctrl`, `Maj`, `2`). Those belong to a shortcut and travel with
 *   it, in `src-tauri/src/shortcuts.rs`, where the combination that is actually
 *   registered can be held against them. Splitting them off would put the
 *   label in one file and the fact it claims in another.
 * - Sentences the showcase assembles from several nodes - a bold lead, a
 *   dimension, a trailing clause - stored WHOLE. A sentence stored whole is a
 *   sentence that cannot be reordered in another language.
 *
 *   AMENDED ON 6 SEPTEMBER 2026, because the rule as written also forbade the
 *   pieces, and that is not what it is for. `shortcutHeldByAnother` and
 *   `shortcutMouseStillWorks` are two such pieces, catalogued one node each:
 *   the launcher has to SAY the refusal now that Rust reports it, and a
 *   sentence typed into a component is exactly the copy check 2 exists to
 *   refuse. They are kept apart rather than joined because they are not equally
 *   true: the first names a cause only Windows can confirm, the second holds
 *   whenever there is no shortcut at all. `src/shortcut-hint.ts` decides which
 *   of the two the screen is entitled to show.
 *
 * `as const` for the reason the showcase states: `noUncheckedIndexedAccess`
 * turns an indexed read of a plain `Record<string, string>` into
 * `string | undefined`, and a label that might be missing is a label every call
 * site has to apologise for.
 */

export const UI_STRINGS = {
  /* The launcher: the home screen's three capture actions. */
  capture: 'Capturer',
  captureActions: 'Actions de capture',
  captureRegion: 'Capturer une zone',
  captureWindow: 'Capturer une fenêtre',
  captureFullScreen: "Capturer tout l'écran",
  comingSoon: 'à venir',

  /* What the capture shortcut is doing, in the states it can be in. */
  shortcutHint: 'capture une zone, même quand Cliché est en arrière-plan.',
  shortcutLoading: 'lecture du registre des raccourcis…',
  shortcutRefused: 'Raccourci refusé',
  /* The two halves of the refusal note, catalogued APART - see the header. The
     first names a cause only Windows can confirm; the second is true whenever
     there is no shortcut, whatever the reason. Kept each on ONE line because
     `scripts/check-strings.mjs` reads one pair per line. */
  shortcutHeldByAnother: 'est tenu par une autre application.',
  shortcutMouseStillWorks: 'Cliché tourne sans son raccourci ; les trois actions ci-dessus restent utilisables à la souris.',

  /* The custom title bar. Drawn by this application, so named by it too. */
  windowMinimize: 'Réduire',
  windowMaximize: 'Agrandir',
  windowRestore: 'Restaurer',
  windowClose: 'Fermer',

  /* The confirmation that follows a capture, and the failure that replaces it.
     `copied` is ONE WORD on purpose: the showcase writes
     `<span class="c-num">933×577</span> copié`, so the sentence is a figure the
     application measures followed by this label. Storing it whole would mean
     storing a dimension nobody can translate. */
  copied: 'copié',

  /* Messages that do not fade on their own need a way out. */
  dismissMessage: 'Fermer le message',
  failure: 'Échec',

  /* Settings: the shortcut recorder. */
  shortcutFieldLabel: 'Raccourci de capture',
  shortcutChange: 'Modifier',
  shortcutListening: 'Appuyez sur une combinaison…',
  shortcutEscapeCancels: 'Échap annule.',
  shortcutUnavailable: 'Indisponible',
  shortcutRegistryUnreadable: "Le registre des raccourcis n'a pas pu être lu.",

  /* Help: the headings, and what each registered shortcut DOES.
     `captureRegion` above is one of these too - it is the launcher tile and the
     help line at once, which is the point of a catalogue.

     `helpTitle` NAMES THE SCREEN. It briefly read « Raccourci » - singular,
     and not the name of anything - because the showcase drew the key map
     (`#/systeme`, section « V1 · aide ») without publishing a name for the
     screen it belongs to, and this catalogue may not invent a word the system
     does not draw. Thierry settled it on 6 September 2026: the SHOWCASE was
     completed, not the rule bent. The word now exists there, as a
     `c-screen__name`, the way « Capturer » does for the launcher. */
  helpTitle: 'Aide',
  helpCategoryCapture: 'Capture',
  dismissVeil: 'Fermer le voile sans capturer',
  captureWindowUnderPointer: 'Capturer la fenêtre sous le pointeur',
} as const;

/**
 * Every key of the catalogue.
 *
 * `ShortcutEntry.descriptionKey` in `src/shortcuts.ts` is NOT typed with this,
 * on purpose: it arrives from Rust at run time, and narrowing an unvalidated
 * string to a union would be a claim TypeScript cannot check. What keeps those
 * keys inside this catalogue is a test that reads both files -
 * `every_description_key_exists_in_the_string_catalogue`, in
 * `src-tauri/src/shortcuts.rs`.
 */
export type StringKey = keyof typeof UI_STRINGS;
