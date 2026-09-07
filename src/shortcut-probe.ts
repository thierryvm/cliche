/**
 * The startup read, and the one thing it has to decide: whether to ask again.
 *
 * # WHY THIS FILE EXISTS - 7 SEPTEMBER 2026
 *
 * Tauri builds the windows declared in `tauri.conf.json` BEFORE it calls our
 * `setup` (`tauri-2.11.5/src/app.rs:2521-2535`), so the launcher boots React and
 * calls `describe_shortcut_status` while `setup` is still enumerating monitors
 * and building the veil's WebView2. Rust now answers that honestly - `starting`,
 * a fourth status that means « pas encore » - instead of failing, and the screen
 * draws it as « en cours » instead of a red note over a shortcut that works.
 *
 * That fix is only half an answer: an application that says « en cours » and is
 * never asked again says « en cours » for ever. This file is the other half, and
 * it is bounded on purpose - see [`ASK_AGAIN_AT_MOST`].
 *
 * # NOTHING HERE TOUCHES REACT, AND THAT IS THE DESIGN
 *
 * Same rule as `window-controls.ts` states in its own header: this repository
 * has no jsdom and no testing-library and buys neither, so whatever is worth a
 * test is a function of its arguments and lives outside the component. The LOOP
 * is included in that - `readRegistry` takes the way it asks and the way it
 * waits as arguments, which is how "it stops asking" became a test instead of a
 * hope. It is the shape `shortcut::change` uses in Rust for the same reason: the
 * registrar is an argument, so the refusal path can be exercised.
 *
 * The two things the components keep, because only they can hold them: the
 * `abandoned` flag that stops a `setState` after unmount, and the timer id that
 * cleanup clears.
 */

import type { RegistryRead } from './shortcut-hint';
import type { ShortcutEntry, ShortcutStatus } from './shortcuts';

/**
 * How long to wait before asking the backend again.
 *
 * WHAT IS BEING WAITED FOR is the rest of `setup`: reading the settings file,
 * building the veil's WebView2 window, then `shortcut::install`. Its own comment
 * in `src-tauri/src/shortcut.rs` calls that "the few hundred milliseconds", and
 * that figure is READ, not measured from here - see the note at the foot of this
 * block.
 *
 * A quarter of a second against that: long enough that an ordinary start is
 * decided by the first or second ask rather than by a burst of IPC calls, short
 * enough that the skeleton keycap is not read as a screen that has stopped.
 */
export const ASK_AGAIN_AFTER_MS = 250;

/**
 * How many times the read may be asked again before the screen gives up.
 *
 * Twelve, so the wait is AT LEAST 3 000 ms - twelve gaps of
 * [`ASK_AGAIN_AFTER_MS`], plus whatever the round trips themselves cost. A cold
 * first launch pays for WebView2 warming up and for whatever the machine's
 * antivirus makes of a new binary, several times what a warm one costs; three
 * seconds is the far side of that. Past it, a `setup` that still has not decided
 * is more likely stuck than slow, and saying so is more useful than a spinner.
 *
 * WHAT THIS BOUND DOES NOT COVER, said rather than left to be assumed: it bounds
 * the number of ASKS, not the time any one of them may take. A single `invoke`
 * that never answers would leave the count untouched and the screen reading
 * « en cours » for ever. Nothing observed does that - an early call answers
 * quickly, with `starting` or with a rejection - so no per-ask deadline is
 * bought here; the day one is needed, this is the paragraph that says why.
 *
 * WHY BOUNDED AT ALL. An unbounded poll would turn a `setup` that died into a
 * screen reading « lecture du registre des raccourcis… » for ever - the defect
 * this lot exists to remove, wearing a different colour. The give-up path says
 * what happened and quotes the budget it spent.
 *
 * NEITHER NUMBER IS MEASURED, and that is stated rather than left to be
 * discovered. What would measure them: print the interval between this window's
 * first `describe_shortcut_status` and the first one that answers anything but
 * `starting`, on a COLD start of the installed binary, twenty times. Until that
 * exists, these are two figures argued from a comment in another file.
 */
export const ASK_AGAIN_AT_MOST = 12;

/** A status that decides something - everything `starting` is not. */
export type DecidedStatus = Exclude<ShortcutStatus, { readonly status: 'starting' }>;

