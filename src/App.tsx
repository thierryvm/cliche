/*
 * The window: its chrome, and whichever screen the hash asks for.
 *
 * No router, and still none: four hashes, four screens. A router is a
 * dependency, and this application has four destinations.
 *
 * `#/diagnostic` WENT on 6 September 2026. The monitor read-out is a section of
 * the help screen now, and a second door into it - one reachable only by typing
 * a URL into a window that has no address bar - was a way for the two to drift
 * apart.
 */

import { useEffect, useState } from 'react';

import Help from './Help';
import Launcher from './Launcher';
import { screenFor, screenTabs } from './screen-tabs';
import Settings from './Settings';
import TitleBar from './TitleBar';
import Showcase from './design/Showcase';

// This file renders .c-shell and .c-shell__body, so it depends on the material
// layer directly. It used to arrive only because Showcase happens to import it —
// an accident that would break the day the showcase is lazy-loaded.
import './design/components.css';

/** The design system page. Draws every state; calls nothing. */
const SHOWCASE_ROUTE = '#/systeme';

/* THE OTHER THREE ROUTES ARE NOT DECLARED HERE any more, since 7 September
   2026. They live in `src/screen-tabs.ts`, with the tabs that lead to them: the
   control that SENDS you to a screen and the switch that decides what is drawn
   there must not be able to disagree, and they were two lists in two halves of
   this file. The showcase route stays because it is the one destination with no
   tab - it renders no chrome at all. */

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

  const screen = screenFor(route);

  return (
    <div className="c-shell">
      <TitleBar
        title={WINDOW_TITLE}
        tabs={screenTabs(route)}
        onGoTo={(hash) => {
          // The hash IS the state, so the browser's own back button keeps
          // working and the screen survives a reload. The launcher's hash is
          // '', which clears the fragment and is what `screenFor` reads as the
          // home screen.
          window.location.hash = hash;
        }}
      />
      {/* Three tabs, one body, and no toggling left: each tab writes ITS OWN
          hash, so going from the help to the settings is one press rather than
          two, and « Capturer » is a way home that is written on the screen
          instead of being a second press on the control you are already on. */}
      <div className="c-shell__body">
        {screen === 'settings' ? <Settings /> : screen === 'help' ? <Help /> : <Launcher />}
      </div>
    </div>
  );
}
