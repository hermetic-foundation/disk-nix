#!/usr/bin/env node
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const docs = [
  ["index", "Docs index", "docs/index.md"],
  ["readme", "README", "README.md"],
  ["user-docs", "User docs", "docs/user-docs.md"],
  ["developer-docs", "Developer docs", "docs/developer-docs.md"],
  ["user-guide", "User guide", "docs/user-guide.md"],
  ["status", "Status", "docs/status.md"],
  ["architecture", "Architecture", "docs/architecture.md"],
  ["storage-scope", "Storage scope", "docs/storage-scope.md"],
  ["cli", "CLI", "docs/cli.md"],
  ["cli-plan-apply", "CLI planning and apply", "docs/cli-plan-apply.md"],
  ["planning", "Planning", "docs/planning.md"],
  ["nixos-module", "NixOS module", "docs/nixos-module.md"],
  ["nixos-module-reference", "NixOS module reference", "docs/nixos-module-reference.md"],
  ["integration-tests", "Integration tests", "docs/integration-tests.md"],
  ["integration-failure-recovery", "Integration failure recovery", "docs/integration-failure-recovery.md"],
  ["integration-smoke-harnesses", "Integration smoke harnesses", "docs/integration-smoke-harnesses.md"],
  ["operator-runbooks", "Operator runbooks", "docs/operator-runbooks.md"],
  ["compatibility", "Compatibility", "docs/compatibility.md"],
  ["feature-checklist", "Feature checklist", "docs/feature-checklist.md"],
];

const outDir = process.argv[2] || "build/docs-site";

function escapeHtml(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function slugify(value) {
  return value
    .toLowerCase()
    .replace(/`([^`]+)`/g, "$1")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function linkHref(href) {
  if (/^[a-z]+:/i.test(href) || href.startsWith("#")) {
    return href;
  }
  const [base, hash] = href.split("#");
  const normalized = base.replace(/^\.\//, "").replace(/^\.\.\//, "");
  const doc = docs.find(([, , file]) => file === normalized || file === `docs/${normalized}`);
  if (!doc) {
    return href;
  }
  return `${doc[0]}.html${hash ? `#${hash}` : ""}`;
}

function inlineMarkdown(value) {
  let html = escapeHtml(value);
  html = html.replace(/`([^`]+)`/g, "<code>$1</code>");
  html = html.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  html = html.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_match, label, href) => {
    return `<a href="${escapeHtml(linkHref(href))}">${label}</a>`;
  });
  return html;
}

function renderTable(lines) {
  const rows = lines.map((line) =>
    line
      .trim()
      .replace(/^\||\|$/g, "")
      .split("|")
      .map((cell) => inlineMarkdown(cell.trim())),
  );
  const header = rows[0] || [];
  const body = rows.slice(2);
  return [
    "<div class=\"table-wrap\"><table>",
    "<thead><tr>",
    ...header.map((cell) => `<th>${cell}</th>`),
    "</tr></thead>",
    "<tbody>",
    ...body.map((row) => `<tr>${row.map((cell) => `<td>${cell}</td>`).join("")}</tr>`),
    "</tbody></table></div>",
  ].join("");
}

