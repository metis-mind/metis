---
name: chinese-doc-style
description: "Use when writing, rewriting, proofreading or reviewing Chinese prose in this repo — docs/ is Chinese-canonical (ADR-0012): ADRs, design docs, research reports. Enforces fidelity-first rewriting, restrained tone, consistent terminology, and house typography (「」 quotes, half-width spacing, one paragraph per line). Adapted from Fenng/Tech-Doc-Style-Chinese (MIT)."
---

# Chinese doc style (docs/ house style)

The repo's language split (ADR-0012): code, rustdoc, AGENTS.md, skills, README
and CONTRIBUTING are English; `docs/` is Chinese-canonical. This skill is the
writing discipline for that Chinese side. It governs _how existing prose is
written_ — whether a piece of prose is allowed to exist at all is the
`doc-maintenance` skill's three-question filter, not this file's business.

## Protected content (never restyled)

Code literals, JSON keys, URLs, API paths, commands, config keys, file paths,
database fields and any other machine-readable identifier stay byte-identical
— inline-code them in Chinese prose. Quotations, error-message originals and
user-supplied fixed copy likewise stay verbatim; deliver suggestions
separately instead of rewriting them.

Accepted ADR bodies are frozen (AGENTS.md → Documentation): never restyle or
edit them — corrections land as a superseding ADR, with only the old ADR's
Status line gaining a pointer.

## Priority order when rules conflict

1. Preserve facts, logic, constraints, safety and legal meaning.
2. Obey explicit user instructions and project conventions (AGENTS.md, the
   frozen ADRs).
3. Keep terminology and machine-readable content accurate.
4. Improve structure, semantics, tone and scannability.
5. Only then polish punctuation, spacing, capitalization.

Never trade a higher layer for a lower one — no sentence gets shorter, more
certain, or more uniform at the cost of facts or constraints.

## Fidelity (事实保真)

- Add no dates, numbers, deadlines, capabilities, conditions, causal claims or
  conclusions that the source or reliable context does not provide.
- Drop no preconditions, applicability limits, exceptions, risks, warnings,
  compatibility notes or failure handling.
- Never upgrade hedged wording (可能 / 计划 / 建议 / 通常) into certainty.
- When information is missing, keep the original meaning or mark 「待确认」 —
  do not fill gaps by invention.

## Task modes (decide first)

- **撰写** — fix audience, content type, fact sources and length first; mark
  fact gaps that would change conclusions, then organize what is confirmable.
- **改写** — preserve facts, logic, information hierarchy, constraints and
  necessary exceptions; deliver the full rewrite, flag semantic judgment
  calls separately.
- **校对** — only the agreed scope: typos, punctuation, spacing,
  capitalization, term consistency. Do not touch structure, tone or facts
  unless asked.
- **审阅** — list problems first (ordered by impact, quoting the original
  lines), then suggestions; never edit the file without being asked.

## Semantics and tone

- Accuracy before rhetoric; clarity before liveliness. Restrained, direct,
  executable Chinese.
- Lead with what it is, what it applies to, what to do, and where to read
  next.
- No greeting-style openers, slogans, empty adjectives, or stacked internet
  jargon (赋能 / 抓手 / 闭环 / 打通 …).
- Default to no direct reader address; when a role is needed, name it
  concretely (「维护者」「插件作者」).
- Replace ambiguous 该 / 其 / 此 / 上述 with the concrete name whenever the
  referent could be in doubt.

## Structure and sentences

- One paragraph carries one main point; one sentence keeps one clear backbone
  — no stacked chains of conditions, actions and exceptions.
- List items share the same level, sentence pattern and information density.
- Headings state purpose, not bare abstract nouns.
- Delete repeated information; never delete facts, conditions or exceptions.

## Terminology and typography

- One concept, one preferred term — never rotate synonyms for variety (this
  is the prose face of the repo's 定义唯一住处 rule).
- Chinese quotes are corner brackets: 「」 (nested: 『』).
- One half-width space between Chinese characters and Latin words / digits /
  version numbers, and around inline code: 「使用 API 获取数据」「等待 3 秒」.
  No space around full-width punctuation. Spacing around a link is judged by
  its display text, not the raw URL. Never adjust spacing inside code, link
  targets, official names or user-specified literals.
- One paragraph per line in the source file — no hard wrapping for line
  width (dprint `textWrap: maintain` preserves this for the files it covers;
  `docs/research/` is excluded from dprint — enforce typography by hand
  there).
- Product names, abbreviations and official spellings follow project or
  official usage — no mechanical replacement.
- No blind batch find-replace: protect code blocks and machine-readable
  spans first.

## Workflow

Run the steps the task mode selects — 校对 runs only steps 1, 2, 6 and 7,
within the agreed scope.

1. Confirm task mode, content type, audience and project conventions.
2. Mark facts, numbers, constraints, quotations and machine-readable content
   as untouchable boundaries.
3. Fix semantic errors, ambiguity, omissions, inconsistencies.
4. Adjust information order, paragraphs, headings, lists.
5. Polish tone, terminology, punctuation, spacing, capitalization.
6. Re-check against the original: facts, conditions, negation scope, causal
   relations, degree of certainty.
7. Run `just fmt` and read the result with `git diff -w` (research reports
   get no dprint pass — check their typography by hand).

## Final checklist

- Nothing source-less added (dates, numbers, deadlines, capabilities,
  conclusions)?
- No condition, exception, risk, unit, default or recovery path dropped?
- Subjects, objects, negation scope, causality and certainty still accurate?
- Same concept, same term throughout?
- No duplicated message across heading / body / list?
- Code, paths, fields, commands and quotations byte-identical?
- Scannable and executable for the intended reader?
- Paragraphs free of hard wraps, with correct CJK/Latin spacing at joins?

---

Adapted from <https://github.com/Fenng/Tech-Doc-Style-Chinese> (MIT,
Copyright (c) 2026 Fenng), trimmed to this repo's doc types (ADR / design /
research); its UI-copy, API-status, product-page and controlled-technical-
writing (操作与故障排查) modules were deliberately not absorbed — no such
content lives in `docs/`.
