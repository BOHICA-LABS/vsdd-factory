---
kind: topic
id: glossary
sources: [BLUEPRINT.md#3.1, pipeline.md#2.1, state.md#2.1, agents-skills.md#2.1, hooks.md#2.1]
---

# Glossary

The terms the design uses. A term that names a thing with its own file is only linked here, because that file
defines it. A term with no file of its own is defined here, in the blueprint's words where the blueprint defines
it and otherwise in the words of the design document that introduced it.

**Agent.** See [[topic:agent-file]], [[table:roster]].

**Agent-written artifact.** An artifact whose text an agent or a person writes with the Edit or Write tool: a
contract, a story, a review report. Code validates it before the write lands. See [[topic:artifact-rules]].

**Artifact.** One file in the tracked part of the state directory. Its path decides its kind. See
[[table:artifacts]].

**Baseline.** A baseline is what a cycle starts from: a release, an ingest or an import, together with the
measurements taken at that point. See [[record:baseline]].

**Checkpoint.** See [[command:checkpoint]].

**Claim.** Two things, told apart by where the word stands. In review data (`claims`, `claim_checks`,
[[schema:claim]]) it is a fixer's structured statement about one finding, handed to the next reviewer.
In the files of team use, in the check names that begin `T-` and in the commands `vsdd claim take` and
`vsdd claim release` it is a work claim: the file on the state branch that says which clone has a story,
a cycle's run, the specification set or the opening of a cycle (decision D33; built when the build
widens). Tables, records, schemas and fields of the second kind are named `work-claim` or `work_claims`
in full. See [[record:work-claim]], [[topic:team-claims]].

**Clerk step.** A clerk step runs one `vsdd` command and relays its JSON output. See [[agent:clerk]],
[[topic:prelude-steps]].

**Clone.** One copy of the repository on one machine, with its own state mount, sessions and local store
(decision D33; built when the build widens). See [[record:clone]].

**Closure record.** See [[record:closure]].

**Cycle.** A cycle is one pass of the pipeline over a body of work, from a brief or a change request to a
release. See [[record:cycle]], [[topic:cycles]].

**Decision.** A decision is a question only a person can answer, stored with a packet a person can read. See
[[record:decision]], [[topic:decisions]], [[topic:human-stops]].

**Derived view.** Anything computed from artifacts on request and never written back: a catalogue, a count, a
traceability table. See [[topic:stored-and-derived]].

**Evidence record.** An evidence record is the file a gate writes. See [[record:evidence]],
[[topic:evidence-lookup]].

**Factory project.** A project whose root holds the project configuration file (`.factory/factory.toml`,
[[doc:conventions]]). See [[topic:state-directory]], [[topic:hooks-inert]].

**Finding.** See [[schema:finding]].

**Flush.** See [[topic:state-commits]].

**Gate.** A gate is a fact the tool measures, such as "the new tests fail" or "the suite passes on this commit".
The hook design also uses the word for a kind of rule: "A rule that can block. An advisor can only add text. A
recorder only writes records." See [[topic:gates]] for the first sense, [[topic:rule-model]] for the second.

**Hand commit.** A commit to the product that the factory did not make (decision D33; built when the build
widens). See [[topic:team-delivery]].

**Integration branch.** The integration branch is the branch stories merge into; its name comes from project
configuration. See [[config:branches]].

**Launcher.** See [[topic:hook-launcher]].

**Local store.** See [[doc:conventions]], [[topic:state-layout]].

**Machine record.** A JSON file that only the `vsdd` command or the hook runner writes. No tool call by an agent
can write one. See [[topic:machine-records]].

**Main session.** The conversation a person is typing into. Hooks see it with an empty `agent_type`. See
[[topic:information-walls]].

**Method file.** See [[topic:method-files]], [[table:methods]].

**Model-visible skill.** A skill whose description is listed to the model so the model can invoke it. Every
agent that has the Skill tool pays for that listing on every start. A skill with
`disable-model-invocation: true` is not listed and only a person can run it. See [[topic:skill-file]].

**Outcome.** See [[topic:rule-model]].

**Pass, round.** One fresh review of the current commit at a perimeter. At the wave and release perimeters a
round is one run of every check that is due. See [[topic:convergence]].

**Path class.** See [[table:path-classes]].

**Perimeter.** A perimeter is the scope of a review: story, wave or release. See [[topic:convergence]].

**Position.** How far a story has got: `ready`, `red`, `green`, `reviewed`, `evidenced`, `delivered`. See
[[topic:story-life]].

**Registry.** See [[table:artifacts]].

**Route.** The order in which workflows run for a given project state. The tool computes it and reports it as
`next`. See [[table:routes]].

**Row.** The line of the wall table selected by the calling agent's type (`agent_type` in the payload). See
[[table:walls]].

**Rule.** See [[topic:rule-model]], [[table:rules]].

**Runner.** See [[topic:hook-runner]].

**Skill.** See [[topic:skill-file]].

**Stage.** A named part of a workflow that produces one set of artifacts. A stage that is already done is
skipped. Stages are the unit of resume. See [[topic:workflow-shape]], [[topic:resume]].

**State branch.** See [[topic:state-directory]].

**State directory.** See [[doc:conventions]], [[topic:state-directory]].

**Step, step record.** A step is one agent run inside a workflow, and a step record is the stored copy of what
it returned. See [[schema:step-result]], [[record:step]], [[topic:prelude-steps]].

**Story behind a flag.** A story whose new behaviour is reachable only with a feature flag on (decision D31;
built when the build widens). A story's own boolean keys, such as `security`, are what the design calls
flagged. See [[artifact:flag]].

**Survey.** See [[command:survey]].

**Sync.** The one step in which a clone's state and the remote's are brought together: fetch, combine, push
(decision D33; built when the build widens). See [[topic:team-sync]], [[command:sync]].

**Tier.** See [[doc:conventions]].

**Unit.** A unit is the thing a workflow works on: a story, a wave (a group of stories built in parallel), the
specification set, a release candidate. See [[record:closure]].

**`vsdd`.** See [[doc:overview]], [[topic:command-conventions]].

**Wall.** A rule that an agent type may not read a class of paths. See [[topic:information-walls]],
[[table:walls]].

**Workflow.** See [[doc:overview]], [[topic:workflow-shape]].

**Workflow step.** One `agent()` call in a workflow script. It runs an agent type and gets a structured result
back. A workflow step has no Agent tool (verified by the platform spike). See [[topic:prelude-steps]].

**Write scope.** The classes of paths an agent type may change with Write or Edit. See
[[topic:information-walls]], [[table:walls]].
