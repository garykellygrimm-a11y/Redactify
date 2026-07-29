#!/usr/bin/env node
/*
 * Aggregate the per-package changelogs into the root CHANGELOG.md.
 *
 * Knope writes one changelog per package (app/CHANGELOG.md and
 * crates/redactify-cli/CHANGELOG.md). This collects every entry from
 * both, labels it with its package, sorts newest-first, and rewrites the
 * generated region of the root file.
 *
 * Written in Node rather than bash/python because it has to run both on
 * a Linux CI runner and locally on Windows, and Node is already a hard
 * dependency of this repo (the frontend build).
 *
 * Two deliberate properties:
 *
 *  - It REGENERATES the whole generated region every run instead of
 *    appending. That makes it idempotent: running it twice produces the
 *    same file, and a botched run is fixed by just running it again.
 *    Append-style scripts need "have I already added this?" logic, which
 *    is exactly where that kind of tool quietly goes wrong.
 *
 *  - It only touches the text BETWEEN the two marker comments. The
 *    preamble above and the frozen pre-0.6 history below are never
 *    rewritten, so the unified releases through 0.5.0 survive intact even
 *    though no per-package changelog contains them.
 */

const fs = require("fs");
const path = require("path");

const ROOT = process.cwd();
const BEGIN = "<!-- BEGIN GENERATED CHANGELOG -->";
const END = "<!-- END GENERATED CHANGELOG -->";

const SOURCES = [
  { label: "Desktop App", file: "app/CHANGELOG.md" },
  { label: "CLI", file: "crates/redactify-cli/CHANGELOG.md" },
];

/** Split a Knope changelog into { version, date, body } entries. */
function parseChangelog(text, label) {
  const entries = [];
  const lines = text.split(/\r?\n/);
  let current = null;

  for (const line of lines) {
    // Knope writes headings as `## 1.2.3 (2026-07-29)`. Anything before
    // the first such heading is the file's own preamble and is skipped.
    const match = line.match(/^##\s+(\d+\.\d+\.\d+[^\s(]*)\s*\((\d{4}-\d{2}-\d{2})\)\s*$/);
    if (match) {
      if (current) entries.push(current);
      current = { label, version: match[1], date: match[2], body: [] };
      continue;
    }
    if (current) current.body.push(line);
  }
  if (current) entries.push(current);

  // Trim leading/trailing blank lines from each body.
  for (const e of entries) {
    while (e.body.length && e.body[0].trim() === "") e.body.shift();
    while (e.body.length && e.body[e.body.length - 1].trim() === "") e.body.pop();
  }
  return entries;
}

function main() {
  const rootPath = path.join(ROOT, "CHANGELOG.md");
  if (!fs.existsSync(rootPath)) {
    console.error("error: CHANGELOG.md not found — run this from the repo root");
    process.exit(1);
  }

  const rootText = fs.readFileSync(rootPath, "utf8");
  const beginIdx = rootText.indexOf(BEGIN);
  const endIdx = rootText.indexOf(END);
  if (beginIdx === -1 || endIdx === -1 || endIdx < beginIdx) {
    console.error(
      `error: CHANGELOG.md must contain both marker comments:\n  ${BEGIN}\n  ${END}`
    );
    process.exit(1);
  }

  let entries = [];
  for (const src of SOURCES) {
    const p = path.join(ROOT, src.file);
    if (!fs.existsSync(p)) {
      // Expected before a package's first release — not an error.
      console.log(`  (no changelog yet at ${src.file}, skipping)`);
      continue;
    }
    entries = entries.concat(parseChangelog(fs.readFileSync(p, "utf8"), src.label));
  }

  // Newest first. Within one date, keep SOURCES order (app then CLI) so
  // repeated runs are byte-identical rather than dependent on sort
  // stability across Node versions.
  const order = new Map(SOURCES.map((s, i) => [s.label, i]));
  entries.sort((a, b) => {
    if (a.date !== b.date) return a.date < b.date ? 1 : -1;
    return order.get(a.label) - order.get(b.label);
  });

  const generated = entries
    .map((e) => `## ${e.label} ${e.version} (${e.date})\n\n${e.body.join("\n")}`)
    .join("\n\n");

  const next =
    rootText.slice(0, beginIdx + BEGIN.length) +
    "\n\n" +
    generated +
    "\n\n" +
    rootText.slice(endIdx);

  fs.writeFileSync(rootPath, next);
  console.log(`  aggregated ${entries.length} entr${entries.length === 1 ? "y" : "ies"} into CHANGELOG.md`);
}

main();
