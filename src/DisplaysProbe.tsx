/*
 * The monitor list, as the backend reports it at startup.
 *
 * # WHY THIS IS A COMPONENT OF ITS OWN, AND WHERE IT ENDED UP
 *
 * It was the whole of `App.tsx` until 6 September 2026, when the launcher took
 * that place. It spent the rest of that day behind `#/diagnostic`, a route whose
 * only job was to keep `describe_displays` called by something a person could
 * open - which is how a probe stops working without anyone noticing.
 *
 * It is now the diagnostic section of `Help.tsx`, and `#/diagnostic` is GONE.
 * One way in, not two: the second was reachable only by typing a URL into a
 * window that has no address bar, and two doors into one read-out is how the
 * two come to disagree.
 *
 * # IT SPOKE ENGLISH UNTIL 6 SEPTEMBER 2026
 *
 * This header used to argue that the wording belonged outside `src/strings.ts`
 * because "this is an engineering read-out, not interface prose". That was true
 * of a screen nobody but a developer could open, and it stopped being true the
 * day the read-out landed inside the help page - in French, under a French
 * heading, one block below the key map. A section in another language is not a
 * technical read-out, it is a page somebody translated halfway.
 *
 * The same header named the way out, and it is the way that was taken: the
 * sentences were decided in `src/design/Showcase.tsx` first (section
 * `s-diagnostic`), then catalogued, then used here. Not one of them is typed in
 * this file.
 *
 * What stays in its own language is the message the SYSTEM returns on a failure.
 * That is a quotation - `xcap`'s own words, reaching this component through the
 * command's `Err(String)` - and translating a quotation is inventing one.
 *
 * The outer `<main class="app">` that used to wrap this went with the launcher:
 * the padding is now `.c-shell__body`'s, and spending `--gutter` twice was the
 * only thing that wrapper still did.
 */

import { useEffect, useState } from 'react';

import { Glyph, ICON } from './design/Glyph';
import { describeDisplays } from './displays';
import type { DisplayInfo } from './displays';
import { UI_STRINGS } from './strings';

import './design/components.css';

type Probe =
  | { readonly status: 'probing' }
  | { readonly status: 'ready'; readonly displays: readonly DisplayInfo[] }
  | { readonly status: 'failed'; readonly message: string };

function toMessage(error: unknown): string {
  // A rejected `invoke` carries the `Err(String)` returned by the command, not
  // an Error instance, so `error.message` would be undefined here.
  if (error instanceof Error) {
    return error.message;
  }
  return typeof error === 'string' ? error : JSON.stringify(error);
}

export default function DisplaysProbe() {
  const [probe, setProbe] = useState<Probe>({ status: 'probing' });

  useEffect(() => {
    // StrictMode runs effects twice in development, so `describe_displays` is
    // logged twice in the terminal. That is the dev double-render, not a bug.
    let abandoned = false;

    describeDisplays().then(
      (displays) => {
        if (!abandoned) {
          setProbe({ status: 'ready', displays });
        }
      },
      (error: unknown) => {
        if (!abandoned) {
          setProbe({ status: 'failed', message: toMessage(error) });
        }
      },
    );

    return () => {
      abandoned = true;
    };
  }, []);

  return (
    <section aria-labelledby="displays-heading">
      <h2 id="displays-heading">{UI_STRINGS.displaysHeading}</h2>
      {probe.status === 'probing' && <p role="status">{UI_STRINGS.displaysReading}</p>}

      {/* PRD A4: the red is the THIRD cue, never the first. The word
          « Échec » and the alert glyph carry the state on their own, which is
          what .c-note--danger is built for — same component the showcase
          publishes at #/systeme. A bare red sentence was colour alone.

          `probe.message` is the only thing on this screen that is not French:
          it is what the system answered, verbatim. See the header. */}
      {probe.status === 'failed' && (
        <div role="alert" className="c-note c-note--danger">
          <Glyph d={ICON.alert} />
          <span>
            <strong>{UI_STRINGS.failure}</strong>
            {' — '}
            {UI_STRINGS.displaysUnreadable} {probe.message}
          </span>
        </div>
      )}

      {probe.status === 'ready' && (
        <>
          {/* Two labels and not one with a « (s) »: a parenthesis in an
              interface is a sentence nobody finished writing. */}
          <p role="status">
            {probe.displays.length}{' '}
            {probe.displays.length === 1 ? UI_STRINGS.displayOne : UI_STRINGS.displayMany}
          </p>
          <ul className="displays">
            {probe.displays.map((display) => (
              <li key={`${display.name}@${display.x},${display.y}`} className="display">
                <span className="display-name">{display.name || UI_STRINGS.displayUnnamed}</span>
                <span className="display-facts">
                  {display.width}×{display.height} {UI_STRINGS.displayPhysicalPixels} ·{' '}
                  {UI_STRINGS.displayOrigin} ({display.x}, {display.y}) ·{' '}
                  {UI_STRINGS.displayScale} {display.scaleFactor}
                </span>
              </li>
            ))}
          </ul>
        </>
      )}
    </section>
  );
}
