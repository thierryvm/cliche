import { useEffect, useState } from 'react';

import { Glyph, ICON } from './design/Glyph';
import Showcase from './design/Showcase';
import { describeDisplays } from './displays';
import type { DisplayInfo } from './displays';

// This screen renders .c-note--danger, so it depends on the material layer
// directly. It used to arrive only because Showcase happens to import it —
// an accident that would break the day the showcase is lazy-loaded.
import './design/components.css';

/** The design system page, at #/systeme. No router: one hash, one screen. */
const SHOWCASE_ROUTE = '#/systeme';

function useHashRoute(): string {
  const [hash, setHash] = useState(() => window.location.hash);

  useEffect(() => {
    const onHashChange = () => setHash(window.location.hash);
    window.addEventListener('hashchange', onHashChange);
    return () => window.removeEventListener('hashchange', onHashChange);
  }, []);

  return hash;
}

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

export default function App() {
  const route = useHashRoute();
  const onShowcase = route === SHOWCASE_ROUTE;
  const [probe, setProbe] = useState<Probe>({ status: 'probing' });

  useEffect(() => {
    // The showcase never displays this probe, and it has to render in a plain
    // browser where `window.__TAURI__` does not exist. Firing the IPC call from
    // here would cost a round trip nobody reads, and would reject on every
    // visit outside the Tauri window. The guard is INSIDE the effect, not
    // around it: a hook that runs only on some routes changes the hook order
    // between renders, which React forbids.
    if (onShowcase) {
      return;
    }

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
  }, [onShowcase]);

  // After the hooks, never before them: the hook order must not depend on the
  // route.
  if (onShowcase) {
    return <Showcase />;
  }

  return (
    <main className="app">
      <h1>Cliché</h1>
      <p className="subtitle">Local screenshot utility. Nothing leaves this machine.</p>

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
              <strong>Failed</strong> — the monitor list could not be read:{' '}
              {probe.message}
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
    </main>
  );
}
