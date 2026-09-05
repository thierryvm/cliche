/*
 * The window: its chrome, and whichever screen the hash asks for.
 *
 * No router, and still none: three hashes, three screens. A router is a
 * dependency, and this application has three destinations.
 */

import { useEffect, useState } from 'react';

import DisplaysProbe from './DisplaysProbe';
import Launcher from './Launcher';
import TitleBar from './TitleBar';
import Showcase from './design/Showcase';

// This file renders .c-shell and .c-shell__body, so it depends on the material
// layer directly. It used to arrive only because Showcase happens to import it —
// an accident that would break the day the showcase is lazy-loaded.
import './design/components.css';

/** The design system page. Draws every state; calls nothing. */
const SHOWCASE_ROUTE = '#/systeme';

/**
 * The monitor read-out, on its way to the help page.
 *
 * A route rather than a component nobody mounts: an unmounted diagnostic is a
 * diagnostic that stops working in silence. See the header of `DisplaysProbe`.
 */
const DIAGNOSTIC_ROUTE = '#/diagnostic';

/** The name in the title bar. The product's NAME, so not in the catalogue. */
const WINDOW_TITLE = 'Cliché';

function useHashRoute(): string {
  const [hash, setHash] = useState(() => window.location.hash);

  useEffect(() => {
    const onHashChange = () => setHash(window.location.hash);
    window.addEventListener('hashchange', onHashChange);
    return () => window.removeEventListener('hashchange', onHashChange);
  }, []);

  return hash;
}

export default function App() {
  const route = useHashRoute();

  // The showcase is the ONE screen that renders no chrome: it has to open in a
  // plain browser tab, and a title bar whose controls drive a window that is
  // not there would be the first thing a reviewer clicked.
  if (route === SHOWCASE_ROUTE) {
    return <Showcase />;
  }

  return (
    <div className="c-shell">
      <TitleBar title={WINDOW_TITLE} />
      <div className="c-shell__body">
        {route === DIAGNOSTIC_ROUTE ? <DisplaysProbe /> : <Launcher />}
      </div>
    </div>
  );
}
