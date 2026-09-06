/*
 * The settings screen: the capture combination, and the one control that changes
 * it.
 *
 * # NOT ONE DECISION IS MADE IN THIS FILE
 *
 * Two modules already hold the two decisions, and both are functions of their
 * arguments with tests of their own:
 *
 *   - `record` (src/shortcut-recorder.ts) turns ONE key press into a
 *     combination, or into the reason it cannot be one. What a modifier press
 *     means, what Escape means, which keys may end a combination: all there.
 *   - `outcomeOf` (same file) turns the backend's answer into what the field
 *     must now show and say — including the row this whole lot exists for, a
 *     refused combination with the previous one registered again.
 *
 * The startup read is the same shape: `hintFor` (src/shortcut-hint.ts) already
 * decides, from the registry and from what Windows answered, whether a
 * combination may honestly be drawn. This screen asks the same two commands the
 * launcher asks, and reads the same answer.
 *
 * # THE MARKUP IS THE MAQUETTE'S, class for class
 *
 * `src/design/Showcase.tsx`, section `s-recorder`: a `.c-field` holding a
 * `.c-field__label`, a `button.c-btn.c-btn--secondary.c-recorder`, and a
 * `.c-field__message` under it. Same two deliberate differences the other
 * screens carry, plus one addition:
 *
 *   1. The screen name is an `<h1>`, not the showcase's `<h2>`: it is the first
 *      heading of a window, and a document whose outline starts at level 2 is a
 *      document a screen reader cannot walk. `.c-screen__name` is unchanged.
 *   2. `aria-describedby` is drawn for EVERY message, where the showcase draws
 *      it only for the error specimens. The specimens are separate drawings with
 *      ids of their own; here there is one message box, and « Échap annule. » is
 *      exactly the sentence a user who cannot see the field needs.
 *   3. `aria-labelledby` names the label AND the button itself, so the control
 *      announces « Raccourci de capture, Ctrl + Maj + 2, Modifier » instead of
 *      « Modifier » alone. The label is a `<span>` in the maquette and stays
 *      one — a `<label>` would be pointing at a button, which is not a form
 *      control.
 *
 * `aria-invalid` follows the maquette exactly, and the interesting case is the
 * one where it is ABSENT: « pris, mais pas écrit » describes a combination that
 * works and a file that could not be written. The value is not invalid; the
 * message says what happened, and the control does not claim otherwise.
 *
 * # THREE THINGS THIS SCREEN DOES NOT DO, said here because a screen that looks
 * # finished is where an unfinished thing hides
 *
 *   - A `set_capture_shortcut` that REJECTS is written to the console and to
 *     nothing else, and the field goes back to what it was showing. Rust rejects
 *     only for a combination `record` has already ruled out, or for a refused
 *     ACL; the maquette has no wording for either, and this file may not invent
 *     any — every sentence on this screen comes from `src/strings.ts`, and every
 *     value there is the showcase's. Same rule, and same gap, as the launcher's
 *     `capture_region`.
 *   - The combination that is CURRENTLY registered still reaches the operating
 *     system while the recorder is listening. `preventDefault` stops the webview
 *     from acting on a press; it cannot stop a global hotkey Windows itself owns.
 *     Pressing the active combination while recording therefore starts a capture.
 *     Freeing it for the duration would take a backend command that does not
 *     exist. NOT OBSERVED IN A RUNNING WINDOW - it follows from what a global
 *     shortcut is, and it is written as the expectation it is.
 *   - While the recorder listens, every key press belongs to it, Tab included.
 *     Escape gives the keyboard back, which is what the message says.
 */

import { useEffect, useState } from 'react';

import Keys from './Keys';
import { Glyph, ICON } from './design/Glyph';
import { hintFor } from './shortcut-hint';
import type { ShortcutHint } from './shortcut-hint';
import { drawn, outcomeOf, record } from './shortcut-recorder';
import type { RecorderNote, RefusalKey } from './shortcut-recorder';
import { describeShortcutStatus, describeShortcuts, setCaptureShortcut } from './shortcuts';
import { UI_STRINGS } from './strings';

import './design/components.css';

/** The gap between the screen's blocks, as the maquette spends it. */
const BLOCK_GAP = { marginBlockStart: 'var(--space-5)' } as const;

/* The three ids the field is wired with. The button names ITSELF as well as its
   label, which is the documented way to keep the combination in the accessible
   name instead of replacing it with the label. */
const LABEL_ID = 'capture-shortcut-label';
const BUTTON_ID = 'capture-shortcut';
const MESSAGE_ID = 'capture-shortcut-message';
const RECORDER_LABELLING = `${LABEL_ID} ${BUTTON_ID}`;

/**
 * What the field says, at rest.
 *
 * `RecorderNote` is the answer to a change. `none` is the one state no change
 * can produce: this application never got a combination registered at all, so
 * there is nothing to name as the one that was refused.
 */
type Note = RecorderNote | { readonly state: 'none' };

/** The combination that answers today, or `undefined` when none does. */
type Active = readonly string[] | undefined;