/**
 * A read that has SETTLED: an answer, or a failure. Never « still going ».
 *
 * `reading` is the state a screen is in before this function has finished, and
 * it is the component's to hold - it is not something the read can hand back.
 * Saying so in the type is what lets the caller narrow on `unreadable` and find
 * a registration on the other side of the `else`.
 */
export type SettledRead = Exclude<RegistryRead, { readonly status: 'reading' }>;

/**
 * What one ask brings back: the table this application intends to hold, and what
 * the system answered about the capture combination in it.
 *
 * Both, every time, and the second half is why: until `install` has run,
 * `describe_shortcuts` publishes the combination the SOURCE ships with (see
 * `rows` in `src-tauri/src/shortcuts.rs`), which the settings file may well
 * replace. A read that kept the first table and only re-asked the status could
 * draw a combination this launch never offered.
 */
export type Answer = readonly [readonly ShortcutEntry[], ShortcutStatus];

/** How the read reaches the backend. Injected, so a test can answer for it. */
export type Ask = () => Promise<Answer>;

/**
 * How the read waits. Injected for the same reason - and because the timer
 * belongs to the component, which is the only thing that knows it has been
 * unmounted.
 */
export type Later = (run: () => void, inMs: number) => void;

/** The two timer calls, injected so the waiting below can be tested. */
export interface Timers {
  readonly set: (run: () => void, inMs: number) => number;
  readonly clear: (handle: number) => void;
}

/** A component's side of the wait: how to postpone, how to stop, and whether it has. */
export interface Waiting {
  /** Hand this to [`readRegistry`]. It arms nothing once [`Waiting.stop`] has run. */
  readonly later: Later;
  /** Called from the effect's cleanup. Idempotent. */
  readonly stop: () => void;
  /** Whether the screen has gone. The ONE flag, so nothing can disagree with it. */
  readonly abandoned: () => boolean;
}

/**
 * The timer a screen owns while it waits for start-up to decide.
 *
 * # THE DEFECT THIS EXISTS FOR, found in review on 7 September 2026
 *
 * Both screens used to arm their timer inline, with a `pending` handle their
 * cleanup cleared. That is not enough, and the hole is exactly one ordering:
 *
 *   1. mount, `readRegistry` puts the FIRST ask in flight - no timer yet;
 *   2. the user leaves the screen before that ask answers, which is the normal
 *      case during the very seconds this whole lot is about. Cleanup runs,
 *      finds `pending === null`, and clears nothing;
 *   3. the ask answers `starting`, so `readRegistry` calls `later(...)` - the
 *      closure of an effect that is already torn down. A timer is armed AFTER
 *      the cleanup that would have cancelled it, and nothing will.
 *
 * Nothing crashed and no `setState` ran on a dead tree - the `abandoned` guard
 * covered that. What DID happen was up to twelve more real `invoke` calls on
 * behalf of a screen nobody is looking at. The comment above the old cleanup
 * claimed to prevent exactly that, which made it a false statement as well as a
 * defect.
 *
 * The remedy is that the flag and the timer live in ONE place and the arming
 * consults the flag. Written here rather than twice in two components, because
 * two copies of this ordering would drift the day one of them is edited.
 */
export function waiting(timers: Timers): Waiting {
  let stopped = false;
  let pending: number | null = null;

  return {
    later: (run, inMs) => {
      // THE LINE THE DEFECT WAS MISSING. An arm requested after `stop` is an
      // arm nobody will ever cancel.
      if (stopped) {
        return;
      }
      // A second arm before the first fired would strand the first handle. It
      // cannot happen today - `readRegistry` waits for its answer before
      // postponing again - so this is a guard, not a fix, and it costs one
      // comparison.
      if (pending !== null) {
        timers.clear(pending);
      }
      pending = timers.set(run, inMs);
    },
    stop: () => {
      stopped = true;
      if (pending !== null) {
        timers.clear(pending);
        pending = null;
      }
    },
    abandoned: () => stopped,
  };
}

/** What to do with the answer that just came back. */
export type NextAsk =
  /** It decided something. The status is handed on, narrowed. */
  | { readonly next: 'settled'; readonly registration: DecidedStatus }
  /** Nothing is decided yet, and there is still patience left. */
  | { readonly next: 'ask-again'; readonly inMs: number; readonly asksLeft: number }
  /** Nothing is decided and the patience is spent. `reason` says as much. */
  | { readonly next: 'give-up'; readonly reason: string };

