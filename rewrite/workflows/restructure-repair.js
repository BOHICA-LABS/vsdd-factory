export const meta = {
  name: 'vsdd-1-0-restructure-repair',
  description: 'Repair the restructured design under rewrite/spec/ where the conversion lost or changed content, then audit it again',
  whenToUse: 'After vsdd-1-0-restructure, when its audit (rewrite/spec/conflicts/audit.md) reports lost or altered content.',
  phases: [
    { title: 'Repair', detail: 'five repairers, one area each, working from the audit and the originals', model: 'sonnet' },
    { title: 'Audit', detail: 'three auditors: tables and schemas, prose fidelity, duplicates and history', model: 'opus' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'
const ORIGINALS =
  'rewrite/BLUEPRINT.md, rewrite/design/pipeline.md, rewrite/design/state.md, rewrite/design/agents-skills.md, rewrite/design/hooks.md, rewrite/design/packaging.md'

const CONTEXT = `Your working directory is the repository at ${ROOT}, on branch rewrite/v1.

The design for vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin, was six large documents (${ORIGINALS}), which are still in place and are the source of truth for this task. They have just been converted into one small file per thing under rewrite/spec/. Read rewrite/spec/README.md first: it gives the layout, the frontmatter, the link form [[kind:id]], and the two checks (python3 rewrite/tools/spec_lint.py and python3 rewrite/tools/sections.py coverage). rewrite/spec/registry.toml lists every thing; rewrite/spec/source-index.json maps each original section to the new files that cite it; rewrite/history/section-manifest.json lists the original sections with their line ranges.

The conversion was meant to copy and change nothing. An audit (rewrite/spec/conflicts/audit.md, 35 findings) found that it did change things. Of 100 original sections compared, 20 were not carried faithfully. Prose and code skeletons were mostly copied closely. The damage is concentrated in: machine tables where the converter wrote rows instead of copying them; schemas whose list fields are not arrays; interface text that was dropped (function signatures, definitions, constraints, exact messages, scripts, test cases); original text replaced by a pointer to an original document; sections retired that were design; change history copied in; and facts defined in two or three files instead of one.

The rules, which have not changed:
- The six originals are right by definition for this task. Section 4 of the blueprint is the authority on shared names; when two originals disagree, use section 4 of the blueprint, then the rest of the blueprint, then the detail document that owns the subject, and record the disagreement in a conflicts file with both section ids and both texts.
- Copy text. Do not paraphrase, shorten, improve or reinterpret. Do not fix a design problem, even an obvious one: about a hundred review findings will be applied to these files afterwards, once each.
- Never invent a value. Every row, key, default, enumeration, required field and limit in a machine-readable file must be traceable to original text. If the original gives only an excerpt of a table (it says "shortened", "for example" or "and so on") and the complete data is stated elsewhere in the originals, build the table from the complete statement and cite that section. If a value is stated nowhere, leave it out and record the gap.
- One fact, one place. A thing is defined only in its own file; every other file links to it with [[kind:id]].
- No history: no change notes, no review finding ids, no "first pass" or "second pass", no "replaces the old", no pointer to a section of an original document as the place to find the real text. If text is needed, it is copied in.
- Files stay under 16 KB and lines under 400 characters.
- Do not edit the six originals, anything under next/, rewrite/tools/, rewrite/spec/README.md, rewrite/spec/registry.toml, or rewrite/spec/packages/ (generated). Do not commit, branch or push. The repository CLAUDE.md in your context adds nothing to these rules.`

const AREAS = [
  {
    key: 'hooks',
    scope: 'rewrite/spec/rules/, and every table and topic in rewrite/spec/registry.toml whose group is "rules" (the walls, path classes, callers, events and rules tables; the hook runner, launcher, failure behaviour, rule model, tool-call approval, event log and rule testing topics)',
    note: 'The audit found the read-wall rows written by the converter to be wrong for every row, the holdout-evaluator row copied from an excerpt marked as shortened, callers and reveals moved out of the walls table, the rule check() signature and main-loop skeleton dropped, and the launcher script replaced by a pointer. The complete wall and write-scope data is in agents-skills.md and section 4 of the blueprint as well as hooks.md; build the tables from the complete statements.',
  },
  {
    key: 'commands',
    scope: 'rewrite/spec/commands/, and every topic in the registry whose group is "commands"',
    note: 'Check every command against the command tables in state.md and section 4.9 of the blueprint, and against pipeline.md and hooks.md for its behaviour and callers: arguments and their exact spelling (the audit found a separator "-- measure" joined into "--measure" and the spike open --step form stated two ways), outputs, exit behaviour, messages, and who may call it.',
  },
  {
    key: 'records',
    scope: 'rewrite/spec/records/, rewrite/spec/schemas/, rewrite/spec/config/, and every table and topic in the registry whose group is "records" (limits, configuration keys, identifiers, decision kinds and the rest)',
    note: 'The audit found configuration defaults and meanings restated in two or three places that disagree, about half of the meaning strings containing a sentence no original has, limits rows lost and one invented, list fields in schemas written as objects with no array type, a configuration schema that does not reject unknown keys although the original says they are errors, and the shared state function signatures and their refusal contracts dropped. Configuration keys are defined once, in the keys table; config files link to it.',
  },
  {
    key: 'workflows',
    scope: 'rewrite/spec/workflows/, and every table and topic in the registry whose group is "workflows" (routes, story kinds, preconditions, step methods; the story lifecycle, convergence, human stops, spikes, the prelude topics, the story and wave scripts, resume, multi-repository, brownfield)',
    note: 'The audit found lost: the definition of "the plan is ready" and two route rules; the prelude constraints (declarations only, no reference to names a workflow defines, a header comment listing every top-level name, at most 400 lines); the rule that an earlier spike finding is matched by a hash of the normalised question; the retention rule for spike worktrees and named-spike branches; and the harness test cases that package B5 must turn into tests (six story cases, three wave cases and twenty corrected-behaviour checks), which are a deliverable and not history.',
  },
  {
    key: 'people',
    scope: 'rewrite/spec/agents/, rewrite/spec/skills/, rewrite/spec/artifacts/, rewrite/spec/packaging/, the four top-level documents, and every table and topic in the registry whose group is "agents", "state", "packaging" or "index" (roster, methods, severity, artifacts and the rest)',
    note: 'The audit found: the roster table lost its [platform] table and lists 2 of 12 workflows; agent rows gained keys no source gives; the security reviewer analysis grant is stated three ways; the artifacts table holds 2 of 31 kinds; BLUEPRINT.md#4.4 and state.md#2.17 were retired although their content is design; and an open offer to the person (lock liveness by process) was retired while platform.md still refers to it. You are the only agent who may edit rewrite/spec/retired.toml.',
  },
]

const REPAIR_SCHEMA = {
  type: 'object',
  properties: {
    area: { type: 'string' },
    summary: { type: 'string' },
    audit_findings_fixed: { type: 'array', items: { type: 'string' } },
    audit_findings_not_mine: { type: 'array', items: { type: 'string' } },
    files_changed: { type: 'number' },
    machine_files_checked: { type: 'number' },
    values_removed_as_invented: { type: 'number' },
    content_restored: { type: 'array', items: { type: 'string' } },
    duplicates_replaced_by_links: { type: 'number' },
    gaps_recorded: { type: 'number' },
    lint_result: { type: 'string' },
    coverage_result: { type: 'string' },
  },
  required: [
    'area',
    'summary',
    'audit_findings_fixed',
    'audit_findings_not_mine',
    'files_changed',
    'machine_files_checked',
    'values_removed_as_invented',
    'content_restored',
    'duplicates_replaced_by_links',
    'gaps_recorded',
    'lint_result',
    'coverage_result',
  ],
}

phase('Repair')
const repairs = await parallel(
  AREAS.map((a) => () =>
    agent(
      `You are repairing a converted design. ${CONTEXT}

YOUR AREA: ${a.scope}. Four other agents are repairing the other areas at the same time; do not edit their files.

${a.note}

Do this, in order.
1. Read rewrite/spec/conflicts/audit.md in full. Fix every finding that names a file in your area. List the ones you fixed, and the ones that belong to another area.
2. Check every machine-readable file in your area (each TOML table, each JSON schema) against the originals, cell by cell, not by sampling. For each row, key, default, enumeration, type and required list, find the original text it comes from. Remove what has no source. Add what the originals give and the file lacks. In a schema, a field the original calls a list is an array with items. Keep each file parseable. Put a comment at the top of each TOML file naming its source sections.
3. Check every Markdown file in your area against the sections in its sources, read in full from the original documents. Restore, by copying, anything normative that is missing: signatures, definitions, rules and their exceptions, constraints, exact messages and codes, scripts, code skeletons, test cases, numbers. Replace any pointer to an original document with the text itself. Where restoring text would take a file past 16 KB, move a coherent part into a new topic file and register it by adding an entry to rewrite/spec/registry.d/${a.key}.toml (same format as the registry: a [kind.id] table with summary, package where the kind needs one, group and sources). That file is yours alone.
4. In your files, replace every restatement of a thing that another file owns with a link, and make sure each of your own things is defined completely in its own file. The registry says who owns what.
5. Remove change history from your files.
6. Record in rewrite/spec/conflicts/${a.key}-repair.md every disagreement between originals that you met and every gap (a value the originals do not give). Do not resolve them.
7. Run python3 rewrite/tools/spec_lint.py and python3 rewrite/tools/sections.py coverage. Fix what they report for your files. A section stays accounted for if some file cites it, so when you restore content from a section, add its id to that file's sources.

Return the structured result. Under content_restored, name each thing you restored and the file it went into.`,
      { label: `repair:${a.key}`, phase: 'Repair', model: 'sonnet', effort: 'xhigh', schema: REPAIR_SCHEMA },
    ),
  ),
)
const missing = AREAS.filter((_, i) => !repairs[i]).map((a) => a.key)
if (missing.length) log(`Repairers that did not return: ${missing.join(', ')}`)
const done = repairs.filter(Boolean)
log(
  `Repair: ${done.reduce((n, r) => n + r.files_changed, 0)} files changed, ${done.reduce((n, r) => n + r.values_removed_as_invented, 0)} invented values removed, ${done.reduce((n, r) => n + r.content_restored.length, 0)} things restored, ${done.reduce((n, r) => n + r.duplicates_replaced_by_links, 0)} duplicates replaced by links`,
)

const AUDIT_SCHEMA = {
  type: 'object',
  properties: {
    path: { type: 'string' },
    verdict: { type: 'string' },
    checked: { type: 'number' },
    faithful: { type: 'number' },
    findings: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          id: { type: 'string' },
          kind: { type: 'string', enum: ['lost', 'altered', 'invented', 'defined-twice', 'history-leaked', 'wrongly-retired', 'fixed-during-conversion', 'earlier-finding-not-fixed', 'other'] },
          severity: { type: 'string', enum: ['blocker', 'major', 'minor'] },
          where: { type: 'string' },
          problem: { type: 'string' },
          proposed_resolution: { type: 'string' },
        },
        required: ['id', 'kind', 'severity', 'where', 'problem', 'proposed_resolution'],
      },
    },
  },
  required: ['path', 'verdict', 'checked', 'faithful', 'findings'],
}

