#!/usr/bin/env node

import { readFileSync, writeFileSync } from 'node:fs';

const [changelogPath = 'CHANGELOG.md', version, outputPath] = process.argv.slice(2);

if (!version || !outputPath) {
  console.error(
    'Usage: node scripts/release/extract-release-notes.js <changelog> <version> <output>',
  );
  process.exit(2);
}

const lines = readFileSync(changelogPath, 'utf8').split(/\r?\n/);
const startMarker = '<!-- release:start -->';
const endMarker = '<!-- release:end -->';
const startIndexes = indexesOf(lines, startMarker);
const endIndexes = indexesOf(lines, endMarker);

if (startIndexes.length !== 1 || endIndexes.length !== 1) {
  fail(
    `Expected exactly one release marker pair, found ${startIndexes.length} start and ${endIndexes.length} end markers`,
  );
}

const start = startIndexes[0];
const end = endIndexes[0];
if (end <= start) {
  fail('Release end marker must follow the start marker');
}

const headingIndex = findPreviousVersionHeading(lines, start);
const expectedHeading = `## ${version}`;
if (headingIndex < 0 || lines[headingIndex].trim() !== expectedHeading) {
  const actual = headingIndex < 0 ? 'none' : lines[headingIndex].trim();
  fail(`Release markers belong to ${actual}, expected ${expectedHeading}`);
}

const nextHeading = lines.findIndex(
  (line, index) => index > headingIndex && /^##\s+\S/.test(line),
);
if (nextHeading >= 0 && end > nextHeading) {
  fail('Release marker pair crosses into the next changelog entry');
}

const notes = lines.slice(start + 1, end);
if (notes.filter((line) => line.trim()).length < 2) {
  fail('Release notes must contain at least two nonempty lines');
}

writeFileSync(outputPath, `${notes.join('\n').replace(/\n+$/, '')}\n`);
console.log(`Extracted release notes for ${version} (${notes.length} lines)`);

function indexesOf(values, expected) {
  const indexes = [];
  values.forEach((value, index) => {
    if (value.trim() === expected) indexes.push(index);
  });
  return indexes;
}

function findPreviousVersionHeading(values, beforeIndex) {
  for (let index = beforeIndex - 1; index >= 0; index -= 1) {
    if (/^##\s+\S/.test(values[index])) return index;
  }
  return -1;
}

function fail(message) {
  console.error(`Error: ${message}`);
  process.exit(1);
}