/**
 * What the note quotes when `setup` never decided anything.
 *
 * English, technical, and this application's own words rather than the system's
 * - the same register as the `not-attempted` reasons Rust writes. It names the
 * budget because "it did not answer" and "it did not answer in three seconds"
 * are not the same report to whoever ends up reading it.
 *
 * Built from the two constants, so it cannot drift from what was actually
 * waited. It describes the budget [`readRegistry`] spends, which is the only
 * caller that starts a read with anything but those constants.
 */
const GAVE_UP =
  `the backend was still starting after ${ASK_AGAIN_AT_MOST} further asks ` +
  `${ASK_AGAIN_AFTER_MS} ms apart, so at least ` +
  `${ASK_AGAIN_AT_MOST * ASK_AGAIN_AFTER_MS} ms of waiting. ` +
  '`setup` did not finish, and nothing decided the capture shortcut';

/**
 * Whether to take this answer, ask again, or stop.
 *
 * `asksLeft` is spent by the caller and handed back decremented rather than
 * counted here: a decision that kept a count of its own would be a decision with
 * a memory, and this one has to be readable from its two arguments alone.
 */
export function nextAsk(registration: ShortcutStatus, asksLeft: number): NextAsk {
  if (registration.status !== 'starting') {
    // Accepted, refused or never attempted: all three are answers, and asking
    // again about a question that has been answered is how a screen comes to
    // contradict itself.
    return { next: 'settled', registration };
  }
  if (asksLeft > 0) {
    return { next: 'ask-again', inMs: ASK_AGAIN_AFTER_MS, asksLeft: asksLeft - 1 };
  }
  return { next: 'give-up', reason: GAVE_UP };
}

/**
 * Turns whatever a rejection carried into something a note can quote.
 *
 * A rejected `invoke` carries the command's `Err(String)` and not an `Error`
 * instance, which is why the string arm is the one that fires in practice; the
 * others are there because the argument is `unknown` and a screen may not print
 * `[object Object]` at a user.
 *
 * `''` means "there is nothing to quote", and it is a real answer rather than a
 * failure of this function: `hintFor` draws the form of the sentence that ends
 * in a full stop, and no separator is left hanging. Same rule, and the same
 * empty case, as `planFor` in `src/veil/confirmation.ts`.
 */
export function reasonText(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === 'string') {
    return error;
  }
  if (error === undefined || error === null) {
    return '';
  }

  try {
    // `JSON.stringify` is typed `string` and returns `undefined` for a function
    // or a symbol, neither of which the arms above catch.
    return JSON.stringify(error) ?? '';
  } catch {
    // It THROWS on a circular object, and on a BigInt. This runs inside the
    // rejection handler of the read: a throw here would reject the promise the
    // screen is waiting on, and the launcher would sit on « lecture… » for ever
    // - this lot's own defect, reached from the other side. Nothing is
    // swallowed that anyone could act on: what could not be written out cannot
    // be shown either.
    return '';
  }
}

/**
 * Reads the shortcut registry, asking again while the backend says it has not
 * decided, and stopping either way.
 *
 * Never rejects: every path ends in a `RegistryRead` the screen can draw, which
 * is what lets the caller be one `.then` with no failure branch of its own.
 */
export function readRegistry(ask: Ask, later: Later): Promise<SettledRead> {
  return new Promise<SettledRead>((resolve) => {
    const attempt = (asksLeft: number): void => {
      // `Promise.resolve().then(ask)` rather than `ask()`: an `ask` that threw
      // synchronously would otherwise reject the promise this function hands
      // back, and the screen would be left with no answer at all.
      Promise.resolve()
        .then(ask)
        .then(
          ([entries, registration]) => {
            const step = nextAsk(registration, asksLeft);

            if (step.next === 'ask-again') {
              later(() => attempt(step.asksLeft), step.inMs);
              return;
            }
            if (step.next === 'give-up') {
              resolve({ status: 'unreadable', reason: step.reason });
              return;
            }
            resolve({ status: 'read', entries, registration: step.registration });
          },
          (error: unknown) => {
            // A rejection IS an answer, and a final one: the ACL, the window
            // guard and a `setup` that never ran all reject the same way every
            // time, so asking again would only spend the budget to be told the
            // same thing twelve more times.
            resolve({ status: 'unreadable', reason: reasonText(error) });
          },
        );
    };

    attempt(ASK_AGAIN_AT_MOST);
  });
}