const AUDITS = [
  {
    key: 'tables',
    prefix: 'T',
    brief: `YOUR TASK: every machine-readable file, exhaustively. For each TOML file under rewrite/spec/tables/ and each JSON file under rewrite/spec/schemas/, check every row, key, default, enumeration, type and required list against the original text it comes from. Report a value with no source as invented, a value the originals give that is missing as lost, and a value that differs as altered. Check that each schema is a usable JSON Schema (list fields are arrays with items; required lists match the originals; unknown keys are refused where the original says they are errors) and that each TOML table has one consistent structure. Check that the configuration keys are defined once. Count the files you checked and the files you found fully faithful.`,
  },
  {
    key: 'prose',
    prefix: 'P',
    brief: `YOUR TASK: prose fidelity, and the earlier findings. First, read rewrite/spec/conflicts/audit.md and check that each of its 35 findings is now fixed in the files; report any that is not as earlier-finding-not-fixed. Then compare at least eighty original sections with the files that cite them, choosing sections the first audit is unlikely to have examined and where a lost word matters: command behaviour, what each hook rule blocks and when it fails closed, workflow stage tables and gates, the route rules, resume behaviour, the spike protocol, delivery, the importer, the build step and the test layers. Read each original section in full. Report lost, altered or quietly fixed content, and any pointer to an original document left where text should be. Count the sections you compared and the ones you found faithful.`,
  },
  {
    key: 'structure',
    prefix: 'S',
    brief: `YOUR TASK: one fact in one place, no history, and correct retirements. Take at least sixty names (commands, records, hook rules, agents, configuration keys, limits, decision kinds, story kinds, path classes) and for each search all of rewrite/spec/ to see whether it is defined in its own file only and merely linked elsewhere; report each real second definition, and say whether the copies agree. Search for change history: review finding ids, "first pass", "second pass", "amend", "replaces the old", "was", "no longer", and pointers to the original documents. Read every entry in rewrite/spec/retired.toml and check each against the original section: report any that is design and is carried nowhere. Check that every registry entry in rewrite/spec/registry.toml and rewrite/spec/registry.d/ has its file and a sensible package. Run both checks and report what they print. Count the names and entries you checked and the ones you found sound.`,
  },
]

