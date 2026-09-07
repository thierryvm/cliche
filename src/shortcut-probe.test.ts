/**
 * The startup read's own decisions, under test.
 *
 * Same rule as `window-controls.test.ts` and `shortcut-hint.test.ts`: no jsdom,
 * no testing-library, so what is worth testing takes its arguments and the
 * component only spends the answer. Here that includes the LOOP - `readRegistry`
 * is handed an `ask` and a `later`, so "asks again, and stops asking" is a test
 * rather than a hope, and no test waits a quarter of a second for anything.
 *
 * WHAT THIS CANNOT REACH, said so the green is not read as more than it is:
 * that `Launcher.tsx` and `Settings.tsx` really hand `describeShortcuts` and
 * `describeShortcutStatus` to `ask`, that their `later` is a `setTimeout` they
 * clear on unmount, and that a component still mounted is what receives the
 * answer. That half needs the Tauri webview and nothing below observes it.
 */

import { describe, expect, it } from 'vitest';

import { hintFor } from './shortcut-hint';
import {
  ASK_AGAIN_AFTER_MS,
  ASK_AGAIN_AT_MOST,
  nextAsk,
  reasonText,
  readRegistry,
  waiting,
} from './shortcut-probe';
import type { Answer, Later } from './shortcut-probe';
import type { ShortcutEntry, ShortcutStatus } from './shortcuts';

/** The registry row the capture tile shares its combination with. */
const CAPTURE: ShortcutEntry = {
  id: 'capture-region',
  accelerator: 'Ctrl+Shift+Digit2',
  keys: ['Ctrl', 'Maj', '2'],
  descriptionKey: 'captureRegion',
  category: 'capture',
};

/**
 * The row `describe_shortcuts` hands back before `install` has run: the one the
 * SOURCE ships with, which the user's settings file may well replace.
 */
const SHIPPED: ShortcutEntry = {
  ...CAPTURE,
  accelerator: 'Ctrl+Alt+F9',
  keys: ['Ctrl', 'Alt', 'F9'],
};

const STARTING: ShortcutStatus = { status: 'starting' };
const ACCEPTED: ShortcutStatus = { status: 'accepted', accelerator: 'Ctrl+Shift+Digit2' };

/** An answer that is still nothing: the table as it stands, and no decision. */
const NOT_YET: Answer = [[SHIPPED], STARTING];
/** An answer that decides: the table as `install` left it, and a yes. */
const DECIDED: Answer = [[CAPTURE], ACCEPTED];

/**
 * An `ask` that hands back the given answers in order, repeating the last one
 * for ever, and counts how many times it was called.
 */
function asking(answers: readonly Answer[]): {
  readonly ask: () => Promise<Answer>;
  readonly made: () => number;
} {
  let made = 0;

  return {
    ask: () => {
      const answer = answers[made] ?? answers[answers.length - 1] ?? NOT_YET;
      made += 1;
      return Promise.resolve(answer);
    },
    made: () => made,
  };
}

/** A `later` that runs at once and keeps the delays it was asked for. */
function immediately(): { readonly later: Later; readonly delays: readonly number[] } {
  const delays: number[] = [];

  return {
    later: (run, inMs) => {
      delays.push(inMs);
      run();
    },
    delays,
  };
}

