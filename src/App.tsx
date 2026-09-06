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
import Settings from './Settings';
import TitleBar from './TitleBar';
import Showcase from './design/Showcase';

// This file renders .c-shell and .c-shell__body, so it depends on the material
// layer directly. It used to arrive only because Showcase happens to import it —
// an accident that would break the day the showcase is lazy-loaded.
import './design/components.css';

/** The design system page. Draws every state; calls nothing. */
const SHOWCASE_ROUTE = '#/systeme';

/**
 * The help screen: the shortcut registry, and the monitor read-out under it.
 *
 * Reached from the title bar and from nowhere else - the control there is what
 * makes this route exist for somebody who is not reading this file. A screen
 * with no way in is a screen that does not exist.
 */
const HELP_ROUTE = '#/aide';

/**
 * The settings screen: the capture combination, and the control that changes it.
 *
 * Reached from the title bar, exactly like the help and for the same reason -
 * the control there is what makes this route exist for somebody who is not
 * reading this file. ASCII in the hash, deliberately: a fragment is part of a
 * URL, and a non-ASCII one is liable to come back from `location.hash`
 * percent-encoded while the literal it is compared against is not. Avoided
 * rather than measured - « reglages » costs nothing and settles the question.
 */
const SETTINGS_ROUTE = '#/reglages';

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

  const onHelp = route === HELP_ROUTE;
  const onSettings = route === SETTINGS_ROUTE;

  return (
    <div className="c-shell">
      <TitleBar
        title={WINDOW_TITLE}
        helpOpen={onHelp}
        onToggleHelp={() => {
          // The hash IS the state, so the browser's own back button keeps
          // working and the screen survives a reload. Writing '' clears the
          // fragment, which `useHashRoute` reads as the launcher.
          window.location.hash = onHelp ? '' : HELP_ROUTE;
        }}
        settingsOpen={onSettings}
        onToggleSettings={() => {
          window.location.hash = onSettings ? '' : SETTINGS_ROUTE;
        }}
      />
      {/* Two toggles, one body: pressing one while the other is up SWAPS the
          screen rather than needing the first to be released. That falls out of
          writing the hash - the control that is not pressed writes its own
          route - and it is why neither needs to know about the other. */}
      <div className="c-shell__body">
        {onSettings ? <Settings /> : onHelp ? <Help /> : <Launcher />}
      </div>
    </div>
  );
}
