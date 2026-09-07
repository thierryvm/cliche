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

  /* The veil: the one line of chrome laid over the frozen screen.

     IT SPOKE ENGLISH UNTIL 7 SEPTEMBER 2026, and it was typed straight into
     `veil.html` - the only sentence of this product that never passed through
     this file. Thierry met it on the installed v0.1.0 and read it as a
     rendering artefact: « le bandeau noir visible en bas sur toute la largeur,
     je ne vois pas pourquoi et à quoi il sert ! ». Being in the wrong language
     was the first reason it said nothing to him.

     THE TWO SPACES AROUND THE SEPARATOR ARE ORDINARY U+0020, and the markup
     they replace used `&#160;`. That is a LOSS, stated rather than hidden: the
     agent that moved this sentence could not write a U+00A0 into a file - every
     one it emitted came back folded to U+0020, checked with
     `rg "copie\x{00A0}"` and again with `rg "copie\x20"`. Nothing else in the
     catalogue carries one either (`rg -c "\x{00A0}" src/` finds exactly one
     character in the whole tree, in `src/Launcher.tsx`), so the line is
     consistent with its neighbours rather than half-repaired.
     TO PUT THEM BACK, edit BOTH sides - here and in `src/design/Showcase.tsx`.
     `scripts/check-strings.mjs` compares character for character and does not
     collapse a non-breaking space, so changing one side alone fails the suite,
     which is the safety net for that edit. */
  veilHint: 'Entrée ou double-clic copie · Échap annule',

  /* The confirmation that follows a capture, and the failure that replaces it.
     `copied` is ONE WORD on purpose: the showcase writes
     `<span class="c-num">933×577</span> copié`, so the sentence is a figure the
     application measures followed by this label. Storing it whole would mean
     storing a dimension nobody can translate. */
  copied: 'copié',

  /* Messages that do not fade on their own need a way out. */
  dismissMessage: 'Fermer le message',
  failure: 'Échec',

  /* Settings: the shortcut recorder.

     `settingsTitle` NAMES THE SCREEN, and it is here for the reason `helpTitle`
     below is: a catalogue may not invent a word the system does not draw, so
     the showcase published « Réglages » as a `c-screen__name` first, next to the
     recorder it belongs to. Same method Thierry settled on that morning for
     « Aide ». */
  settingsTitle: 'Réglages',
  /* WHAT THIS SCREEN IS, in one line under its name. Added 7 September 2026:
     one field in a 900 px window reads as a screen somebody forgot to finish,
     and the missing information is the PERIMETER - what is here now, and what
     is coming to this screen and not to another.
     EVERY FUTURE IT NAMES IS IN `docs/PRD.md` §3 as INDISPENSABLE v1: the PNG
     save with its « ne pas enregistrer automatiquement » switch, and retention
     with definitive erasure. No date is promised, because none is known. */
  settingsLede: "Un seul réglage aujourd'hui : la combinaison qui déclenche la capture. L'enregistrement automatique des captures et leur durée de rétention viendront ici.",
  shortcutFieldLabel: 'Raccourci de capture',
  shortcutChange: 'Modifier',
  shortcutListening: 'Appuyez sur une combinaison…',
  shortcutEscapeCancels: 'Échap annule.',
  shortcutUnavailable: 'Indisponible',
  shortcutRegistryUnreadable: "Le registre des raccourcis n'a pas pu être lu.",
  /* THE SAME SENTENCE IN ITS QUOTING FORM, added 7 September 2026. Not a second
     wording: the one above ENDS a note, this one INTRODUCES the words of
     whatever rejected - `describe_shortcut_status`'s own error string, in
     English, shown as it arrived. `displaysUnreadable` below is the same pair of
     shapes for the same reason, and it is the precedent this follows.
     WHY BOTH ARE KEPT rather than merged into the colon form: a rejection can
     carry nothing readable, and « lu : » with an empty tail is a sentence the
     interface failed to finish. `src/veil/confirmation.ts` settled that rule and
     `hintFor` in `src/shortcut-hint.ts` is what picks between the two.
     The space before the colon is an ORDINARY U+0020, like the one in
     `displaysUnreadable`: the whole tree holds exactly one non-breaking space
     (see the `veilHint` note above), and a lone U+00A0 here would be a
     typographic rule applied in one place out of two. */
  shortcutRegistryUnreadableWithReason: "Le registre des raccourcis n'a pas pu être lu :",

  /* What the recorder says once the system has answered. Catalogued in PIECES,
     for the reason the header amends: each of these is one node of a sentence
     whose other nodes are combinations this application measured, and a
     sentence stored whole would be a sentence holding a combination nobody can
     translate. Kept each on ONE line - `scripts/check-strings.mjs` reads one
     pair per line. */
  shortcutAlreadyTaken: 'est déjà pris par une autre application.',
  shortcutPreviousStillActive: 'reste actif.',
  shortcutNoneActive: "Aucun raccourci n'est actif. Choisissez-en un autre.",
  shortcutNotSaved: "Le raccourci fonctionne, mais le réglage n'a pas pu être écrit : il ne survivra pas au redémarrage.",
  /* The two presses this application refuses on its own, before Windows is
     asked. The first is the dangerous one: a bare key registered globally is
     taken from every other application on the machine. */
  shortcutNeedsModifier: 'Une touche seule serait prise à toutes les autres applications. Ajoutez un modificateur.',
  shortcutKeyUnsupported: 'Cette touche ne peut pas servir de raccourci. Essayez une lettre, un chiffre ou une touche de fonction.',

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
  /* Same line, same day, same reason as `settingsLede` above: this screen shows
     one registered combination and a list of monitors, and without a word about
     its perimeter it reads as an empty page rather than a short one.
     THE THREE FUTURES IT NAMES ARE THE PRD'S, not this file's: the annotation
     editor and the local library are §3 INDISPENSABLE v1, the scrolling page
     capture is §3 SOUHAITABLE v1. They belong in a sentence about the HELP
     because this screen derives from the shortcut registry - a capability that
     binds a combination shows up here on its own, which is lot 2's whole
     point. */
  helpLede: "Pour l'instant, cette page dit deux choses : les raccourcis enregistrés, et les écrans que cette machine expose. L'éditeur d'annotation, la bibliothèque et la capture de page défilante y ajouteront leurs lignes.",
  helpCategoryCapture: 'Capture',
  dismissVeil: 'Fermer le voile sans capturer',
  captureWindowUnderPointer: 'Capturer la fenêtre sous le pointeur',

  /* Help: the diagnostic read-out under the key map.

     IT SPOKE ENGLISH UNTIL 6 SEPTEMBER 2026, and the reason is worth keeping:
     it was the whole of the old debug screen, and it kept its wording when it
     became a section of the help page. `DisplaysProbe.tsx`'s own header used to
     argue that an engineering read-out is not interface prose - which is true
     of a screen nobody but a developer opens, and stopped being true the day it
     landed inside the help, in French, under a French heading. What it says
     about the machine is now French like everything else around it.

     The one thing that is NOT catalogued is the message the system itself
     returns on a failure. That is a quotation - `xcap`'s own words - and
     translating a quotation is inventing one. */
  displaysHeading: 'Écrans détectés au démarrage',
  displaysReading: 'Lecture de la liste des écrans…',
  displaysUnreadable: "la liste des écrans n'a pas pu être lue :",
  displayOne: 'écran',
  displayMany: 'écrans',
  displayUnnamed: '(sans nom)',
  displayPhysicalPixels: 'px physiques',
  displayOrigin: 'origine',
  displayScale: 'échelle',
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
