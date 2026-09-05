/*
 * The monitor list, as the backend reports it at startup.
 *
 * # WHY THIS IS A COMPONENT OF ITS OWN, AND WHERE IT IS GOING
 *
 * It was the whole of `App.tsx` until 6 September 2026, when the launcher took
 * that place. It is NOT dead code and was not deleted: it is the diagnostic
 * section of the help page, which does not exist yet. Until it does, it is
 * mounted behind `#/diagnostic` so that `describe_displays` keeps being called
 * by something a person can open, rather than sitting in a file nothing
 * imports - which is how a probe stops working without anyone noticing.
 *
 * The wording is deliberately still English and deliberately NOT in
 * `src/strings.ts`: this is an engineering read-out, and the day it becomes a
 * section of the help page it gets French sentences decided in the showcase
 * first, like everything else in that catalogue.
 *
 * The outer `<main class="app">` that used to wrap this went with the launcher:
 * the padding is now `.c-shell__body`'s, and spending `--gutter` twice was the
 * only thing that wrapper still did.
 */

import { useEffect, useState } from 'react';

import { Glyph, ICON } from './design/Glyph';
import { describeDisplays } from './displays';
import type { DisplayInfo } from './displays';

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
      <h2 id="displays-heading">Displays detected at startup</h2>
      {probe.status === 'probing' && <p role="status">Reading the monitor list…</p>}

      {/* PRD A4: the red is the THIRD cue, never the first. The word
          "Failed" and the alert glyph carry the state on their own, which is
          what .c-note--danger is built for — same component the showcase
          publishes at #/systeme. A bare red sentence was colour alone. */}
      {probe.status === 'failed' && (
        <div role="alert" className="c-note c-note--danger">
          <Glyph d={ICON.alert} />
          <span>
            <strong>Failed</strong> — the monitor list could not be read: {probe.message}
          </span>
        </div>
      )}

      {probe.status === 'ready' && (
        <>
          <p role="status">
            {probe.displays.length} display{probe.displays.length === 1 ? '' : 's'}
          </p>
          <ul className="displays">
            {probe.displays.map((display) => (
              <li key={`${display.name}@${display.x},${display.y}`} className="display">
                <span className="display-name">{display.name || '(unnamed)'}</span>
                <span className="display-facts">
                  {display.width}×{display.height} physical px · origin ({display.x},{' '}
                  {display.y}) · scale {display.scaleFactor}
                </span>
              </li>
            ))}
          </ul>
        </>
      )}
    </section>
  );
}