function renderMarkdown(markdown) {
  const lines = markdown.split(/\r?\n/);
  const html = [];
  const headings = [];
  let i = 0;
  let inParagraph = false;
  let listTag = "";
  let inCode = false;
  let codeLang = "";
  let codeLines = [];

  const closeParagraph = () => {
    if (inParagraph) {
      html.push("</p>");
      inParagraph = false;
    }
  };
  const closeList = () => {
    if (listTag) {
      html.push(`</${listTag}>`);
      listTag = "";
    }
  };
  const openList = (tag) => {
    if (listTag === tag) {
      return;
    }
    closeList();
    html.push(`<${tag}>`);
    listTag = tag;
  };

  while (i < lines.length) {
    const line = lines[i];

    if (inCode) {
      if (line.startsWith("```")) {
        html.push(`<pre><code class="language-${escapeHtml(codeLang)}">${escapeHtml(codeLines.join("\n"))}</code></pre>`);
        inCode = false;
        codeLang = "";
        codeLines = [];
      } else {
        codeLines.push(line);
      }
      i += 1;
      continue;
    }

    if (line.startsWith("```")) {
      closeParagraph();
      closeList();
      inCode = true;
      codeLang = line.slice(3).trim();
      i += 1;
      continue;
    }

    if (/^\|.+\|$/.test(line) && i + 1 < lines.length && /^\|?[-: |]+\|?$/.test(lines[i + 1])) {
      closeParagraph();
      closeList();
      const table = [];
      while (i < lines.length && /^\|.+\|$/.test(lines[i])) {
        table.push(lines[i]);
        i += 1;
      }
      html.push(renderTable(table));
      continue;
    }

    const heading = /^(#{1,6})\s+(.+)$/.exec(line);
    if (heading) {
      closeParagraph();
      closeList();
      const level = heading[1].length;
      const text = heading[2].trim();
      const id = slugify(text);
      headings.push({ level, text, id });
      html.push(`<h${level} id="${id}">${inlineMarkdown(text)}</h${level}>`);
      i += 1;
      continue;
    }

    const listItem = /^-\s+(.*)$/.exec(line);
    if (listItem) {
      closeParagraph();
      openList("ul");
      let text = listItem[1].trim();
      i += 1;
      while (
        i < lines.length
        && lines[i].trim() !== ""
        && !lines[i].startsWith("```")
        && !/^(#{1,6})\s+/.test(lines[i])
        && !/^-\s+/.test(lines[i])
        && !/^\|.+\|$/.test(lines[i])
      ) {
        text += ` ${lines[i].trim()}`;
        i += 1;
      }
      html.push(`<li>${inlineMarkdown(text)}</li>`);
      continue;
    }

    const orderedListItem = /^\d+\.\s+(.*)$/.exec(line);
    if (orderedListItem) {
      closeParagraph();
      openList("ol");
      let text = orderedListItem[1].trim();
      i += 1;
      while (
        i < lines.length
        && lines[i].trim() !== ""
        && !lines[i].startsWith("```")
        && !/^(#{1,6})\s+/.test(lines[i])
        && !/^\d+\.\s+/.test(lines[i])
        && !/^-\s+/.test(lines[i])
        && !/^\|.+\|$/.test(lines[i])
      ) {
        text += ` ${lines[i].trim()}`;
        i += 1;
      }
      html.push(`<li>${inlineMarkdown(text)}</li>`);
      continue;
    }

    if (line.trim() === "") {
      closeParagraph();
      closeList();
      i += 1;
      continue;
    }

    closeList();
    if (!inParagraph) {
      html.push("<p>");
      inParagraph = true;
    } else {
      html.push(" ");
    }
    html.push(inlineMarkdown(line.trim()));
    i += 1;
  }

  closeParagraph();
  closeList();
  return { body: html.join("\n"), headings };
}

function page(title, activeSlug, body, headings) {
  const nav = docs
    .map(([slug, label]) => `<a class="${slug === activeSlug ? "active" : ""}" href="${slug}.html">${label}</a>`)
    .join("\n");
  const toc = headings
    .filter((heading) => heading.level >= 2 && heading.level <= 3)
    .slice(0, 80)
    .map((heading) => `<a class="toc-l${heading.level}" href="#${heading.id}">${escapeHtml(heading.text.replace(/`/g, ""))}</a>`)
    .join("\n");

  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>${escapeHtml(title)} - disk-nix docs</title>
  <link rel="stylesheet" href="style.css">
</head>
<body>
  <aside class="site-nav">
    <div class="brand">disk-nix docs</div>
    ${nav}
  </aside>
  <main>
    <article>
      ${body}
    </article>
  </main>
  <aside class="toc">
    <div class="toc-title">On this page</div>
    ${toc || "<span>No sections</span>"}
  </aside>
</body>
</html>`;
}

const css = `
:root {
  color-scheme: light;
  --bg: #f7f8fa;
  --panel: #ffffff;
  --text: #18202a;
  --muted: #647184;
  --border: #d9dee7;
  --code-bg: #101820;
  --code-text: #f5f7fa;
  --accent: #2458c2;
}
* { box-sizing: border-box; }
body {
  margin: 0;
  color: var(--text);
  background: var(--bg);
  font: 16px/1.62 system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}
.site-nav {
  position: fixed;
  inset: 0 auto 0 0;
  width: 248px;
  overflow: auto;
  padding: 24px 18px;
  background: var(--panel);
  border-right: 1px solid var(--border);
}
.brand {
  font-weight: 700;
  margin-bottom: 16px;
}
.site-nav a,
.toc a {
  display: block;
  color: var(--muted);
  text-decoration: none;
  border-radius: 6px;
}
.site-nav a {
  padding: 7px 9px;
}
.site-nav a.active,
.site-nav a:hover,
.toc a:hover {
  color: var(--accent);
  background: #edf3ff;
}
main {
  margin-left: 248px;
  margin-right: 284px;
  padding: 40px 48px;
}
article {
  max-width: 980px;
  margin: 0 auto;
  padding: 44px 56px;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow-wrap: anywhere;
}
.toc {
  position: fixed;
  inset: 0 0 0 auto;
  width: 284px;
  overflow: auto;
  padding: 28px 22px;
  background: var(--bg);
  border-left: 1px solid var(--border);
}
.toc-title {
  margin-bottom: 10px;
  color: var(--muted);
  font-size: 13px;
  font-weight: 700;
  text-transform: uppercase;
}
.toc a {
  padding: 5px 7px;
  font-size: 13px;
}
.toc-l3 {
  padding-left: 18px !important;
}
h1, h2, h3, h4 {
  line-height: 1.22;
  letter-spacing: 0;
}
h1 {
  margin: 0 0 18px;
  font-size: 2.2rem;
}
h2 {
  margin-top: 42px;
  padding-top: 24px;
  border-top: 1px solid var(--border);
}
h3 {
  margin-top: 28px;
}
p, ul, ol {
  margin: 0 0 16px;
}
li {
  margin: 6px 0;
}
a {
  color: var(--accent);
}
code {
  padding: 0.14em 0.32em;
  border-radius: 4px;
  background: #eef1f5;
  font: 0.92em ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  overflow-wrap: anywhere;
}
pre {
  overflow: auto;
  margin: 20px 0;
  padding: 18px;
  border-radius: 8px;
  background: var(--code-bg);
}
pre code {
  padding: 0;
  color: var(--code-text);
  background: transparent;
  white-space: pre;
  overflow-wrap: normal;
}
.table-wrap {
  overflow: auto;
  margin: 20px 0;
}
table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.95rem;
}
th, td {
  padding: 9px 11px;
  border: 1px solid var(--border);
  text-align: left;
  vertical-align: top;
}
th {
  background: #eef1f5;
}
@media (max-width: 1100px) {
  .toc { display: none; }
  main { margin-right: 0; }
}
@media (max-width: 760px) {
  .site-nav {
    position: static;
    width: auto;
    border-right: 0;
    border-bottom: 1px solid var(--border);
  }
  main {
    margin: 0;
    padding: 18px;
  }
  article {
    padding: 24px;
  }
}
`;

await mkdir(outDir, { recursive: true });
await writeFile(path.join(outDir, "style.css"), css);

for (const [slug, title, file] of docs) {
  const markdown = await readFile(file, "utf8");
  const rendered = renderMarkdown(markdown);
  await writeFile(path.join(outDir, `${slug}.html`), page(title, slug, rendered.body, rendered.headings));
}

console.log(`Rendered ${docs.length} docs to ${outDir}`);
