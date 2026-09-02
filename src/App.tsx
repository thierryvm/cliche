import { useEffect, useState } from 'react';

import { describeDisplays } from './displays';
import type { DisplayInfo } from './displays';

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
    <main className="app">
      <h1>Cliché</h1>
      <p className="subtitle">Local screenshot utility. Nothing leaves this machine.</p>

      <section aria-labelledby="displays-heading">
        <h2 id="displays-heading">Displays detected at startup</h2>
        {probe.status === 'probing' && <p role="status">Reading the monitor list…</p>}

        {probe.status === 'failed' && (
          <p role="alert" className="error">
            describe_displays failed: {probe.message}
          </p>
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
