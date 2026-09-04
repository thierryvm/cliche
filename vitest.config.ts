/**
 * The JavaScript test runner, and the reasoning behind the version pinned in
 * package.json.
 *
 * # A SEPARATE FILE, never a `test` block in vite.config.ts
 *
 * Vitest reads `vitest.config.ts` in preference to `vite.config.ts` when both
 * exist, and merges nothing from the latter unless asked. That is exactly what
 * is wanted here: `vite.config.ts` produces the veil bundle whose paint budget
 * lot 1 measures, and it was validated on screen. A `test` key added inside it
 * would put the runner one careless edit away from the two entry points and the
 * `chrome110` target that the product depends on. This file cannot do that.
 *
 * The cost of the separation is real and worth naming: the two configurations
 * do not share `resolve.alias` or plugins. Neither is used by the code under
 * test - `src/veil/zones.ts` imports nothing at all - so today the cost is zero.
 * The day a test needs a plugin, the honest move is `mergeConfig`, not a `test`
 * block next to `rollupOptions`.
 *
 * # WHY VITEST 4.1.11 AND NOT 5.0.0
 *
 * Read on the npm registry on 4 September 2026:
 *
 *   | version | published  | peer `vite`                      |
 *   | 4.1.11  | 2026-08-18 | ^6.0.0 \|\| ^7.0.0 \|\| ^8.0.0   |
 *   | 5.0.0   | 2026-09-03 | ^6.4.0 \|\| ^7.0.0 \|\| ^8.0.0   |
 *
 * 5.0.0 was one day old. A major release of a TEST RUNNER is the dependency one
 * least wants fresh: its failure mode is not a missing feature, it is a FALSE
 * RED - a test that fails for a reason belonging to the runner - and a false
 * red costs more than a feature nobody has yet. The `peer` range decides the
 * rest: 4.1.11 accepts any Vite 6, 5.0.0 requires >= 6.4. This repository is on
 * `vite: ^6.0.0`, so the newer major would tighten a constraint of the product
 * in exchange for a major nothing here asks for.
 *
 * Pinned exactly, without a caret, like `@tauri-apps/cli`: the argument above
 * is about 4.1.11 and about that peer range, and a range would let the argument
 * quietly stop describing what is installed.
 *
 * # NO DOM, AND NO DEPENDENCY THAT WOULD PROVIDE ONE
 *
 * `environment: 'node'` is Vitest's default and is stated anyway, because it is
 * a decision rather than an omission. The only file under test, `zones.ts`,
 * touches nothing but its arguments; jsdom or happy-dom would be a second
 * dependency bought to simulate something no test reads. The DOM half of the
 * veil stays covered the way it already was - by the throws at load in
 * `main.ts`, which fire in the real WebView2 during the preheat.
 */

import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'node',

    // Tests sit next to what they test, as they do in Rust. Restricted to
    // `src` so the runner never wanders into `src-tauri/target`, which holds
    // build output measured in gigabytes.
    include: ['src/**/*.test.ts'],
  },
});