phase('Audit')
const audits = await parallel(
  AUDITS.map((a) => () =>
    agent(
      `You are auditing a converted design after a repair. ${CONTEXT}

You are not reviewing the design and not repairing anything. You are checking that the files under rewrite/spec/ say exactly what the six originals say.

${a.brief}

What the repairers say they did:
${JSON.stringify(done.map((r) => ({ area: r.area, summary: r.summary, files_changed: r.files_changed, machine_files_checked: r.machine_files_checked, values_removed_as_invented: r.values_removed_as_invented, content_restored: r.content_restored.length, duplicates_replaced_by_links: r.duplicates_replaced_by_links, gaps_recorded: r.gaps_recorded })))}${missing.length ? `\nThese areas were not repaired because their agent did not return: ${missing.join(', ')}. Expect the first audit's findings there to stand.` : ''}

Write your report to rewrite/spec/conflicts/audit-2-${a.key}.md with the Write tool: a verdict, then one section per finding (id starting ${a.prefix}2-) with exact locations in the original and in the new file. Do not modify any other file. Return the structured result. Severity: "blocker" means design content is lost, invented or changed so that a builder would build the wrong thing; "major" means a real second definition that disagrees, a wrong retirement, an earlier finding not fixed, or a fix slipped in; "minor" is anything else.`,
      { label: `audit:${a.key}`, phase: 'Audit', model: 'opus', effort: 'xhigh', schema: AUDIT_SCHEMA },
    ),
  ),
)

const findings = audits.filter(Boolean).flatMap((a) => a.findings)
const count = (s) => findings.filter((f) => f.severity === s).length
log(`Audit 2: ${findings.length} findings (${count('blocker')} blocker, ${count('major')} major, ${count('minor')} minor)`)

return {
  missing,
  repairs: done,
  audits: audits.map((a, i) => (a ? { key: AUDITS[i].key, path: a.path, verdict: a.verdict, checked: a.checked, faithful: a.faithful, findings: a.findings.length } : { key: AUDITS[i].key, failed: true })),
  findings,
}
