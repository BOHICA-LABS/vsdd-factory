export const meta = {
  name: 'vsdd-1-0-restructure',
  description: 'Restructure the vsdd-factory 1.0 design from six large documents into one file per thing under rewrite/spec/',
  whenToUse: 'Run once, after rewrite/tools/sections.py manifest and gen_packages.py. Copies the design into rewrite/spec/ without changing its meaning.',
  phases: [
    { title: 'Index', detail: 'the registry of every thing, and the four top-level documents', model: 'fable' },
    { title: 'Extract', detail: 'seven extractors, one group of files each', model: 'sonnet' },
    { title: 'Close', detail: 'make the layout check and the coverage check pass', model: 'sonnet' },
    { title: 'Audit', detail: 'sample the result for lost or altered content', model: 'opus' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'
const ORIGINALS =
  'rewrite/BLUEPRINT.md, rewrite/design/pipeline.md, rewrite/design/state.md, rewrite/design/agents-skills.md, rewrite/design/hooks.md, rewrite/design/packaging.md'

const CONTEXT = `Your working directory is the repository at ${ROOT}, on branch rewrite/v1.

The design for vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin, is today six large documents: ${ORIGINALS}. Together they are about 980 KB. The same fact is written in several of them, so three rounds of fixes each left some copies uncorrected. The design is being restructured into one small file per thing under rewrite/spec/, so that each fact has one home.

Read rewrite/spec/README.md first. It gives the rules of the new layout: the kinds and their folders, the frontmatter, the link form [[kind:id]], the size limits, and the two checks (python3 rewrite/tools/spec_lint.py and python3 rewrite/tools/sections.py coverage). Section ids for the sources field are in rewrite/history/section-manifest.json: each has the form <document>#<section number>, for example pipeline.md#2.9.4 or BLUEPRINT.md#4.9.

This is a conversion, not a redesign and not a review. The rules of the conversion:
- Copy text. Do not paraphrase, shorten, improve or reinterpret it. A builder will implement from these files, so a changed word is a changed design.
- Section 4 of the blueprint is the authority on shared names. When two documents state the same fact differently, use section 4 of the blueprint, then the rest of the blueprint, then the detail document that owns the subject. Record every such disagreement in your conflicts file, with both section ids and both texts, and say which you used. Do not resolve a disagreement any other way.
- Do not fix anything, even something obviously wrong. Review reports exist with about a hundred open findings; they will be applied to the new files afterwards, in one place each. Fixing as you go would make that impossible to check.
- One fact, one place. Define a thing only in its own file. Anywhere else, link to it with [[kind:id]] and do not restate it.
- Leave history behind: amendment lists and notes, tables of review findings, statements of what changed from an earlier version, and mappings from the old system are not design. Do not copy them; report their section ids so they can be recorded as retired.
- Keep files under 16 KB and lines under 400 characters. A long table row becomes a short list under a heading. If a thing needs more than 16 KB, split it into the thing's file and linked topic files, and report that.
- The repository CLAUDE.md that was loaded into your context may be short or may describe the old system; either way its only instruction that matters here is: do not commit, branch or push.
- Do not edit the six original documents, anything under next/, anything under rewrite/tools/, rewrite/spec/README.md, or rewrite/spec/packages/ (those briefs are generated).`

// ---------------------------------------------------------------- Index
const INDEX_SCHEMA = {
  type: 'object',
  properties: {
    summary: { type: 'string' },
    counts: { type: 'string' },
    groups: {
      type: 'array',
      items: {
        type: 'object',
        properties: { group: { type: 'string' }, things: { type: 'number' } },
        required: ['group', 'things'],
      },
    },
    doubts: { type: 'array', items: { type: 'string' } },
  },
  required: ['summary', 'counts', 'groups', 'doubts'],
}

phase('Index')
const index = await agent(
  `You are building the index for a restructured design. ${CONTEXT}

YOUR TASK has two parts.

PART 1: the registry. Extend rewrite/spec/registry.toml so that it lists every thing in the design, once. The file already holds a generated block of [package.<ID>] entries between two marker comments; leave that block exactly as it is and add your entries after it.

Each entry is a TOML table named [<kind>.<id>] with these keys:
- summary: one line saying what the thing is.
- package: the id of the build package that delivers it, from section 9 of the blueprint. Required for the kinds command, record, schema, rule, agent, skill, workflow, artifact, table and config. Use "none" only if no package delivers it.
- group: which extractor writes its file. One of: commands, records, rules, agents, workflows, state, packaging, index.
- sources: a list of the section ids where the thing is mainly defined, so the extractor knows where to start.

What to register, and the group that owns each kind:
- command (group commands): every vsdd command and subcommand in section 4.9 of the blueprint and in the command tables of state.md. One entry per command a person or agent can type, with the id made from its words joined by hyphens (gate-red, story-deliver, spike-open). Group the gate names as one entry each.
- record, schema, config (group records): every machine record type; every JSON schema (the result schemas of section 4.10 and pipeline.md, and a schema for each record type and each structured tool output the design gives fields for); every group of project configuration keys. Also the tables limits, config-keys and identifiers, and topics that explain evidence, digests and decisions as records.
- rule (group rules): every hook rule. Also the tables walls, path-classes, callers and events (hook wiring), and topics for the hook runner, its launcher and failure behaviour, the rule model, the approval of tool calls, the event log, and how rules are tested.
- agent, skill (group agents): every agent and every skill. Also the table roster (tier, effort, tools, package for each agent), the table methods (each method file and the agent that owns it), and topics for the model tiers, the agent file template, the skill file template, and how research and browser tools reach agents.
- workflow (group workflows): every workflow. Also the tables routes and story-kinds, and topics for the life of a story, the review loop and convergence, human stops and decisions as a person meets them, research spikes, the workflow prelude and its functions, how workflows resume, multi-repository projects, and existing code and finished builds (brownfield).
- artifact (group state): every kind of project artifact, with its path, identifier, frontmatter and writer. Also the table artifacts, and topics for the state directory and its storage, who commits state and when, the status record and the session briefing, compaction and cold start, the lock, the importer, templates, and multi-repository state.
- packaging (group packaging): repository layout, the plugin manifest, the build step, each test layer, continuous integration, release, cutover, each operator tool, the documentation set, and the build plan order.
- doc (group index): overview, decisions, conventions, platform. You write these four yourself in part 2.

Read the blueprint in full; it names nearly every thing. Then read the tables of contents and the main tables of the five detail documents to catch things the blueprint only summarises. It is better to register a thing that turns out to be small than to leave something with no home. Use lowercase ids with hyphens. Keep topic files focused: a topic that would run past 16 KB should be several topics.

PART 2: the four top-level documents, each with frontmatter (kind: doc, the id, and sources).
- rewrite/spec/overview.md: what 1.0 is and how the parts fit, from sections 1 and 3 of the blueprint. Link to things; do not define them here.
- rewrite/spec/decisions.md: the record of the adopted decisions from section 2 of the blueprint, as decided.
- rewrite/spec/conventions.md: names, paths, model tiers and identifier rules from section 4 of the blueprint (4.1, 4.2, 4.8 and any naming rule elsewhere in section 4). If it passes 16 KB, split off a second doc and register it.
- rewrite/spec/platform.md: the platform facts proven and not yet proven, from section 10 of the blueprint.

Then run python3 rewrite/tools/gen_packages.py (it regenerates the package briefs from your registry) and python3 rewrite/tools/spec_lint.py. At this stage the check will report a missing file for every thing the extractors have yet to write; that is expected. Fix every other kind of problem it reports: an unknown kind, a bad id, a missing summary or package, a package that is not in the build plan.

Return the structured result: counts as one line (how many of each kind), the number of things in each group, and under doubts anything you could not place with confidence.`,
  { label: 'index', phase: 'Index', model: 'fable', effort: 'xhigh', schema: INDEX_SCHEMA },
)
if (!index) {
  log('The indexer did not return; stopping, because the extractors need the registry')
  return { failed: 'index' }
}
log(`Registry written: ${index.counts}`)

// ---------------------------------------------------------------- Extract
const GROUPS = [
  { key: 'commands', folders: 'rewrite/spec/commands/', note: 'Each command file gives: what it does, who may call it, its arguments, what it reads and writes (link the records), its output (link the schema), its exit behaviour and messages, and its failure cases. The main sources are the command tables in state.md and section 4.9 of the blueprint, with the gate and delivery behaviour described in pipeline.md and the caller rules in hooks.md.' },
  { key: 'records', folders: 'rewrite/spec/records/, rewrite/spec/schemas/, rewrite/spec/config/, and your tables and topics', note: 'Write real JSON Schema files, valid JSON, one per schema, copying the fields, types, enumerations and required lists the design gives; where the design gives a field list but no types or required list, write what is given, add nothing, and record the gap in your conflicts file. Write the configuration keys as a TOML table (key, type, default, meaning, section) and keep the prose in config files short.' },
  { key: 'rules', folders: 'rewrite/spec/rules/, and your tables and topics', note: 'Each rule file gives: the events and tools it runs on, what it checks, what it returns, whether it fails closed, how a project may relax it, and its messages. Write the walls, path classes and caller permissions as TOML tables. The main source is hooks.md.' },
  { key: 'agents', folders: 'rewrite/spec/agents/, rewrite/spec/skills/, and your tables and topics', note: 'Each agent file gives: role, tier and effort, tool list, what it is given, the result it returns (link the schema), its walls and write scopes (link the tables), and its methods. Each skill file gives: purpose, who invokes it, its frontmatter, and what it does. Write the roster and the method list as TOML tables. The main source is agents-skills.md.' },
  { key: 'workflows', folders: 'rewrite/spec/workflows/, and your tables and topics', note: 'Each workflow file gives: purpose, arguments, its stages in order (step, agent, what it produces, its gate), where it stops for a person, how it resumes, and what it returns. Link commands, agents and schemas; do not restate them. Keep the JavaScript skeletons the design gives, in code blocks, in the prelude topics and in the workflow they belong to. Write routes and story kinds as TOML tables. The main source is pipeline.md.' },
  { key: 'state', folders: 'rewrite/spec/artifacts/, and your tables and topics', note: 'Each artifact file gives: path, identifier scheme, frontmatter fields, which agent role writes it, size cap, and lifecycle. Commands and machine records belong to other extractors: link to them. The main source is state.md, without its command and record tables.' },
  { key: 'packaging', folders: 'rewrite/spec/packaging/', note: 'The main source is packaging.md, with sections 5.5 and 9 of the blueprint. The per-package briefs in rewrite/spec/packages/ are generated from the build plan table and the registry; do not write or edit them. Put the detail that packaging.md gives about how to build or test a particular package into the packaging file for that subject, not into a brief.' },
]

const EXTRACT_SCHEMA = {
  type: 'object',
  properties: {
    group: { type: 'string' },
    files_written: { type: 'number' },
    summary: { type: 'string' },
    not_found: { type: 'array', items: { type: 'string' } },
    not_in_registry: {
      type: 'array',
      items: {
        type: 'object',
        properties: { kind: { type: 'string' }, id: { type: 'string' }, summary: { type: 'string' }, package: { type: 'string' }, file: { type: 'string' } },
        required: ['kind', 'id', 'summary', 'package', 'file'],
      },
    },
    history_sections: { type: 'array', items: { type: 'string' } },
    conflicts: { type: 'number' },
    lint_result: { type: 'string' },
  },
  required: ['group', 'files_written', 'summary', 'not_found', 'not_in_registry', 'history_sections', 'conflicts', 'lint_result'],
}

phase('Extract')
const extracts = await parallel(
  GROUPS.map((g) => () =>
    agent(
      `You are one of seven extractors restructuring a design. ${CONTEXT}

YOUR GROUP: ${g.key}. Open rewrite/spec/registry.toml and find every entry whose group is "${g.key}". Those are your things, and yours alone: write one file for each, in ${g.folders}. The other six extractors are writing the other groups at the same time, so link to their things by the ids in the registry and trust that the files will exist.

${g.note}

For each of your things:
1. Start from the sections listed in its registry entry, then search all six original documents for its name, because parts of its definition are usually in two or three of them.
2. Write its file with frontmatter (kind, id, package where the kind needs one, and sources listing every section you took text from), then the definition, copied.
3. Replace any restatement of someone else's thing with a link.

Also write rewrite/spec/conflicts/${g.key}.md: one entry for every place where two documents disagreed about one of your things, and for every gap you noticed while copying (a field with no type, a name used but never defined). Give section ids and quote both texts. This file is the input to the next fixing step, so be exact and do not editorialise.

Do not edit rewrite/spec/registry.toml: seven agents share it. If you find a thing in your area that the registry lacks, write its file anyway and report it under not_in_registry. If a registry entry in your group has no text anywhere, do not invent any: report it under not_found.

When your files are written, run python3 rewrite/tools/spec_lint.py --only <each of your folders> and fix what it reports for your files: frontmatter, bad links, files over 16 KB, lines over 400 characters. Then return the structured result. Under history_sections, list the ids of sections in your sources that are history and that you deliberately did not copy.`,
      { label: `extract:${g.key}`, phase: 'Extract', model: 'sonnet', effort: 'xhigh', schema: EXTRACT_SCHEMA },
    ),
  ),
)
const missingGroups = GROUPS.filter((_, i) => !extracts[i]).map((g) => g.key)
if (missingGroups.length) log(`Extractors that did not return: ${missingGroups.join(', ')}`)
const done = extracts.filter(Boolean)
log(`Extraction: ${done.reduce((n, e) => n + e.files_written, 0)} files, ${done.reduce((n, e) => n + e.conflicts, 0)} conflicts recorded`)

// ---------------------------------------------------------------- Close
const CLOSE_SCHEMA = {
  type: 'object',
  properties: {
    summary: { type: 'string' },
    lint: { type: 'string' },
    coverage: { type: 'string' },
    registered: { type: 'number' },
    retired: { type: 'number' },
    content_added: { type: 'array', items: { type: 'string' } },
    unresolved: { type: 'array', items: { type: 'string' } },
  },
  required: ['summary', 'lint', 'coverage', 'registered', 'retired', 'content_added', 'unresolved'],
}

phase('Close')
const closed = await agent(
  `You are closing out a restructure of a design. ${CONTEXT}

Seven extractors have written the files under rewrite/spec/. ${missingGroups.length ? `These groups did not finish and their files may be missing or partial: ${missingGroups.join(', ')}. Write the missing files for those groups yourself, by the same rules, before anything else. ` : ''}Your job is to make the two checks pass without changing the meaning of anything.

What the extractors reported:
${JSON.stringify(done.map((e) => ({ group: e.group, not_found: e.not_found, not_in_registry: e.not_in_registry, history_sections: e.history_sections, lint_result: e.lint_result })))}

Do this in order.
1. Register what the extractors found outside the registry: add an entry to rewrite/spec/registry.toml for each item under not_in_registry, after checking that the file exists and the id is not already taken under another name. You are the only agent editing the registry now.
2. For each item under not_found, search the six original documents once more. If the thing has text, write its file. If it has none, remove its registry entry and list it under unresolved.
3. Run python3 rewrite/tools/gen_packages.py, then python3 rewrite/tools/spec_lint.py, and fix every problem until it reports none. Fix a broken link by correcting the id it should point to; do not delete a link to make the check pass.
4. Run python3 rewrite/tools/sections.py coverage. It lists every section of the original documents that no file cites. For each one, read the section and do exactly one of these:
   - its content is already in a spec file: add the section id to that file's sources;
   - it is history (a table of review findings, an amendment list, a mapping from the old system, a statement of what changed): add it to rewrite/spec/retired.toml as an entry of the array "retired" with id and reason = "history";
   - it only repeats another section: add it to retired.toml with reason = "duplicate of <section id>";
   - its content is design and is in no spec file: copy it into the file where it belongs, or a new registered topic, cite it, and list it under content_added.
   Also add the sections the extractors reported as history, after checking each really is.
   The format of retired.toml is:
   [[retired]]
   id = "BLUEPRINT.md#8"
   reason = "history"
5. Run both checks again until both are clean, then run python3 rewrite/tools/sections.py coverage --write-index, which writes the index from old section to new files.

Return the structured result with the last line each check printed. List under unresolved anything you could not settle.`,
  { label: 'close', phase: 'Close', model: 'sonnet', effort: 'xhigh', schema: CLOSE_SCHEMA },
)

// ---------------------------------------------------------------- Audit
const AUDIT_SCHEMA = {
  type: 'object',
  properties: {
    path: { type: 'string' },
    verdict: { type: 'string' },
    lint: { type: 'string' },
    coverage: { type: 'string' },
    sections_compared: { type: 'number' },
    faithful: { type: 'number' },
    findings: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          id: { type: 'string' },
          kind: { type: 'string', enum: ['lost', 'altered', 'defined-twice', 'history-leaked', 'wrongly-retired', 'fixed-during-conversion', 'other'] },
          severity: { type: 'string', enum: ['blocker', 'major', 'minor'] },
          where: { type: 'string' },
          problem: { type: 'string' },
          proposed_resolution: { type: 'string' },
        },
        required: ['id', 'kind', 'severity', 'where', 'problem', 'proposed_resolution'],
      },
    },
  },
  required: ['path', 'verdict', 'lint', 'coverage', 'sections_compared', 'faithful', 'findings'],
}

