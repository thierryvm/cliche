/**
 * What `?raw` hands back, told to TypeScript.
 *
 * Vite serves `import source from './thing.html?raw'` as the file's text. `tsc`
 * knows nothing about that suffix and would refuse the import outright, so the
 * shape is declared here - narrowly, and by hand.
 *
 * NOT `/// <reference types="vite/client" />`, which is the usual answer and
 * declares far more than this repository uses: `*?raw` for every extension,
 * `*?url`, `*?worker`, every asset type, and `import.meta.env`. Two suffixes are
 * imported in this project, both by `src/documents.test.ts`, and a declaration
 * that says exactly that is one a reader can hold against the code.
 */

declare module '*.html?raw' {
  const source: string;
  export default source;
}

declare module '*.ts?raw' {
  const source: string;
  export default source;
}

/* NO `*.css?raw`, and that is a measurement rather than an omission: under
   Vitest such an import arrives as an EMPTY STRING (`test.css` is false by
   default), so a declaration for it would type something no test can use.
   Measured again on 7 September 2026 - 0 characters for components.css against
   31 333 for veil.html. `scripts/check-hidden.mjs` exists for that reason. */
