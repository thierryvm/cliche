#!/usr/bin/env node
// Cliche keeps its version number in THREE files, because three different
// toolchains each insist on reading their own:
//
//   package.json          -> pnpm / vite
//   src-tauri/Cargo.toml  -> cargo, and the version embedded in the binary
//   src-tauri/tauri.conf.json -> the bundler, the installer, and the string
//                                the Help page will display (lot 2)
//
// Three copies of one fact is exactly the pattern this project forbids
// elsewhere (see the shortcut registry, lot 2). It is tolerated here only
// because no single-source mechanism has been VERIFIED for this Tauri
// version yet -- see docs/RELEASES.md, "One version, three files".
//
// So the copies are not trusted: they are CHECKED. This script fails the
// build if they drift. It is wired into `pnpm test` and into CI, which means
// a release cannot be cut with a stale number in one of the three.
//
// No dependency on purpose: it must run before `pnpm install` has any say.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');

/** Reads package.json and returns its `version` field. */
function fromPackageJson() {
  const raw = readFileSync(join(root, 'package.json'), 'utf8');
  const value = JSON.parse(raw).version;
  if (typeof value !== 'string') throw new Error('package.json: no "version" string');
  return value;
}

/** Reads tauri.conf.json and returns its top-level `version` field. */
function fromTauriConf() {
  const raw = readFileSync(join(root, 'src-tauri', 'tauri.conf.json'), 'utf8');
  const value = JSON.parse(raw).version;
  if (typeof value !== 'string') throw new Error('tauri.conf.json: no "version" string');
  return value;
}

/**
 * Reads the `version` of the [package] section of Cargo.toml.
 *
 * Deliberately naive: it stops at the first section header after [package],
 * so a `version` belonging to a dependency can never be mistaken for the
 * crate version. A full TOML parser would be a dependency, and this file
 * must stay dependency-free.
 */
function fromCargoToml() {
  const raw = readFileSync(join(root, 'src-tauri', 'Cargo.toml'), 'utf8');
  const lines = raw.split(/\r?\n/);
  let inPackage = false;
  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith('[')) {
      inPackage = trimmed === '[package]';
      continue;
    }
    if (!inPackage) continue;
    const match = /^version\s*=\s*"([^"]+)"/.exec(trimmed);
    if (match) return match[1];
  }
  throw new Error('Cargo.toml: no version in the [package] section');
}

const found = {
  'package.json': fromPackageJson(),
  'src-tauri/Cargo.toml': fromCargoToml(),
  'src-tauri/tauri.conf.json': fromTauriConf(),
};

const distinct = [...new Set(Object.values(found))];

for (const [file, version] of Object.entries(found)) {
  console.log(`  ${version.padEnd(12)} ${file}`);
}

if (distinct.length !== 1) {
  console.error(
    `\nFAIL: the version number differs across files (${distinct.join(', ')}).\n` +
      'Release procedure: docs/RELEASES.md. All three must be bumped together.',
  );
  process.exit(1);
}

console.log(`\nOK: one version everywhere - ${distinct[0]}`);