phase('Audit')
const audit = await agent(
  `You are auditing a restructure of a design. ${CONTEXT}

Agents have just converted the six original documents into about two hundred small files under rewrite/spec/. They were told to copy and not to change anything. Your job is to find where they did not: content that was lost, content whose meaning changed, a fact now defined in two files, history that was copied in, a section retired that was really design, and a problem that was quietly fixed during the conversion. You are not reviewing the design itself.

Do this.
1. Run python3 rewrite/tools/spec_lint.py and python3 rewrite/tools/sections.py coverage and record what each prints last.
2. Compare at least sixty sections of the original documents with the files that cite them (rewrite/spec/source-index.json maps each old section to its new files). Choose the sections where a lost or changed word would matter most: command behaviour, hook rules and what they block, schemas and their required fields, workflow stage tables and gates, walls and write scopes, configuration keys and defaults, limits, and the route table. Spread them across all six documents. For each, read the original section in full and the new file or files, and decide whether the new text carries everything normative in the old.
3. Read every entry in rewrite/spec/retired.toml whose reason is "history" or "duplicate" and check twenty of them against the original.
4. Search rewrite/spec/ for facts defined twice. Take thirty names of commands, records, rules and configuration keys and check that each is defined in its own file only, and merely linked elsewhere.
5. Check that every JSON file under rewrite/spec/schemas/ is a usable JSON Schema and that the TOML tables hold what their names say.

Write your report to rewrite/spec/conflicts/audit.md with the Write tool: a verdict, then one section per finding with exact locations in both the original and the new files. Do not modify any other file. Return the structured result. Severity: "blocker" means design content was lost or changed and a builder would build the wrong thing; "major" means a real duplicate, a wrong retirement, or a fix slipped in; "minor" is anything else.`,
  { label: 'audit', phase: 'Audit', model: 'opus', effort: 'xhigh', schema: AUDIT_SCHEMA },
)

return {
  index: { counts: index.counts, groups: index.groups, doubts: index.doubts },
  extracts: done.map((e) => ({ group: e.group, files: e.files_written, conflicts: e.conflicts, not_found: e.not_found.length, not_in_registry: e.not_in_registry.length, lint: e.lint_result })),
  missingGroups,
  closed,
  audit,
}