describe('nextAsk', () => {
  it('stops on an answer that decides something', () => {
    expect(nextAsk(ACCEPTED, ASK_AGAIN_AT_MOST)).toEqual({
      next: 'settled',
      registration: ACCEPTED,
    });
  });

  it('stops on a refusal too - a decision is a decision, whichever way it went', () => {
    // The three decided answers must all END the read. Only `starting` is « pas
    // encore »; asking again about a combination Windows has already refused
    // would be asking a question that has been answered.
    const refused: ShortcutStatus = {
      status: 'refused-by-system',
      accelerator: 'Ctrl+Shift+Digit2',
      reason: 'HotKey already registered',
    };
    const never: ShortcutStatus = { status: 'not-attempted', reason: 'the plugin failed to load' };

    expect(nextAsk(refused, ASK_AGAIN_AT_MOST).next).toBe('settled');
    expect(nextAsk(never, ASK_AGAIN_AT_MOST).next).toBe('settled');
  });

  it('asks again while the backend is starting, and spends one ask doing it', () => {
    // The decrement is the whole of the bound: without it the two arms below
    // are the same arm, and the launcher polls a dead `setup` for ever.
    expect(nextAsk(STARTING, 3)).toEqual({
      next: 'ask-again',
      inMs: ASK_AGAIN_AFTER_MS,
      asksLeft: 2,
    });
  });

  it('gives up when the asks have run out, and says why', () => {
    const step = nextAsk(STARTING, 0);

    expect(step.next).toBe('give-up');

    // The reason is what the note QUOTES, so an empty one would put « Échec — »
    // on the screen with nothing after it. It names the budget it spent,
    // because "it did not answer" and "it did not answer in three seconds" are
    // not the same report to whoever reads it.
    //
    // Read out before the assertion rather than guarded by an `if`: a branch
    // here would SKIP the assertion on the day the arm above stops firing, and
    // a test that skips its own point passes for the wrong reason.
    const reason = step.next === 'give-up' ? step.reason : '';

    expect(reason).toContain(String(ASK_AGAIN_AT_MOST * ASK_AGAIN_AFTER_MS));
  });

  it('spends a bounded number of asks over a few seconds, and not more', () => {
    // The numbers themselves, held against the reasoning next to them: what is
    // being waited for is a `setup` that builds a WebView2 window, "a few
    // hundred milliseconds" by its own comment. A budget under a second would
    // give up on a cold start; one over ten would be a spinner.
    expect(ASK_AGAIN_AT_MOST * ASK_AGAIN_AFTER_MS).toBeGreaterThanOrEqual(1000);
    expect(ASK_AGAIN_AT_MOST * ASK_AGAIN_AFTER_MS).toBeLessThanOrEqual(10000);
  });
});

describe('reasonText', () => {
  it('takes the message of an Error and not the whole object', () => {
    expect(reasonText(new Error('window main is not allowed'))).toBe('window main is not allowed');
  });

  it('hands a string back as it arrived, which is what an invoke rejects with', () => {
    // `invoke` rejects with the command's `Err(String)`, not with an Error - so
    // this is the case that actually happens on a refused ACL.
    expect(reasonText('describe_shortcut_status is not allowed here')).toBe(
      'describe_shortcut_status is not allowed here',
    );
  });

  it('writes out an object rather than stringifying it to [object Object]', () => {
    expect(reasonText({ code: 'ACL', detail: 'refused' })).toBe(
      '{"code":"ACL","detail":"refused"}',
    );
  });

  it('says nothing rather than throwing on an object that cannot be written out', () => {
    // The argument is `unknown`, and this runs inside the rejection handler of
    // the read: a throw here would reject the promise the screen is waiting on,
    // and the launcher would sit on « lecture… » for ever - the exact failure
    // this whole lot is about, reached from the other side.
    const loop: { self?: unknown } = {};
    loop.self = loop;

    expect(reasonText(loop)).toBe('');
  });

  it('says nothing at all when there was nothing to say', () => {
    // The empty case is not decoration: `JSON.stringify(undefined)` is the
    // value `undefined`, and a screen that drew it would print « undefined »
    // after a French sentence. '' is what `hintFor` reads as "quote nothing".
    expect(reasonText(undefined)).toBe('');
    expect(reasonText(null)).toBe('');
  });
});

