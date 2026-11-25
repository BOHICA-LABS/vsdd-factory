---
kind: doc
id: decisions-later
sources: [decisions.md#D31, decisions.md#D32, decisions.md#D33]
---

# Decisions adopted after 2026-11-23

The decisions D1 to D30 are in [[doc:decisions]]. These were adopted later, each on the date it gives, and
are fixed in the same way. Under each, "Decided" is what was decided and "What the answer changes" records
what the other answer would have changed. [[topic:decisions-built]] says where each one is built.

### D31

- **Question.** Should the factory treat feature flags as a first-class concern of the software it builds: a
  sound flag system in the product, and features delivered through it? This is not about flags inside
  vsdd-factory.
- **Decided.** Yes, adopted 2026-11-24, opt-in per project. The product gets one flag interface (OpenFeature
  where its language supports it), a record of every flag, safe defaults when the flag source cannot be
  reached and a documented kill switch; a story can ship behind a flag, default off. The factory uses an
  established system and you confirm which, once per project: one already in the code, then one you or the
  brief name, then one the architect researches and recommends. Rolling our own is the last resort and needs
  your explicit approval, with the reason in the decision record. The factory writes flag definitions into
  the repository; it never creates or changes flags in the outside service, keys stay out of the repository,
  and tests do not reach the live service. It adds no workflow. Operating flags in production is out of
  scope. It is written into the design now and built when the build widens past the thin slice. The whole
  of it is [[topic:feature-flags]].
- **What the answer changes.** Without it, flags stay the two lines of advice the old plugin had: work can
  merge only when it is safe to expose, and nothing checks that a flag is recorded, tested in both states or
  ever removed. It adds six build packages, F1 to F6, and nothing on the thin slice path.

### D32

- **Question.** The hook runner approves an agent's own single `vsdd` call so that it can attach the marker
  that says who is calling ([[topic:caller-identity]]). Should `hooks.rules.auto-allow.scope = "off"`, the
  default under D4, stop that approval?
- **Decided.** No, adopted 2026-11-24. That approval is outside the key. The key still governs every other
  call the plugin may approve, the main session is still asked, and D4 otherwise stands ([[config:hooks]]).
- **What the answer changes.** If `off` stopped it, every unattended run would stop to ask on each `vsdd`
  call, because the platform applies the rewrite that carries the marker only when the hook also approves.

### D33

- **Question.** Should several people, each with their own machine, clone and Claude Code session, be able
  to run the factory against one shared repository, at the same time or at different times, as a normal
  development team does, with a result as sound as when one person runs it?
- **Decided.** Yes, adopted 2026-11-24, in eight clauses.
  1. Several people, each with their own machine, clone and Claude Code session, run the factory against
     one shared repository, at the same time or at different times, as a normal development team does.
     The result must be as sound as when one person runs it.
  2. The Git remote is the only thing they share and the only coordination channel. There is no server,
     hosted service or other channel.
  3. Everything is bound by what git can do, in your words "this should be bound within the physics
     achievable with git". Coordination and hand-off rest on fetch, push and the atomic update of a ref,
     and promise nothing stronger. Picking up work that was begun on another machine continues from what
     has been pushed.
  4. What the factory must provide:
     - parallel work: two people can work on different stories, or different stages, without blocking
       each other or corrupting shared state;
     - no double work: everyone can see who is working on what, and two machines cannot both take the
       same story, wave or specification change;
     - current state: a run starts from what the team has already pushed, not from a stale local copy;
     - shared decisions: a question that waits for a person can be seen and answered by a teammate on
       another machine;
     - attribution: records say which person started a run, approved a specification, settled a decision
       or merged;
     - team delivery: stories reach the shared branch through pull requests and CI, not a merge on one
       person's machine;
     - matching setups: every machine runs a compatible plugin version and the same project settings;
       data that belongs to one machine stays there.
  5. The factory records who did what. It does not enforce who may approve, merge or release, in your
     words "a properly setup repo should enforce this". No roles and no permission model.
  6. It must cope with teammates who commit by hand, without the plugin, to the product.
  7. Not meant: several people sharing one session or one working copy; the multi-repository layout,
     which is a separate feature that already exists.
  8. It is designed now, so that the state code being built for the thin slice does not bake in the
     assumption of one machine. The thin slice stays one story on one machine and nothing it does
     changes. Team use is built when the build widens past the thin slice, and its proof is a test on
     two clones of one remote.

  The whole of it, and where each clause ends as a check, is [[topic:team-use]].
- **What the answer changes.** Without it, two clones meet only when state is pushed: nothing stops both
  from building the same story, a run starts from whatever its clone last saw, and a commit made by hand is
  invisible to every gate. It adds six build packages, M1 to M6, and nothing on the thin slice path.