/**
 * Where the recorder stands.
 *
 * Flat and discriminated on `phase`, so that a field cannot be both listening
 * and waiting for an answer, and so that a note cannot be carried by a state
 * that has nothing to report yet.
 */
type Field =
  /** The startup read is in flight. The field reserves its box. */
  | { readonly phase: 'reading' }
  /** Nothing could be read, so nothing is claimed and nothing can be set. */
  | { readonly phase: 'unreadable' }
  /** At rest: what answers today, and what last happened. */
  | { readonly phase: 'idle'; readonly active: Active; readonly note: Note }
  /** Armed: the next press is read as a combination. */
  | {
      readonly phase: 'listening';
      readonly active: Active;
      /** The last press this application refused on its own, if there was one. */
      readonly refusal: RefusalKey | undefined;
    }
  /** A combination has been offered to the system; its answer is in flight. */
  | { readonly phase: 'offering'; readonly active: Active };

/**
 * The field the startup read leaves behind.
 *
 * `refused` becomes the `stranded` note on purpose: Windows turned the
 * combination down, nothing is registered, and both halves of that sentence are
 * already catalogued. `unavailable` becomes `none` because there is no
 * combination anyone may be told was refused.
 *
 * THIS IS THE ONE DERIVATION THIS FILE MAKES, and it is the one that ought to
 * live in `shortcut-recorder.ts` next to a test. It is here because this run was
 * to add no test, and an untested function in that file would quietly break the
 * rule its header states.
 */
function fieldFrom(hint: ShortcutHint): Field {
  switch (hint.state) {
    case 'ready':
      return { phase: 'idle', active: hint.keys, note: { state: 'settled' } };
    case 'refused':
      return { phase: 'idle', active: undefined, note: { state: 'stranded', refused: hint.keys } };
    case 'unavailable':
      return { phase: 'idle', active: undefined, note: { state: 'none' } };
    // `loading` cannot arrive here: `hintFor` returns it only for a read still
    // in flight, and this is called on one that came back. It is named rather
    // than left to a `default`, so the day a sixth state is added this switch
    // stops compiling instead of quietly claiming the registry is unreadable.
    case 'unreadable':
    case 'loading':
      return { phase: 'unreadable' };
  }
}

