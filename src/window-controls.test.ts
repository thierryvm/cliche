/**
 * The maximise/restore control's decision, under test.
 *
 * # Why a pure function, and why the test lives here rather than on a component
 *
 * This repository has no jsdom and no testing-library, and buys neither: a DOM
 * simulator is a large dependency bought to assert on markup this project can
 * look at directly at `#/systeme`. So the rule the title bar is written under is
 * that anything worth testing must be a function of its arguments, and the
 * component must only render what that function returns.
 *
 * WHAT THIS CANNOT REACH, said here so the green is not read as more than it is:
 * whether the component ever LEARNS that the window was maximised. That half is
 * `getCurrentWindow().onResized(...)` in `TitleBar.tsx`, it needs a real window,
 * and nothing below observes it.
 */

import { describe, expect, it } from 'vitest';

import { maximiseControl } from './window-controls';

describe('maximiseControl', () => {
  it('offers to RESTORE a window that is already maximised', () => {
    // The defect this exists for: a title bar that draws « Agrandir » on a
    // window filling the screen. It is invisible in a screenshot of the resting
    // state and obvious to anyone using the application.
    expect(maximiseControl(true)).toEqual({ labelKey: 'windowRestore', icon: 'restore' });
  });

  it('offers to MAXIMISE a window that is not', () => {
    expect(maximiseControl(false)).toEqual({ labelKey: 'windowMaximize', icon: 'maximize' });
  });

  it('draws neither the same glyph nor the same name in the two states', () => {
    // Without this, a mapping that answered the same thing to both questions
    // would satisfy one of the two assertions above and fail only the other -
    // and a control whose two states look alike is exactly the bug, whichever
    // of the two labels it settles on.
    const maximised = maximiseControl(true);
    const restored = maximiseControl(false);

    expect(maximised.icon).not.toBe(restored.icon);
    expect(maximised.labelKey).not.toBe(restored.labelKey);
  });
});