describe('readRegistry', () => {
  it('asks once when the first answer already decides', async () => {
    const { ask, made } = asking([DECIDED]);
    const { later, delays } = immediately();

    expect(await readRegistry(ask, later)).toEqual({
      status: 'read',
      entries: [CAPTURE],
      registration: ACCEPTED,
    });
    expect(made()).toBe(1);
    expect(delays).toEqual([]);
  });

  it('asks again while the backend is starting, and settles on the answer that decides', async () => {
    const { ask, made } = asking([NOT_YET, NOT_YET, DECIDED]);
    const { later, delays } = immediately();

    const read = await readRegistry(ask, later);

    expect(read.status).toBe('read');
    expect(made()).toBe(3);
    expect(delays).toEqual([ASK_AGAIN_AFTER_MS, ASK_AGAIN_AFTER_MS]);
  });

  it('keeps the TABLE of the ask that decided, not the one it started with', async () => {
    // `describe_shortcuts` answers from the moment the window exists, and until
    // `install` has run it publishes the combination the source ships with. A
    // read that kept the first table would draw a combination this launch never
    // offered - the very thing `shortcut-hint.ts` refuses to do.
    const { ask } = asking([NOT_YET, DECIDED]);
    const { later } = immediately();

    const read = await readRegistry(ask, later);

    expect(read.status === 'read' && read.entries).toEqual([CAPTURE]);
  });

  it('gives up after a bounded number of asks rather than loading for ever', async () => {
    const { ask, made } = asking([NOT_YET]);
    const { later } = immediately();

    const read = await readRegistry(ask, later);

    expect(read.status).toBe('unreadable');
    expect(made()).toBe(ASK_AGAIN_AT_MOST + 1);
  });

  it('carries the reason it gave up, so the note can say more than « échec »', async () => {
    const { ask } = asking([NOT_YET]);
    const { later } = immediately();

    const read = await readRegistry(ask, later);

    expect(read.status === 'unreadable' && read.reason).not.toBe('');
  });

  it('turns a rejection into an unreadable read that quotes what rejected', async () => {
    // The path that drew a bare red note until 7 September 2026: the message was
    // written to the console and thrown away.
    const read = await readRegistry(
      () => Promise.reject(new Error('window main is not allowed')),
      immediately().later,
    );

    expect(read).toEqual({ status: 'unreadable', reason: 'window main is not allowed' });
  });

  it('hands the reason all the way to the note the screen draws', async () => {
    // The two halves together, and the one property Thierry asked for on
    // 7 September 2026: what rejected must reach the banner, not the console
    // alone. Everything between the rejection and the drawn sentence is here -
    // only the JSX is not, and the JSX has no decision left in it.
    const read = await readRegistry(
      () => Promise.reject('window main is not allowed'),
      immediately().later,
    );

    expect(hintFor(read)).toEqual({
      state: 'unreadable',
      sentenceKey: 'shortcutRegistryUnreadableWithReason',
      quotation: ' window main is not allowed',
    });
  });

  it('does not ask again after a rejection: that is an answer, and a final one', async () => {
    let made = 0;
    const read = await readRegistry(
      () => {
        made += 1;
        return Promise.reject('nope');
      },
      immediately().later,
    );

    expect(read.status).toBe('unreadable');
    expect(made).toBe(1);
  });
});

describe('the timer a screen owns while it waits', () => {
  /** A fake pair of timer calls, so nothing here waits and everything is counted. */
  function fakeTimers() {
    const armed: { handle: number; run: () => void; inMs: number }[] = [];
    const cleared: number[] = [];
    let next = 1;

    return {
      armed,
      cleared,
      timers: {
        set: (run: () => void, inMs: number) => {
          const handle = next;
          next += 1;
          armed.push({ handle, run, inMs });
          return handle;
        },
        clear: (handle: number) => {
          cleared.push(handle);
        },
      },
    };
  }

  it('arms nothing once the screen has gone', () => {
    // THE DEFECT THIS TEST EXISTS FOR, found in review on 7 September 2026.
    // `readRegistry` calls `later` when its ask ANSWERS, which can be long
    // after the cleanup ran. Before the guard, that call armed a timer the
    // cleanup could not have cancelled - it had already run - and the screen
    // went on asking the backend up to twelve more times for nobody.
    const { armed, timers } = fakeTimers();
    const wait = waiting(timers);

    wait.stop();
    wait.later(() => {}, ASK_AGAIN_AFTER_MS);

    expect(armed).toEqual([]);
  });

  it('cancels a timer that was already armed when the screen went', () => {
    const { armed, cleared, timers } = fakeTimers();
    const wait = waiting(timers);

    wait.later(() => {}, ASK_AGAIN_AFTER_MS);
    wait.stop();

    expect(armed).toHaveLength(1);
    expect(cleared).toEqual([armed[0]?.handle]);
  });

  it('says whether the screen has gone, so one flag answers for everything', () => {
    // The read guards its own `setState` on this. Two flags that could disagree
    // is the shape the defect above came in.
    const wait = waiting(fakeTimers().timers);

    expect(wait.abandoned()).toBe(false);
    wait.stop();
    expect(wait.abandoned()).toBe(true);
  });

  it('strands no handle if a second wait is asked for before the first fires', () => {
    // Cannot happen today - `readRegistry` waits for its answer before
    // postponing again - so this holds a guard, not a repair.
    const { armed, cleared, timers } = fakeTimers();
    const wait = waiting(timers);

    wait.later(() => {}, ASK_AGAIN_AFTER_MS);
    wait.later(() => {}, ASK_AGAIN_AFTER_MS);

    expect(armed).toHaveLength(2);
    expect(cleared).toEqual([armed[0]?.handle]);
  });

  it('stops twice without cancelling a handle twice', () => {
    // React can run a cleanup more than once in StrictMode. Clearing a handle
    // that is already cleared is harmless in the browser, but a second entry
    // here would mean this object forgot it had stopped.
    const { cleared, timers } = fakeTimers();
    const wait = waiting(timers);

    wait.later(() => {}, ASK_AGAIN_AFTER_MS);
    wait.stop();
    wait.stop();

    expect(cleared).toHaveLength(1);
  });
});