export default function Settings() {
  const [field, setField] = useState<Field>({ phase: 'reading' });

  useEffect(() => {
    // StrictMode runs effects twice in development, so both commands are asked
    // twice there. That is the dev double-render, not a bug.
    let abandoned = false;

    // Both or neither, for the reason the launcher gives: what the field shows
    // is drawn from the table AND from what the system answered about it, and a
    // screen holding one of the two would have to guess the other.
    Promise.all([describeShortcuts(), describeShortcutStatus()]).then(
      ([entries, registration]) => {
        if (abandoned) return;

        if (registration.status !== 'accepted') {
          // The operating system's own words, or this application's: English,
          // technical, and useful to exactly one reader. The screen says what
          // the user can act on; this puts the rest where it belongs.
          console.warn(
            `[cliche] settings: the capture shortcut is ${registration.status}`,
            registration.reason,
          );
        }
        setField(fieldFrom(hintFor({ status: 'read', entries, registration })));
      },
      (error: unknown) => {
        if (!abandoned) {
          console.error('[cliche] settings: the shortcut registry could not be read', error);
          setField({ phase: 'unreadable' });
        }
      },
    );

    return () => {
      abandoned = true;
    };
  }, []);

  const listening = field.phase === 'listening';

  useEffect(() => {
    if (!listening) return undefined;

    const onKeyDown = (event: KeyboardEvent) => {
      // Every press belongs to the recorder while it is armed, Tab and Space
      // included: a Space that reached the button would re-fire the click that
      // armed it, and a Tab would take the keyboard away mid-recording.
      event.preventDefault();

      const press = record(event);

      if (press.kind === 'holding') {
        // Modifiers on the way to a combination. Not a refusal, and not a
        // reason to clear one either: the message stands until a real press
        // replaces it.
        return;
      }

      if (press.kind === 'cancelled') {
        setField((current) =>
          current.phase === 'listening'
            ? { phase: 'idle', active: current.active, note: { state: 'settled' } }
            : current,
        );
        return;
      }

      if (press.kind === 'refused') {
        setField((current) =>
          current.phase === 'listening' ? { ...current, refusal: press.why } : current,
        );
        return;
      }

      setField((current) =>
        current.phase === 'listening' ? { phase: 'offering', active: current.active } : current,
      );

      setCaptureShortcut(press.accelerator).then(
        (change) => {
          const outcome = outcomeOf(change);
          // Guarded on the phase rather than on a flag: the user may have armed
          // the recorder again while this was in flight, and an answer about the
          // press before theirs must not overwrite what they are doing now.
          setField((current) =>
            current.phase === 'offering'
              ? { phase: 'idle', active: outcome.active, note: outcome.note }
              : current,
          );
        },
        (error: unknown) => {
          // The header says why nothing is drawn for this: there is no wording
          // for it, and this file may not invent one. The field goes back to the
          // combination that was answering before the press.
          console.error('[cliche] settings: the combination could not be offered', error);
          setField((current) =>
            current.phase === 'offering'
              ? { phase: 'idle', active: current.active, note: { state: 'settled' } }
              : current,
          );
        },
      );
    };

    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
    };
  }, [listening]);

  const active =
    field.phase === 'reading' || field.phase === 'unreadable' ? undefined : field.active;
  const note = field.phase === 'idle' ? field.note : undefined;
  const refusal = field.phase === 'listening' ? field.refusal : undefined;

  /* A message is drawn in every state but one: at rest with nothing to report. */
  const described =
    listening || field.phase === 'unreadable' || (note !== undefined && note.state !== 'settled');

  /* `unsaved` and `none` are deliberately NOT invalid: the first describes a
     combination that works, the second describes no combination at all. Only a
     press this application refused, or one the system did, is a value the
     control is holding wrongly. */
  const invalid =
    refusal !== undefined ||
    (note !== undefined && (note.state === 'kept' || note.state === 'stranded'));

  return (
    <>
      <h1 className="c-screen__name">{UI_STRINGS.settingsTitle}</h1>

      <div className="c-field" style={BLOCK_GAP}>
        <span className="c-field__label" id={LABEL_ID}>
          {UI_STRINGS.shortcutFieldLabel}
        </span>

        <button
          type="button"
          id={BUTTON_ID}
          className="c-btn c-btn--secondary c-recorder"
          aria-labelledby={RECORDER_LABELLING}
          aria-describedby={described ? MESSAGE_ID : undefined}
          aria-pressed={listening}
          aria-invalid={invalid}
          aria-busy={field.phase === 'reading' || field.phase === 'offering'}
          disabled={field.phase === 'unreadable'}
          onClick={() => {
            setField((current) => {
              if (current.phase === 'listening') {
                return { phase: 'idle', active: current.active, note: { state: 'settled' } };
              }
              if (current.phase === 'idle') {
                return { phase: 'listening', active: current.active, refusal: undefined };
              }
              // `reading` and `offering` are round trips in flight, `unreadable`
              // is disabled: none of the three has anything to arm.
              return current;
            });
          }}
        >
          {listening ? (
            <span>{UI_STRINGS.shortcutListening}</span>
          ) : field.phase === 'reading' ? (
            /* The reserved keycap, as on the launcher: the box is held while the
               round trip is in flight, and it is held EMPTY. The combination
               belongs to the Rust registry and is not known yet, and drawing one
               here would be inventing it. `&nbsp;` and not a plain space -
               `.c-kbd.c-skeleton` paints its text out but still measures it, and
               ordinary whitespace would collapse and leave a chip with no line
               box. Written as the entity so the character is VISIBLE in this
               file; JSX decodes it the way it decodes the showcase's `&apos;`. */
            <>
              <span className="c-kbd c-skeleton" aria-hidden="true">
                &nbsp;
              </span>
              <span>{UI_STRINGS.shortcutChange}</span>
            </>
          ) : active === undefined ? (
            <span>{UI_STRINGS.shortcutUnavailable}</span>
          ) : (
            <>
              <Keys keys={active} />
              <span>{UI_STRINGS.shortcutChange}</span>
            </>
          )}
        </button>

        {listening && refusal === undefined && (
          <span className="c-field__message" id={MESSAGE_ID}>
            {UI_STRINGS.shortcutEscapeCancels}
          </span>
        )}

        {refusal !== undefined && (
          <span className="c-field__message c-field__message--error" id={MESSAGE_ID}>
            <Glyph d={ICON.alert} />
            {UI_STRINGS[refusal]}
          </span>
        )}

        {/* The maquette draws this one WITHOUT the error colour and without a
            glyph: nothing was refused, the screen simply has nothing to read. */}
        {field.phase === 'unreadable' && (
          <span className="c-field__message" id={MESSAGE_ID}>
            {UI_STRINGS.shortcutRegistryUnreadable}
          </span>
        )}

        {note !== undefined && note.state === 'unsaved' && (
          <span className="c-field__message c-field__message--error" id={MESSAGE_ID}>
            <Glyph d={ICON.alert} />
            {UI_STRINGS.shortcutNotSaved}
          </span>
        )}

        {/* `kept` and `stranded` share their first half - the combination that
            was refused, named - and differ only on what is left. Drawn as one
            branch because the difference is exactly `active`: the combination
            that answers now, or none. */}
        {note !== undefined && (note.state === 'kept' || note.state === 'stranded') && (
          <span className="c-field__message c-field__message--error" id={MESSAGE_ID}>
            <Glyph d={ICON.alert} />
            {drawn(note.refused)} {UI_STRINGS.shortcutAlreadyTaken}{' '}
            {active === undefined
              ? UI_STRINGS.shortcutNoneActive
              : `${drawn(active)} ${UI_STRINGS.shortcutPreviousStillActive}`}
          </span>
        )}

        {note !== undefined && note.state === 'none' && (
          <span className="c-field__message c-field__message--error" id={MESSAGE_ID}>
            <Glyph d={ICON.alert} />
            {UI_STRINGS.shortcutNoneActive}
          </span>
        )}
      </div>
    </>
  );
}
