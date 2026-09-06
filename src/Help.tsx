/*
 * The help screen: every registered shortcut, and the monitor read-out.
 *
 * # NOT ONE COMBINATION IS WRITTEN HERE
 *
 * That is the whole of `docs/PLAN.md` lot 2, and the reason this file is so
 * thin: it asks `describe_shortcuts` for the Rust registry, hands the answer to
 * `helpLines` / `helpSections` in `src/help.ts`, and draws rows. Add an entry to
 * `REGISTRY` in `src-tauri/src/shortcuts.rs`, restart, and it appears here -
 * without this file, or any other under src/, being opened. The derivation is
 * where the claim lives and where the test is; nothing is decided in this file.
 *
 * # THE MARKUP IS THE MAQUETTE'S, class for class
 *
 * `src/design/Showcase.tsx`, section `s-keymap`: a `<dl class="c-keymap">` whose
 * `<dt>` is the combination and whose `<dd>` is the action, under a heading. Two
 * differences, both deliberate and both the same kind the launcher already
 * carries:
 *
 *   1. The heading levels are promoted. The showcase draws the panel's heading
 *      as an `<h3>` because it sits under that page's own `h1` and `h2`; here
 *      the screen name is the first heading of a window, so it is `<h1>` and
 *      the category is `<h2>`. `.c-screen__name` and `.c-section__label` are
 *      unchanged, so nothing about the look moves.
 *   2. `.c-keymap > dt` and `.c-keymap > dd` are DIRECT-child selectors. The
 *      rows are wrapped in a `<Fragment>`, which renders no DOM node, so they
 *      stay direct children of the `<dl>`. A `<div>` per row would silently
 *      lose the grid.
 *
 * # THE DIAGNOSTIC IS A SECTION OF THIS SCREEN, since 6 September 2026
 *
 * `DisplaysProbe` used to be mounted behind `#/diagnostic`, as a placeholder
 * until this page existed. That route is GONE: two ways into one read-out is
 * one way too many, and the second was reachable only by typing a URL into a
 * window with no address bar.
 */

import { Fragment, useEffect, useState } from 'react';

import DisplaysProbe from './DisplaysProbe';
import Keys from './Keys';
import { Glyph, ICON } from './design/Glyph';
import { helpSections } from './help-rows';
import type { HelpSection } from './help-rows';
import { describeShortcuts } from './shortcuts';
import type { ShortcutEntry } from './shortcuts';
import { UI_STRINGS } from './strings';

import './design/components.css';

/** The gap between the screen's blocks, as the maquette spends it. */
const BLOCK_GAP = { marginBlockStart: 'var(--space-5)' } as const;

/** Where this screen's read of the registry stands. */
type RegistryRead =
  | { readonly status: 'reading' }
  | { readonly status: 'read'; readonly entries: readonly ShortcutEntry[] }
  | { readonly status: 'unreadable' };

export default function Help() {
  const [read, setRead] = useState<RegistryRead>({ status: 'reading' });

  useEffect(() => {
    // StrictMode runs effects twice in development, so the command is asked
    // twice there. That is the dev double-render, not a bug.
    let abandoned = false;

    describeShortcuts().then(
      (entries) => {
        if (!abandoned) {
          setRead({ status: 'read', entries });
        }
      },
      (error: unknown) => {
        if (!abandoned) {
          // The message is not shown, for the reason the launcher gives: what
          // the user needs to know is that the shortcuts cannot be listed. A
          // refused ACL or a wrong-window guard is actionable in a console and
          // nowhere else.
          console.error('[cliche] help: the shortcut registry could not be read', error);
          setRead({ status: 'unreadable' });
        }
      },
    );

    return () => {
      abandoned = true;
    };
  }, []);

  // Deliberately computed on every render rather than memoised: it is a `map`
  // over a list one entry long, and a `useMemo` here would cost more to read
  // than it saves to run.
  //
  // The annotation is not decoration either: without it the ternary types as
  // `readonly HelpSection[] | never[]`, and calling `.map` on a UNION of array
  // types is one of the shapes TypeScript refuses to resolve a signature for.
  const sections: readonly HelpSection[] =
    read.status === 'read' ? helpSections(read.entries) : [];

  return (
    <>
      <h1 className="c-screen__name">{UI_STRINGS.helpTitle}</h1>

      {read.status === 'reading' && (
        <p className="c-hint" style={BLOCK_GAP} aria-busy="true">
          <span>{UI_STRINGS.shortcutLoading}</span>
        </p>
      )}

      {read.status === 'unreadable' && (
        <div className="c-note c-note--danger" role="alert" style={BLOCK_GAP}>
          <Glyph d={ICON.alert} />
          <span>
            <strong>{UI_STRINGS.failure}</strong>
            {' — '}
            {UI_STRINGS.shortcutRegistryUnreadable}
          </span>
        </div>
      )}

      {sections.map((section) => (
        <section key={section.category} style={BLOCK_GAP}>
          {/* The heading is drawn only when the catalogue names this category.
              A category it cannot name is drawn WITHOUT one rather than under a
              word invented here — see `headingFor` in src/help.ts. */}
          {section.heading !== undefined && (
            <h2 className="c-section__label">{section.heading}</h2>
          )}
          <dl className="c-keymap" style={BLOCK_GAP}>
            {section.lines.map((line) => (
              <Fragment key={line.id}>
                <dt>
                  <Keys keys={line.keys} />
                </dt>
                {/* An entry whose description key is not in the catalogue
                    leaves this cell EMPTY. The row still exists, because the
                    combination is registered and the user can press it; the
                    alternative — printing the raw key — would put an
                    engineering identifier in front of them. A Rust test
                    (`every_description_key_exists_in_the_string_catalogue`)
                    is what keeps this branch unreachable in a shipped build. */}
                <dd>{line.description}</dd>
              </Fragment>
            ))}
          </dl>
        </section>
      ))}

      {/* The engineering read-out, in English and deliberately outside
          src/strings.ts — its own header says why. It is a section of this
          screen now, and the only way to it. */}
      <div style={BLOCK_GAP}>
        <DisplaysProbe />
      </div>
    </>
  );
}
