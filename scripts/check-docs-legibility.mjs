#!/usr/bin/env node
import { readFile } from "node:fs/promises";
import path from "node:path";

const files = [
  "README.md",
  "docs/index.md",
  "docs/user/index.md",
  "docs/user/user-guide.md",
  "docs/user/status.md",
  "docs/user/storage-scope.md",
  "docs/user/cli.md",
  "docs/user/cli-plan-apply.md",
  "docs/user/nixos-module.md",
  "docs/user/nixos-module-reference.md",
  "docs/user/operator-runbooks.md",
  "docs/developer/index.md",
  "docs/developer/architecture.md",
  "docs/developer/planning.md",
  "docs/developer/compatibility.md",
  "docs/developer/feature-checklist.md",
  "docs/developer/integration-tests.md",
  "docs/developer/integration-failure-recovery.md",
  "docs/developer/integration-smoke-harnesses.md",
];

const maxWords = 60;
const failures = [];
const root = process.argv[2] || ".";

function countWords(text) {
  return text.trim().split(/\s+/).filter(Boolean).length;
}

function isStructural(line) {
  return (
    line.trim() === ""
    || /^#{1,6}\s+/.test(line)
    || /^[-*]\s+/.test(line)
    || /^\s{2,}\S/.test(line)
    || /^\d+\.\s+/.test(line)
    || /^>\s?/.test(line)
    || /^\|/.test(line)
    || /^---+$/.test(line.trim())
  );
}

for (const file of files) {
  const text = await readFile(path.join(root, file), "utf8");
  const lines = text.split(/\r?\n/);
  let inCode = false;
  let paragraph = [];
  let startLine = 0;

  const flush = (endLine) => {
    if (paragraph.length === 0) {
      return;
    }
    const value = paragraph.join(" ");
    const words = countWords(value);
    if (words > maxWords) {
      failures.push(`${file}:${startLine}-${endLine}: ${words} words`);
    }
    paragraph = [];
    startLine = 0;
  };

  lines.forEach((line, index) => {
    const lineNumber = index + 1;
    if (line.startsWith("```")) {
      flush(lineNumber - 1);
      inCode = !inCode;
      return;
    }
    if (inCode || isStructural(line)) {
      flush(lineNumber - 1);
      return;
    }
    if (paragraph.length === 0) {
      startLine = lineNumber;
    }
    paragraph.push(line.trim());
  });
  flush(lines.length);
}

if (failures.length > 0) {
  console.error(`Documentation prose paragraphs exceed ${maxWords} words:`);
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log(`Documentation prose paragraphs are ${maxWords} words or shorter.`);
