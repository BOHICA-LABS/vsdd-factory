// workflow-lib/prelude.js — the shared code stamped into every workflow script.
// tools/build.py regenerates the two marked sub-blocks below (generated:tables, generated:schemas)
// from tests/lint/roster.toml and <plugin>/schemas/*.json, then copies this whole file between the
// ">>> stamped:prelude" markers of every workflow. The 400-line cap excludes the generated sub-blocks.
//
// Top-level names introduced (everything else a script sees is a platform global):
//   RUN, CLOSE_RESERVE, TOKEN_FLOOR, LIMIT_KEYS, applyLimits, safe, enc, parseArgs,
//   stepSchema, made, brief, runSpike, step, gated, relay, runStories,
//   STR_ARR, OBJ, KV, S_REVIEW, S_RESOLUTION, S_FIXES, S_RULINGS, SCORE, SYMPTOM,
//   S_HOLDOUT, S_PRECONDITION, S_NEXT, S_SURVEY, S_CLOSE, S_DELIVERY, S_WAVE_GATE,
//   HARD, REACH, judge, converge, holdoutFindings
// plus the generated constants (TIER, ROLE, OVERRIDE, the S_* schema literals).
// Constraints (packaging.md 2.5): declarations only; no import/export; no clocks.

// >>> generated:tables
const TIER = {
  "planner": {
    "model": "fable",
    "effort": "xhigh"
  },
  "builder": {
    "model": "sonnet",
    "effort": "xhigh"
  },
  "reviewer": {
    "model": "opus",
    "effort": "xhigh"
  },
  "clerk": {
    "model": "sonnet",
    "effort": "low"
  }
};
const ROLE = {
  "product-owner": "planner",
  "architect": "planner",
  "ux-designer": "planner",
  "story-writer": "planner",
  "codebase-analyzer": "planner",
  "researcher": "planner",
  "spike-runner": "planner",
  "test-writer": "builder",
  "implementer": "builder",
  "verifier": "builder",
  "demo-recorder": "builder",
  "technical-writer": "builder",
  "devops": "builder",
  "adversary": "reviewer",
  "code-reviewer": "reviewer",
  "security-reviewer": "reviewer",
  "holdout-evaluator": "reviewer",
  "ui-reviewer": "reviewer",
  "scenario-reviewer": "reviewer",
  "clerk": "clerk"
};
// The only steps that run off their agent's tier (decision D5). step() consults it by "<workflow>/<stage>";
// a script cannot mark one of its own, and lint rejects any override the roster does not list.
const OVERRIDE = {
  "ingest/map": "builder",
  "ingest/analyze": "builder"
};
// <<< generated:tables

// >>> generated:schemas
const S_FINDING = {
  "type": "object",
  "properties": {
    "key": {
      "type": "string",
      "maxLength": 200,
      "description": "Stable identity, <category>:<path>:<symbol or clause>. The same defect has the same key in every pass."
    },
    "title": {
      "type": "string",
      "maxLength": 160
    },
    "severity": {
      "type": "string",
      "enum": [
        "critical",
        "high",
        "medium",
        "low"
      ]
    },
    "category": {
      "type": "string",
      "enum": [
        "spec-fidelity",
        "test",
        "code",
        "security",
        "integration",
        "performance",
        "accessibility",
        "ux",
        "docs",
        "spec",
        "traceability",
        "holdout",
        "formal",
        "coverage"
      ]
    },
    "scope": {
      "type": "string",
      "enum": [
        "unit",
        "integration",
        "system",
        "spec"
      ]
    },
    "owner": {
      "type": "string",
      "enum": [
        "product-owner",
        "architect",
        "ux-designer",
        "story-writer",
        "test-writer",
        "implementer",
        "verifier",
        "technical-writer",
        "devops",
        "codebase-analyzer",
        "demo-recorder"
      ],
      "description": "The roster agent that should fix it. Fixes are routed on this field."
    },
    "location": {
      "type": "object",
      "properties": {
        "path": {
          "type": "string",
          "description": "Relative to the root of the reviewed tree: the unit's worktree for a story, the integration worktree for a wave or a release, the project root for files under .factory/."
        },
        "line_start": {
          "type": "integer"
        },
        "line_end": {
          "type": "integer"
        },
        "quote": {
          "type": "string",
          "maxLength": 400
        }
      },
      "required": [
        "path"
      ]
    },
    "spec_ref": {
      "type": "string",
      "description": "Contract clause or acceptance criterion violated; empty if none."
    },
    "repro": {
      "type": "string",
      "maxLength": 800,
      "description": "Command or scenario that shows the defect; empty if none."
    },
    "observed": {
      "type": "string",
      "maxLength": 600
    },
    "expected": {
      "type": "string",
      "maxLength": 600
    },
    "suggestion": {
      "type": "string",
      "maxLength": 600
    },
    "confidence": {
      "type": "number",
      "minimum": 0,
      "maximum": 1
    }
  },
  "required": [
    "key",
    "title",
    "severity",
    "category",
    "scope",
    "owner",
    "location",
    "spec_ref",
    "repro",
    "observed",
    "expected"
  ]
};
const S_GATE = {
  "type": "object",
  "properties": {
    "ok": {
      "type": "boolean",
      "description": "False when the command itself could not run."
    },
    "state": {
      "type": "string",
      "enum": [
        "done",
        "pending"
      ],
      "description": "pending: the job is still running; run the same command again. Absent means done."
    },
    "diff_patch": {
      "type": "string",
      "description": "Green and wave gates: path of the patch the gate wrote for the commit it measured. Given to the next reviewer."
    },
    "gate": {
      "type": "string"
    },
    "subject": {
      "type": "string"
    },
    "passed": {
      "type": "boolean"
    },
    "sha": {
      "type": "string",
      "description": "Commit the gate measured."
    },
    "evidence": {
      "type": "string",
      "description": "Path of the evidence record."
    },
    "metrics": {
      "type": "object"
    },
    "failures": {
      "type": "array",
      "items": {
        "type": "string",
        "maxLength": 400
      },
      "maxItems": 20
    },
    "flaky": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "description": "Identities of inherited tests that failed and then passed on the second run (section 2.9.8). Copied into the unit's closure record."
    },
    "findings": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "key": {
            "type": "string",
            "maxLength": 200,
            "description": "Stable identity, <category>:<path>:<symbol or clause>. The same defect has the same key in every pass."
          },
          "title": {
            "type": "string",
            "maxLength": 160
          },
          "severity": {
            "type": "string",
            "enum": [
              "critical",
              "high",
              "medium",
              "low"
            ]
          },
          "category": {
            "type": "string",
            "enum": [
              "spec-fidelity",
              "test",
              "code",
              "security",
              "integration",
              "performance",
              "accessibility",
              "ux",
              "docs",
              "spec",
              "traceability",
              "holdout",
              "formal",
              "coverage"
            ]
          },
          "scope": {
            "type": "string",
            "enum": [
              "unit",
              "integration",
              "system",
              "spec"
            ]
          },
          "owner": {
            "type": "string",
            "enum": [
              "product-owner",
              "architect",
              "ux-designer",
              "story-writer",
              "test-writer",
              "implementer",
              "verifier",
              "technical-writer",
              "devops",
              "codebase-analyzer",
              "demo-recorder"
            ],
            "description": "The roster agent that should fix it. Fixes are routed on this field."
          },
          "location": {
            "type": "object",
            "properties": {
              "path": {
                "type": "string",
                "description": "Relative to the root of the reviewed tree: the unit's worktree for a story, the integration worktree for a wave or a release, the project root for files under .factory/."
              },
              "line_start": {
                "type": "integer"
              },
              "line_end": {
                "type": "integer"
              },
              "quote": {
                "type": "string",
                "maxLength": 400
              }
            },
            "required": [
              "path"
            ]
          },
          "spec_ref": {
            "type": "string",
            "description": "Contract clause or acceptance criterion violated; empty if none."
          },
          "repro": {
            "type": "string",
            "maxLength": 800,
            "description": "Command or scenario that shows the defect; empty if none."
          },
          "observed": {
            "type": "string",
            "maxLength": 600
          },
          "expected": {
            "type": "string",
            "maxLength": 600
          },
          "suggestion": {
            "type": "string",
            "maxLength": 600
          },
          "confidence": {
            "type": "number",
            "minimum": 0,
            "maximum": 1
          }
        },
        "required": [
          "key",
          "title",
          "severity",
          "category",
          "scope",
          "owner",
          "location",
          "spec_ref",
          "repro",
          "observed",
          "expected"
        ]
      }
    },
    "unit": {
      "type": "object",
      "description": "Only from vsdd gate plan --story, and always when it passes: the admitted story unit (section 2.10.8)."
    },
    "error": {
      "type": "string"
    }
  },
  "required": [
    "ok",
    "gate",
    "subject",
    "passed",
    "sha",
    "evidence",
    "failures"
  ]
};
const S_GATE_OUTPUT = {
  "type": "object",
  "anyOf": [
    {
      "type": "object",
      "properties": {
        "ok": {
          "type": "boolean",
          "description": "False when the command itself could not run."
        },
        "state": {
          "type": "string",
          "enum": [
            "done",
            "pending"
          ],
          "description": "pending: the job is still running; run the same command again. Absent means done."
        },
        "diff_patch": {
          "type": "string",
          "description": "Green and wave gates: path of the patch the gate wrote for the commit it measured. Given to the next reviewer."
        },
        "gate": {
          "type": "string"
        },
        "subject": {
          "type": "string"
        },
        "passed": {
          "type": "boolean"
        },
        "sha": {
          "type": "string",
          "description": "Commit the gate measured."
        },
        "evidence": {
          "type": "string",
          "description": "Path of the evidence record."
        },
        "metrics": {
          "type": "object"
        },
        "failures": {
          "type": "array",
          "items": {
            "type": "string",
            "maxLength": 400
          },
          "maxItems": 20
        },
        "flaky": {
          "type": "array",
          "items": {
            "type": "string"
          },
          "description": "Identities of inherited tests that failed and then passed on the second run (section 2.9.8). Copied into the unit's closure record."
        },
        "findings": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "key": {
                "type": "string",
                "maxLength": 200,
                "description": "Stable identity, <category>:<path>:<symbol or clause>. The same defect has the same key in every pass."
              },
              "title": {
                "type": "string",
                "maxLength": 160
              },
              "severity": {
                "type": "string",
                "enum": [
                  "critical",
                  "high",
                  "medium",
                  "low"
                ]
              },
              "category": {
                "type": "string",
                "enum": [
                  "spec-fidelity",
                  "test",
                  "code",
                  "security",
                  "integration",
                  "performance",
                  "accessibility",
                  "ux",
                  "docs",
                  "spec",
                  "traceability",
                  "holdout",
                  "formal",
                  "coverage"
                ]
              },
              "scope": {
                "type": "string",
                "enum": [
                  "unit",
                  "integration",
                  "system",
                  "spec"
                ]
              },
              "owner": {
                "type": "string",
                "enum": [
                  "product-owner",
                  "architect",
                  "ux-designer",
                  "story-writer",
                  "test-writer",
                  "implementer",
                  "verifier",
                  "technical-writer",
                  "devops",
                  "codebase-analyzer",
                  "demo-recorder"
                ],
                "description": "The roster agent that should fix it. Fixes are routed on this field."
              },
              "location": {
                "type": "object",
                "properties": {
                  "path": {
                    "type": "string",
                    "description": "Relative to the root of the reviewed tree: the unit's worktree for a story, the integration worktree for a wave or a release, the project root for files under .factory/."
                  },
                  "line_start": {
                    "type": "integer"
                  },
                  "line_end": {
                    "type": "integer"
                  },
                  "quote": {
                    "type": "string",
                    "maxLength": 400
                  }
                },
                "required": [
                  "path"
                ]
              },
              "spec_ref": {
                "type": "string",
                "description": "Contract clause or acceptance criterion violated; empty if none."
              },
              "repro": {
                "type": "string",
                "maxLength": 800,
                "description": "Command or scenario that shows the defect; empty if none."
              },
              "observed": {
                "type": "string",
                "maxLength": 600
              },
              "expected": {
                "type": "string",
                "maxLength": 600
              },
              "suggestion": {
                "type": "string",
                "maxLength": 600
              },
              "confidence": {
                "type": "number",
                "minimum": 0,
                "maximum": 1
              }
            },
            "required": [
              "key",
              "title",
              "severity",
              "category",
              "scope",
              "owner",
              "location",
              "spec_ref",
              "repro",
              "observed",
              "expected"
            ]
          }
        },
        "unit": {
          "type": "object",
          "description": "Only from vsdd gate plan --story, and always when it passes: the admitted story unit (section 2.10.8)."
        },
        "error": {
          "type": "string"
        }
      },
      "required": [
        "ok",
        "gate",
        "subject",
        "passed",
        "sha",
        "evidence",
        "failures"
      ]
    },
    {
      "type": "object",
      "properties": {
        "ok": {
          "type": "boolean"
        },
        "state": {
          "type": "string",
          "enum": [
            "pending"
          ],
          "description": "The job is still running; run the same command again."
        },
        "retry": {
          "type": "string",
          "description": "The same command to run again."
        }
      },
      "required": [
        "ok",
        "state",
        "retry"
      ]
    }
  ]
};
const S_REVIEW_DATA = {
  "type": "object",
  "properties": {
    "reviewed_sha": {
      "type": "string",
      "description": "The commit the pass reviewed."
    },
    "findings": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "key": {
            "type": "string",
            "maxLength": 200,
            "description": "Stable identity, <category>:<path>:<symbol or clause>. The same defect has the same key in every pass."
          },
          "title": {
            "type": "string",
            "maxLength": 160
          },
          "severity": {
            "type": "string",
            "enum": [
              "critical",
              "high",
              "medium",
              "low"
            ]
          },
          "category": {
            "type": "string",
            "enum": [
              "spec-fidelity",
              "test",
              "code",
              "security",
              "integration",
              "performance",
              "accessibility",
              "ux",
              "docs",
              "spec",
              "traceability",
              "holdout",
              "formal",
              "coverage"
            ]
          },
          "scope": {
            "type": "string",
            "enum": [
              "unit",
              "integration",
              "system",
              "spec"
            ]
          },
          "owner": {
            "type": "string",
            "enum": [
              "product-owner",
              "architect",
              "ux-designer",
              "story-writer",
              "test-writer",
              "implementer",
              "verifier",
              "technical-writer",
              "devops",
              "codebase-analyzer",
              "demo-recorder"
            ],
            "description": "The roster agent that should fix it. Fixes are routed on this field."
          },
          "location": {
            "type": "object",
            "properties": {
              "path": {
                "type": "string",
                "description": "Relative to the root of the reviewed tree: the unit's worktree for a story, the integration worktree for a wave or a release, the project root for files under .factory/."
              },
              "line_start": {
                "type": "integer"
              },
              "line_end": {
                "type": "integer"
              },
              "quote": {
                "type": "string",
                "maxLength": 400
              }
            },
            "required": [
              "path"
            ]
          },
          "spec_ref": {
            "type": "string",
            "description": "Contract clause or acceptance criterion violated; empty if none."
          },
          "repro": {
            "type": "string",
            "maxLength": 800,
            "description": "Command or scenario that shows the defect; empty if none."
          },
          "observed": {
            "type": "string",
            "maxLength": 600
          },
          "expected": {
            "type": "string",
            "maxLength": 600
          },
          "suggestion": {
            "type": "string",
            "maxLength": 600
          },
          "confidence": {
            "type": "number",
            "minimum": 0,
            "maximum": 1
          }
        },
        "required": [
          "key",
          "title",
          "severity",
          "category",
          "scope",
          "owner",
          "location",
          "spec_ref",
          "repro",
          "observed",
          "expected"
        ]
      }
    },
    "claim_checks": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "key": {
            "type": "string"
          },
          "outcome": {
            "type": "string",
            "enum": [
              "resolved",
              "not_resolved",
              "withdrawn"
            ]
          },
          "note": {
            "type": "string"
          }
        },
        "required": [
          "key",
          "outcome"
        ]
      }
    },
    "coverage": {
      "type": "array",
      "description": "One entry per review item. state is checked, not_checkable (the note is the reason) or skipped.",
      "items": {
        "type": "object",
        "properties": {
          "item": {
            "type": "string"
          },
          "state": {
            "type": "string",
            "enum": [
              "checked",
              "not_checkable",
              "skipped"
            ]
          },
          "note": {
            "type": "string",
            "description": "Required: the reason for the state, so a not_checkable item never stands unexplained."
          }
        },
        "required": [
          "item",
          "state",
          "note"
        ]
      }
    }
  },
  "required": [
    "reviewed_sha",
    "findings",
    "claim_checks",
    "coverage"
  ]
};
const S_CLAIM = {
  "type": "object",
  "properties": {
    "key": {
      "type": "string"
    },
    "path": {
      "type": "string",
      "description": "The location's path, for a claim that is not about a holdout finding."
    },
    "line_start": {
      "type": "integer"
    },
    "line_end": {
      "type": "integer"
    },
    "resolution": {
      "type": "string",
      "enum": [
        "fixed",
        "disputed"
      ]
    },
    "commit": {
      "type": "string"
    }
  },
  "required": [
    "key",
    "resolution",
    "commit"
  ]
};
const S_FIX_DATA = {
  "type": "object",
  "properties": {
    "resolutions": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "key": {
            "type": "string"
          },
          "resolution": {
            "type": "string",
            "enum": [
              "fixed",
              "disputed"
            ]
          },
          "commit": {
            "type": "string"
          },
          "note": {
            "type": "string",
            "description": "Kept in the step record for the arbiter only; it never reaches the next reviewer."
          }
        },
        "required": [
          "key",
          "resolution"
        ]
      }
    }
  },
  "required": [
    "resolutions"
  ]
};
const S_RULING_DATA = {
  "type": "object",
  "properties": {
    "rulings": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "key": {
            "type": "string"
          },
          "ruling": {
            "type": "string",
            "enum": [
              "defect",
              "not_defect",
              "spec_gap"
            ]
          },
          "reason": {
            "type": "string"
          }
        },
        "required": [
          "key",
          "ruling",
          "reason"
        ]
      }
    }
  },
  "required": [
    "rulings"
  ]
};
const S_HOLDOUT_DATA = {
  "type": "object",
  "properties": {
    "scores": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "scenario": {
            "type": "string"
          },
          "satisfaction": {
            "type": "number",
            "minimum": 0,
            "maximum": 1
          },
          "must_pass": {
            "type": "boolean"
          },
          "ran": {
            "type": "boolean"
          }
        },
        "required": [
          "scenario",
          "satisfaction",
          "must_pass",
          "ran"
        ]
      }
    },
    "symptoms": {
      "type": "array",
      "description": "Observed and expected behaviour; no location and no scenario text.",
      "items": {
        "type": "object",
        "properties": {
          "scenario": {
            "type": "string"
          },
          "observed": {
            "type": "string"
          },
          "expected": {
            "type": "string"
          }
        },
        "required": [
          "scenario",
          "observed",
          "expected"
        ]
      }
    }
  },
  "required": [
    "scores",
    "symptoms"
  ]
};
const S_INTAKE_DATA = {
  "type": "object",
  "properties": {
    "covers": {
      "type": "array",
      "items": {
        "type": "string"
      }
    }
  },
  "required": [
    "covers"
  ]
};
const S_STORY_DRAFT_DATA = {
  "type": "object",
  "properties": {
    "units": {
      "type": "array",
      "description": "Story units in the shape section 2.10.8 fixes; the script passes each to runStories().",
      "items": {
        "type": "object"
      }
    }
  },
  "required": [
    "units"
  ]
};
const S_SPIKE_REQUEST = {
  "type": "object",
  "properties": {
    "question": {
      "type": "string",
      "maxLength": 400,
      "description": "One question an experiment can answer."
    },
    "why_blocking": {
      "type": "string",
      "maxLength": 400
    },
    "kind": {
      "type": "string",
      "enum": [
        "experiment",
        "research"
      ],
      "description": "experiment runs code; research reads documentation and sources. Both run in an isolated worktree, so every runner has its own working directory."
    },
    "method_hint": {
      "type": "string",
      "maxLength": 600
    },
    "ref": {
      "type": "string",
      "description": "Branch or commit to start from; empty means the commit the asking step left."
    },
    "done_when": {
      "type": "string",
      "maxLength": 300
    }
  },
  "required": [
    "question",
    "why_blocking",
    "kind"
  ]
};
const S_SPIKE_FINDING = {
  "type": "object",
  "properties": {
    "ledger_id": {
      "type": "string",
      "description": "Id returned by vsdd spike open; empty only when the runner returned nothing."
    },
    "question": {
      "type": "string"
    },
    "verdict": {
      "type": "string",
      "enum": [
        "confirmed",
        "refuted",
        "answered",
        "inconclusive",
        "refused"
      ],
      "description": "The spike ledger in the state design uses this same list."
    },
    "answer": {
      "type": "string",
      "maxLength": 1200
    },
    "method": {
      "type": "string",
      "maxLength": 800
    },
    "evidence": {
      "type": "array",
      "items": {
        "type": "string",
        "maxLength": 300
      },
      "maxItems": 10
    },
    "confidence": {
      "type": "number",
      "minimum": 0,
      "maximum": 1
    },
    "cached": {
      "type": "boolean",
      "description": "True when the ledger already held this answer and no experiment ran."
    },
    "artifacts": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "description": "Paths in the kept worktree, for reference only."
    },
    "follow_up": {
      "type": "string",
      "maxLength": 400
    }
  },
  "required": [
    "ledger_id",
    "question",
    "verdict",
    "answer",
    "method",
    "evidence",
    "confidence",
    "cached"
  ]
};
const S_STEP_RESULT = {
  "type": "object",
  "properties": {
    "step": {
      "type": "object",
      "description": "Identity given in the prompt, returned unchanged. The step record is stored under it.",
      "properties": {
        "run": {
          "type": "string"
        },
        "workflow": {
          "type": "string"
        },
        "name": {
          "type": "string"
        },
        "subject": {
          "type": "string"
        },
        "attempt": {
          "type": "integer",
          "minimum": 1
        }
      },
      "required": [
        "run",
        "workflow",
        "name",
        "subject",
        "attempt"
      ]
    },
    "status": {
      "type": "string",
      "enum": [
        "done",
        "needs_spike",
        "blocked",
        "failed"
      ]
    },
    "summary": {
      "type": "string",
      "maxLength": 600
    },
    "head": {
      "type": "string",
      "description": "Commit the step left its branch at; empty if it made none."
    },
    "artifacts": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "maxItems": 50
    },
    "gates": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "ok": {
            "type": "boolean",
            "description": "False when the command itself could not run."
          },
          "state": {
            "type": "string",
            "enum": [
              "done",
              "pending"
            ],
            "description": "pending: the job is still running; run the same command again. Absent means done."
          },
          "diff_patch": {
            "type": "string",
            "description": "Green and wave gates: path of the patch the gate wrote for the commit it measured. Given to the next reviewer."
          },
          "gate": {
            "type": "string"
          },
          "subject": {
            "type": "string"
          },
          "passed": {
            "type": "boolean"
          },
          "sha": {
            "type": "string",
            "description": "Commit the gate measured."
          },
          "evidence": {
            "type": "string",
            "description": "Path of the evidence record."
          },
          "metrics": {
            "type": "object"
          },
          "failures": {
            "type": "array",
            "items": {
              "type": "string",
              "maxLength": 400
            },
            "maxItems": 20
          },
          "flaky": {
            "type": "array",
            "items": {
              "type": "string"
            },
            "description": "Identities of inherited tests that failed and then passed on the second run (section 2.9.8). Copied into the unit's closure record."
          },
          "findings": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "key": {
                  "type": "string",
                  "maxLength": 200,
                  "description": "Stable identity, <category>:<path>:<symbol or clause>. The same defect has the same key in every pass."
                },
                "title": {
                  "type": "string",
                  "maxLength": 160
                },
                "severity": {
                  "type": "string",
                  "enum": [
                    "critical",
                    "high",
                    "medium",
                    "low"
                  ]
                },
                "category": {
                  "type": "string",
                  "enum": [
                    "spec-fidelity",
                    "test",
                    "code",
                    "security",
                    "integration",
                    "performance",
                    "accessibility",
                    "ux",
                    "docs",
                    "spec",
                    "traceability",
                    "holdout",
                    "formal",
                    "coverage"
                  ]
                },
                "scope": {
                  "type": "string",
                  "enum": [
                    "unit",
                    "integration",
                    "system",
                    "spec"
                  ]
                },
                "owner": {
                  "type": "string",
                  "enum": [
                    "product-owner",
                    "architect",
                    "ux-designer",
                    "story-writer",
                    "test-writer",
                    "implementer",
                    "verifier",
                    "technical-writer",
                    "devops",
                    "codebase-analyzer",
                    "demo-recorder"
                  ],
                  "description": "The roster agent that should fix it. Fixes are routed on this field."
                },
                "location": {
                  "type": "object",
                  "properties": {
                    "path": {
                      "type": "string",
                      "description": "Relative to the root of the reviewed tree: the unit's worktree for a story, the integration worktree for a wave or a release, the project root for files under .factory/."
                    },
                    "line_start": {
                      "type": "integer"
                    },
                    "line_end": {
                      "type": "integer"
                    },
                    "quote": {
                      "type": "string",
                      "maxLength": 400
                    }
                  },
                  "required": [
                    "path"
                  ]
                },
                "spec_ref": {
                  "type": "string",
                  "description": "Contract clause or acceptance criterion violated; empty if none."
                },
                "repro": {
                  "type": "string",
                  "maxLength": 800,
                  "description": "Command or scenario that shows the defect; empty if none."
                },
                "observed": {
                  "type": "string",
                  "maxLength": 600
                },
                "expected": {
                  "type": "string",
                  "maxLength": 600
                },
                "suggestion": {
                  "type": "string",
                  "maxLength": 600
                },
                "confidence": {
                  "type": "number",
                  "minimum": 0,
                  "maximum": 1
                }
              },
              "required": [
                "key",
                "title",
                "severity",
                "category",
                "scope",
                "owner",
                "location",
                "spec_ref",
                "repro",
                "observed",
                "expected"
              ]
            }
          },
          "unit": {
            "type": "object",
            "description": "Only from vsdd gate plan --story, and always when it passes: the admitted story unit (section 2.10.8)."
          },
          "error": {
            "type": "string"
          }
        },
        "required": [
          "ok",
          "gate",
          "subject",
          "passed",
          "sha",
          "evidence",
          "failures"
        ]
      }
    },
    "blocked": {
      "type": "object",
      "properties": {
        "kind": {
          "type": "string",
          "enum": [
            "spec_gap",
            "environment",
            "spike_limit",
            "gate",
            "agent_budget",
            "conflict",
            "reviewer_blocked",
            "other"
          ]
        },
        "reason": {
          "type": "string",
          "maxLength": 600
        },
        "question": {
          "type": "string",
          "maxLength": 400
        }
      },
      "required": [
        "kind",
        "reason"
      ]
    },
    "spike_request": {
      "type": "object",
      "properties": {
        "question": {
          "type": "string",
          "maxLength": 400,
          "description": "One question an experiment can answer."
        },
        "why_blocking": {
          "type": "string",
          "maxLength": 400
        },
        "kind": {
          "type": "string",
          "enum": [
            "experiment",
            "research"
          ],
          "description": "experiment runs code; research reads documentation and sources. Both run in an isolated worktree, so every runner has its own working directory."
        },
        "method_hint": {
          "type": "string",
          "maxLength": 600
        },
        "ref": {
          "type": "string",
          "description": "Branch or commit to start from; empty means the commit the asking step left."
        },
        "done_when": {
          "type": "string",
          "maxLength": 300
        }
      },
      "required": [
        "question",
        "why_blocking",
        "kind"
      ]
    },
    "data": {
      "type": "object",
      "description": "Step-specific payload; shapes in section 2.10.8. Required; it may be empty. Every field a script reads is required by that step's data schema."
    }
  },
  "required": [
    "step",
    "status",
    "summary",
    "head",
    "data"
  ]
};
const S_VERDICT = {
  "type": "object",
  "properties": {
    "perimeter": {
      "type": "string",
      "enum": [
        "spec",
        "plan",
        "story",
        "wave",
        "release"
      ]
    },
    "subject": {
      "type": "string"
    },
    "head": {
      "type": "string",
      "description": "Commit the last pass reviewed."
    },
    "passes": {
      "type": "integer",
      "minimum": 0
    },
    "outcome": {
      "type": "string",
      "enum": [
        "converged",
        "blocked",
        "failed"
      ]
    },
    "reason": {
      "type": "string",
      "enum": [
        "clean",
        "cap",
        "spec_gap",
        "no_result",
        "reviewer_blocked"
      ]
    },
    "rounds_used": {
      "type": "integer",
      "minimum": 0,
      "description": "Passes or rounds used across every run for this unit."
    },
    "blocking": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "key": {
            "type": "string",
            "maxLength": 200,
            "description": "Stable identity, <category>:<path>:<symbol or clause>. The same defect has the same key in every pass."
          },
          "title": {
            "type": "string",
            "maxLength": 160
          },
          "severity": {
            "type": "string",
            "enum": [
              "critical",
              "high",
              "medium",
              "low"
            ]
          },
          "category": {
            "type": "string",
            "enum": [
              "spec-fidelity",
              "test",
              "code",
              "security",
              "integration",
              "performance",
              "accessibility",
              "ux",
              "docs",
              "spec",
              "traceability",
              "holdout",
              "formal",
              "coverage"
            ]
          },
          "scope": {
            "type": "string",
            "enum": [
              "unit",
              "integration",
              "system",
              "spec"
            ]
          },
          "owner": {
            "type": "string",
            "enum": [
              "product-owner",
              "architect",
              "ux-designer",
              "story-writer",
              "test-writer",
              "implementer",
              "verifier",
              "technical-writer",
              "devops",
              "codebase-analyzer",
              "demo-recorder"
            ],
            "description": "The roster agent that should fix it. Fixes are routed on this field."
          },
          "location": {
            "type": "object",
            "properties": {
              "path": {
                "type": "string",
                "description": "Relative to the root of the reviewed tree: the unit's worktree for a story, the integration worktree for a wave or a release, the project root for files under .factory/."
              },
              "line_start": {
                "type": "integer"
              },
              "line_end": {
                "type": "integer"
              },
              "quote": {
                "type": "string",
                "maxLength": 400
              }
            },
            "required": [
              "path"
            ]
          },
          "spec_ref": {
            "type": "string",
            "description": "Contract clause or acceptance criterion violated; empty if none."
          },
          "repro": {
            "type": "string",
            "maxLength": 800,
            "description": "Command or scenario that shows the defect; empty if none."
          },
          "observed": {
            "type": "string",
            "maxLength": 600
          },
          "expected": {
            "type": "string",
            "maxLength": 600
          },
          "suggestion": {
            "type": "string",
            "maxLength": 600
          },
          "confidence": {
            "type": "number",
            "minimum": 0,
            "maximum": 1
          }
        },
        "required": [
          "key",
          "title",
          "severity",
          "category",
          "scope",
          "owner",
          "location",
          "spec_ref",
          "repro",
          "observed",
          "expected"
        ]
      }
    },
    "advisory": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "key": {
            "type": "string",
            "maxLength": 200,
            "description": "Stable identity, <category>:<path>:<symbol or clause>. The same defect has the same key in every pass."
          },
          "title": {
            "type": "string",
            "maxLength": 160
          },
          "severity": {
            "type": "string",
            "enum": [
              "critical",
              "high",
              "medium",
              "low"
            ]
          },
          "category": {
            "type": "string",
            "enum": [
              "spec-fidelity",
              "test",
              "code",
              "security",
              "integration",
              "performance",
              "accessibility",
              "ux",
              "docs",
              "spec",
              "traceability",
              "holdout",
              "formal",
              "coverage"
            ]
          },
          "scope": {
            "type": "string",
            "enum": [
              "unit",
              "integration",
              "system",
              "spec"
            ]
          },
          "owner": {
            "type": "string",
            "enum": [
              "product-owner",
              "architect",
              "ux-designer",
              "story-writer",
              "test-writer",
              "implementer",
              "verifier",
              "technical-writer",
              "devops",
              "codebase-analyzer",
              "demo-recorder"
            ],
            "description": "The roster agent that should fix it. Fixes are routed on this field."
          },
          "location": {
            "type": "object",
            "properties": {
              "path": {
                "type": "string",
                "description": "Relative to the root of the reviewed tree: the unit's worktree for a story, the integration worktree for a wave or a release, the project root for files under .factory/."
              },
              "line_start": {
                "type": "integer"
              },
              "line_end": {
                "type": "integer"
              },
              "quote": {
                "type": "string",
                "maxLength": 400
              }
            },
            "required": [
              "path"
            ]
          },
          "spec_ref": {
            "type": "string",
            "description": "Contract clause or acceptance criterion violated; empty if none."
          },
          "repro": {
            "type": "string",
            "maxLength": 800,
            "description": "Command or scenario that shows the defect; empty if none."
          },
          "observed": {
            "type": "string",
            "maxLength": 600
          },
          "expected": {
            "type": "string",
            "maxLength": 600
          },
          "suggestion": {
            "type": "string",
            "maxLength": 600
          },
          "confidence": {
            "type": "number",
            "minimum": 0,
            "maximum": 1
          }
        },
        "required": [
          "key",
          "title",
          "severity",
          "category",
          "scope",
          "owner",
          "location",
          "spec_ref",
          "repro",
          "observed",
          "expected"
        ]
      }
    },
    "deferred": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "key": {
            "type": "string",
            "maxLength": 200,
            "description": "Stable identity, <category>:<path>:<symbol or clause>. The same defect has the same key in every pass."
          },
          "title": {
            "type": "string",
            "maxLength": 160
          },
          "severity": {
            "type": "string",
            "enum": [
              "critical",
              "high",
              "medium",
              "low"
            ]
          },
          "category": {
            "type": "string",
            "enum": [
              "spec-fidelity",
              "test",
              "code",
              "security",
              "integration",
              "performance",
              "accessibility",
              "ux",
              "docs",
              "spec",
              "traceability",
              "holdout",
              "formal",
              "coverage"
            ]
          },
          "scope": {
            "type": "string",
            "enum": [
              "unit",
              "integration",
              "system",
              "spec"
            ]
          },
          "owner": {
            "type": "string",
            "enum": [
              "product-owner",
              "architect",
              "ux-designer",
              "story-writer",
              "test-writer",
              "implementer",
              "verifier",
              "technical-writer",
              "devops",
              "codebase-analyzer",
              "demo-recorder"
            ],
            "description": "The roster agent that should fix it. Fixes are routed on this field."
          },
          "location": {
            "type": "object",
            "properties": {
              "path": {
                "type": "string",
                "description": "Relative to the root of the reviewed tree: the unit's worktree for a story, the integration worktree for a wave or a release, the project root for files under .factory/."
              },
              "line_start": {
                "type": "integer"
              },
              "line_end": {
                "type": "integer"
              },
              "quote": {
                "type": "string",
                "maxLength": 400
              }
            },
            "required": [
              "path"
            ]
          },
          "spec_ref": {
            "type": "string",
            "description": "Contract clause or acceptance criterion violated; empty if none."
          },
          "repro": {
            "type": "string",
            "maxLength": 800,
            "description": "Command or scenario that shows the defect; empty if none."
          },
          "observed": {
            "type": "string",
            "maxLength": 600
          },
          "expected": {
            "type": "string",
            "maxLength": 600
          },
          "suggestion": {
            "type": "string",
            "maxLength": 600
          },
          "confidence": {
            "type": "number",
            "minimum": 0,
            "maximum": 1
          }
        },
        "required": [
          "key",
          "title",
          "severity",
          "category",
          "scope",
          "owner",
          "location",
          "spec_ref",
          "repro",
          "observed",
          "expected"
        ]
      }
    },
    "dismissed": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "key": {
            "type": "string"
          },
          "by": {
            "type": "string",
            "enum": [
              "reviewer",
              "arbiter",
              "human"
            ]
          },
          "reason": {
            "type": "string"
          }
        },
        "required": [
          "key",
          "by",
          "reason"
        ]
      }
    },
    "coverage_missing": {
      "type": "array",
      "items": {
        "type": "string"
      }
    }
  },
  "required": [
    "perimeter",
    "subject",
    "head",
    "passes",
    "outcome",
    "reason",
    "blocking",
    "advisory",
    "deferred",
    "dismissed",
    "coverage_missing"
  ]
};
const S_WORKFLOW_RESULT = {
  "type": "object",
  "properties": {
    "workflow": {
      "type": "string"
    },
    "subject": {
      "type": "string"
    },
    "status": {
      "type": "string",
      "enum": [
        "done",
        "partial",
        "needs_human",
        "blocked",
        "failed"
      ]
    },
    "summary": {
      "type": "string",
      "maxLength": 600
    },
    "progress": {
      "type": "boolean",
      "description": "False when the run changed nothing on disk."
    },
    "decisions": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "id": {
            "type": "string"
          },
          "kind": {
            "type": "string"
          },
          "summary": {
            "type": "string"
          }
        },
        "required": [
          "id",
          "kind",
          "summary"
        ]
      }
    },
    "next": {
      "type": "object",
      "properties": {
        "kind": {
          "type": "string",
          "enum": [
            "workflow",
            "human",
            "done"
          ]
        },
        "command": {
          "type": "string"
        },
        "args": {
          "type": "object"
        }
      },
      "required": [
        "kind"
      ]
    },
    "counts": {
      "type": "object",
      "properties": {
        "agents": {
          "type": "integer"
        },
        "spikes": {
          "type": "integer"
        }
      }
    }
  },
  "required": [
    "workflow",
    "subject",
    "status",
    "summary"
  ]
};
// <<< generated:schemas

const RUN = {
  id: '', dir: '', root: '', wf: '', tier: null, agents: 0, spikes: 0,
  limits: {
    agents: 900, story_agents: 40, stories_per_run: 24, attempts: 2, passes: 3, rounds: 3,
    delivery_rounds: 3, gate_minutes: 60, ci_minutes: 30, spikes_per_step: 2, spikes_per_run: 6, spike_minutes: 20,
  },
}
const CLOSE_RESERVE = 30, TOKEN_FLOOR = 20000 // the close reserve; the smallest token budget a step may start with

// The survey returns limits under these names (blueprint 4.12); an unknown key fails the run.
const LIMIT_KEYS = [
  'agents', 'story_agents', 'stories_per_run', 'attempts', 'passes', 'rounds', 'delivery_rounds', 'gate_minutes',
  'ci_minutes', 'spikes_per_step', 'spikes_per_run', 'spike_minutes',
]
function applyLimits(from) {
  for (const k of Object.keys(from || {})) {
    if (!LIMIT_KEYS.includes(k)) throw new Error(`Unknown limit key "${k}" from the survey.`)
    RUN.limits[k] = from[k]
  }
}

const safe = (s) => (/^[A-Za-z0-9][A-Za-z0-9._\/-]*$/.test(String(s || '')) ? String(s) : '') // identifiers only; never an option (-C...)
const enc = (s) => encodeURIComponent(String(s || '')).replace(/[!'()*]/g, (c) => '%' + c.charCodeAt(0).toString(16).toUpperCase()) // free text on a command line; even the RFC 3986 extras; the tool decodes it

// A person's typed text, or an object, becomes the argument object; a key=value word is a named
// argument only when key is in names.
function parseArgs(raw, main, names = []) {
  if (raw && typeof raw === 'object') return raw
  const out = {}, rest = []
  for (const w of String(raw || '').trim().split(/\s+/).filter(Boolean)) {
    const m = /^([a-z_-]+)=(.+)$/.exec(w)
    if (m && names.includes(m[1])) out[m[1]] = m[2]
    else rest.push(w)
  }
  if (rest.length) out[main] = rest.join(' ')
  return out
}

const stepSchema = (data) => ({
  type: 'object',
  properties: {
    ...S_STEP_RESULT.properties,
    spike_request: S_SPIKE_REQUEST,
    data: data || { type: 'object', properties: {} },
  },
  required: S_STEP_RESULT.required,
})
const STR_ARR = { type: 'array', items: { type: 'string' } }
const OBJ = { type: 'object', properties: {} }
const KV = (props, req) => ({ type: 'object', properties: props, required: req })
// Composites of pipeline.md 2.10.8; the generated block holds the rest, from <plugin>/schemas/.
const S_REVIEW = {
  type: 'object',
  properties: {
    reviewed_sha: { type: 'string' }, findings: { type: 'array', items: S_FINDING },
    claim_checks: { type: 'array', items: KV({ key: { type: 'string' }, outcome: { type: 'string', enum: ['resolved', 'not_resolved', 'withdrawn'] }, note: { type: 'string' } }, ['key', 'outcome']) },
    coverage: { type: 'array', items: KV({ item: { type: 'string' }, state: { type: 'string', enum: ['checked', 'not_checkable', 'skipped'] }, note: { type: 'string' } }, ['item', 'state']) },
  },
  required: ['reviewed_sha', 'findings', 'claim_checks', 'coverage'],
}
const S_RESOLUTION = KV({ key: { type: 'string' }, resolution: { type: 'string', enum: ['fixed', 'disputed'] }, commit: { type: 'string' }, note: { type: 'string' } }, ['key', 'resolution'])
const S_FIXES = { type: 'object', properties: { resolutions: { type: 'array', items: S_RESOLUTION } }, required: ['resolutions'] }
const S_RULINGS = { type: 'object', properties: { rulings: { type: 'array', items: KV({ key: { type: 'string' }, ruling: { type: 'string', enum: ['defect', 'not_defect', 'spec_gap'] }, reason: { type: 'string' } }, ['key', 'ruling']) } }, required: ['rulings'] }
const SCORE = KV({ scenario: { type: 'string' }, satisfaction: { type: 'number' }, must_pass: { type: 'boolean' }, ran: { type: 'boolean' } }, ['scenario', 'satisfaction', 'must_pass', 'ran'])
const SYMPTOM = KV({ scenario: { type: 'string' }, observed: { type: 'string' }, expected: { type: 'string' } }, ['scenario', 'observed', 'expected'])
const S_HOLDOUT = { type: 'object', properties: { scores: { type: 'array', items: SCORE }, symptoms: { type: 'array', items: SYMPTOM } }, required: ['scores', 'symptoms'] }
const S_PRECONDITION = KV({ name: { type: 'string' }, met: { type: 'boolean' }, detail: { type: 'string' } }, ['name', 'met', 'detail'])
const S_NEXT = { type: 'object', properties: { kind: { type: 'string' }, command: { type: 'string' }, args: OBJ } }
const S_SURVEY = {
  type: 'object',
  properties: {
    ok: { type: 'boolean' }, error: { type: 'string' }, run_id: { type: 'string' }, workflow: { type: 'string' },
    subject: { type: 'string' }, cycle: { type: 'string' }, scope: { type: 'string', enum: ['full', 'delta'] },
    project_dir: { type: 'string' }, plugin_root: { type: 'string' }, head: { type: 'string' },
    preconditions: { type: 'array', items: S_PRECONDITION }, done: STR_ARR, units: { type: 'array', items: OBJ },
    config: OBJ, limits: OBJ, spikes_used: { type: 'integer' }, decisions: STR_ARR, waived: STR_ARR, next: S_NEXT,
  },
  required: ['ok', 'run_id', 'project_dir', 'plugin_root', 'preconditions', 'done', 'units', 'config', 'limits'],
}
// The close composes the workflow result's progress, decisions and next (pipeline.md 2.5).
const S_CLOSE = {
  type: 'object',
  properties: { ok: { type: 'boolean' }, status: { type: 'string' }, progress: S_WORKFLOW_RESULT.properties.progress, decisions: S_WORKFLOW_RESULT.properties.decisions, next: S_WORKFLOW_RESULT.properties.next },
  required: ['ok', 'status', 'next'],
}
const S_DELIVERY = {
  type: 'object',
  properties: {
    ok: { type: 'boolean' }, error: { type: 'string' }, state: { type: 'string' }, pr: { type: 'string' },
    merged_sha: { type: 'string' }, detail: { type: 'string' }, reviewed_sha: { type: 'string' }, changed_files: STR_ARR,
    diff_patch: { type: 'string' }, needs_person: { type: 'string' },
  },
  required: ['ok'],
}
// vsdd gate wave (pipeline.md 2.10.8): the gate result fields plus the wave's own.
const S_WAVE_GATE = {
  type: 'object',
  properties: {
    ...S_GATE.properties,
    pending: { type: 'boolean' }, worktree: { type: 'string' }, base: { type: 'string' },
    stories: { type: 'array', items: OBJ }, needs: KV({ security: { type: 'boolean' }, ui: { type: 'boolean' } }, []),
    holdout_batches: { type: 'array', items: OBJ }, diff_patch: { type: 'string' }, fix_units: { type: 'array', items: OBJ },
  },
  required: [...S_GATE.required, 'stories'],
}

const made = (id, status, kind, reason) => ({ step: { ...id, attempt: 0 }, status, summary: reason, head: '', blocked: { kind, reason }, data: {} })

// What every step is told. o.closing is a vsdd command without "vsdd"; with o.each the command
// holds <id> and is run once per item, and gated() requires every entry to pass.
function brief(id, n, task, spikes, o) {
  return [
    `Method: ${o.method}. Read ${RUN.root}/methods/${o.method}.md first.`,
    task,
    `Project directory: ${RUN.dir}.`,
    'You are running as a workflow step. If you cannot finish without an experiment, stop, return status ' +
      '"needs_spike" and fill spike_request with one question. A spike will be run for you in isolation and ' +
      'this step will be started again with the finding.',
    spikes.length ? `Spike findings already obtained for this step: ${JSON.stringify(spikes)}` : '',
    n > 1 || o.dirty ? 'Earlier work on this step may be on disk, some of it uncommitted. Check before redoing it.' : '',
    `Return step = ${JSON.stringify({ ...id, attempt: n })} unchanged.`,
    o.closing
      ? `Finish by running${o.each ? `, once for each ${o.each}` : ''}: vsdd -C ${RUN.dir} ${o.closing} --wait 540 --json  with a Bash timeout of 600000. ` +
        'If it prints state "pending", run the same command again. Put its final output in gates.'
      : '',
  ].filter(Boolean).join('\n')
}

// One research spike. Nothing about the asker is on the command line: vsdd spike open reads it
// from the needs_spike step record, stepKey.
async function runSpike(req, stepKey, phaseTitle) {
  RUN.spikes++
  RUN.agents++
  const tier = RUN.tier || TIER[ROLE['spike-runner']] // the spike runner's own tier, for every kind of spike
  const cmd = `vsdd -C ${RUN.dir} spike`
  const f = await agent(
    `Method: spike. Read ${RUN.root}/methods/spike.md first.\n` +
      `Question: ${req.question}\nWhy it blocks: ${req.why_blocking}\n` +
      `Suggested method: ${req.method_hint || 'your choice'}\nDone when: ${req.done_when || 'the question has an answer with evidence'}\n` +
      `First run: ${cmd} open --run ${RUN.id} --step ${stepKey} --ref ${safe(req.ref) || 'integration'} --json\n` +
      'If it returns an existing finding, return that finding with cached = true and do nothing else. If it refuses, return ' +
      'a finding with verdict "refused" and its reason. Otherwise run the experiment inside your own worktree only. Write the ' +
      `finding as JSON to a file in the temporary directory, then run: ${cmd} record --id <ledger_id> --json-file <that file>  and return the same finding.`,
    {
      agentType: 'vsdd-factory:spike-runner', model: tier.model, effort: tier.effort, schema: S_SPIKE_FINDING,
      phase: phaseTitle, label: `spike ${RUN.spikes}: ${String(req.question || '').slice(0, 48)}`,
      isolation: 'worktree', // always, research included: a step does not inherit the agent file's isolation
    },
  )
  return f || { ledger_id: '', question: req.question, verdict: 'inconclusive', answer: 'The spike agent returned nothing.', method: '', evidence: [], confidence: 0, cached: false }
}

// Run one step. o.method is required; retries an unusable result; serves spike requests within the limits.
async function step(type, name, subject, task, o = {}) {
  if (!o.method) throw new Error(`Step ${name} names no method.`)
  const tier = RUN.tier || TIER[OVERRIDE[`${RUN.wf}/${o.stage || name}`] || ROLE[type]]
  if (!tier) throw new Error(`Agent "${type}" is not in the roster, and no tier override covers ${RUN.wf}/${o.stage || name}.`)
  const id = { run: RUN.id, workflow: RUN.wf, name, subject }
  const spikes = o.spikes || []
  let bad = 0
  for (let n = 1; ; n++) {
    if (RUN.agents >= RUN.limits.agents)
      return made(id, 'blocked', 'agent_budget', 'The agent budget for this run is spent.')
    if (typeof budget !== 'undefined' && budget.total && budget.remaining() < TOKEN_FLOOR)
      return made(id, 'blocked', 'agent_budget', 'The token budget for this run is nearly spent.')
    RUN.agents++
    const r = await agent(brief(id, n, task, spikes, o), {
      agentType: 'vsdd-factory:' + type, model: tier.model, effort: tier.effort, schema: stepSchema(o.data),
      phase: o.phase, label: `${subject} · ${name} #${n}`,
    })
    if (!r || !r.step || r.step.name !== name || r.step.subject !== subject) {
      if (++bad >= RUN.limits.attempts) return made(id, 'failed', 'other', 'The step returned no usable result.')
      continue
    }
    if (r.status !== 'needs_spike') return r
    const q = r.spike_request
    if (!q || !q.question) return made(id, 'failed', 'other', 'The step asked for a spike without a question.')
    const over = spikes.length >= RUN.limits.spikes_per_step || RUN.spikes >= RUN.limits.spikes_per_run
    if (over) return { ...r, status: 'blocked', blocked: { kind: 'spike_limit', reason: 'The spike limit for this step or run is reached.', question: q.question } }
    // The tool also counts per run id: parent and children share the limit.
    const f = await runSpike({ ...q, ref: q.ref || r.head || '' }, `${RUN.wf}/${subject}/${name}.${n}`, o.phase)
    if (f.verdict === 'refused') return { ...r, status: 'blocked', blocked: { kind: 'spike_limit', reason: f.answer, question: q.question } }
    spikes.push(f)
  }
}

// Run a step until its gate passes on the commit it left behind; spike findings carry across
// attempts. With o.each every entry must pass and have its unit in data.units.
async function gated(type, name, subject, task, gate, o = {}) {
  let note = ''
  const spikes = []
  for (let i = 1; i <= RUN.limits.attempts; i++) {
    // Keep o.stage for every attempt, so the OVERRIDE lookup resolves the retry's tier.
    const r = await step(type, i === 1 ? name : `${name}-retry${i - 1}`, subject, task + note, { ...o, stage: o.stage || name, spikes })
    if (r.status !== 'done') return r
    const all = (r.gates || []).filter((x) => x.gate === gate)
    const gs = o.each ? all : all.slice(0, 1)
    const units = (r.data && r.data.units) || []
    const ok = (g) => g.passed && g.sha === r.head
    const bad = gs.filter((g) => !ok(g) || (o.each && !units.some((u) => u.id === g.subject)))
    if (gs.length && !bad.length) return r
    const why = bad.map((g) => (o.each ? `${g.subject}: ` : '') + (ok(g) ? 'passed, but its unit is not in data.units' : (g.failures || []).join('; ')))
    note = `\nThe ${gate} gate did not pass last time: ${gs.length ? why.join(' | ').slice(0, 1500) : 'no gate result was returned'}. Deal with that first.` +
      (o.each ? ` Then run the closing command again for every ${o.each}, and return every unit.` : '')
  }
  return made({ run: RUN.id, workflow: RUN.wf, name, subject }, 'blocked', 'gate', `The ${gate} gate still fails after ${RUN.limits.attempts} attempts.`)
}

// Run one vsdd command through the clerk and return its JSON output. One retry on nothing.
async function relay(name, subject, command, schema, phaseTitle) {
  const full = RUN.dir ? command.replace(/^vsdd /, `vsdd -C ${RUN.dir} `) : command
  for (let n = 1; n <= 2; n++) {
    // No agent-limit check: the checkpoint releases the lock and must always
    // be attempted; runStories reserves the close (the child deliver relay too).
    RUN.agents++
    const r = await agent(
      'Run exactly this command, with a Bash timeout of 600000, and return the JSON it prints as your result. ' +
        'If the JSON has state "pending", run the same command again, and repeat until it does not. ' +
        'Do not interpret, repair or summarise it. If it prints no JSON, return {"ok": false, "error": "<what it printed>"}.\n' + full,
      {
        agentType: 'vsdd-factory:clerk', model: (RUN.tier || TIER.clerk).model, effort: (RUN.tier || TIER.clerk).effort,
        schema, phase: phaseTitle, label: `${subject} · ${name}${n > 1 ? ' (retry)' : ''}`,
      },
    )
    if (r) return r
  }
  return { ok: false, error: 'The clerk step returned nothing, twice.' }
}

// Start child stories (wave, verify, feature, maintain, multi). A batch starts only if it fits
// beside the close reserve; a child that throws or returns nothing is charged its full budget.
async function runStories(units, extra = {}) {
  const per = RUN.limits.story_agents
  const fit = Math.max(0, Math.floor((RUN.limits.agents - RUN.agents - CLOSE_RESERVE) / per))
  const batch = units.slice(0, Math.min(fit, RUN.limits.stories_per_run))
  const share = Math.max(1, Math.floor((RUN.limits.spikes_per_run - RUN.spikes) / Math.max(1, batch.length)))
  const lost = (u, why) => ({ subject: u.id, status: 'failed', summary: why, deferred: [], agents: per, spikes: 0 })
  const rs = await parallel(batch.map((u) => async () => {
    try {
      return await workflow('vsdd-factory:story', {
        ...extra, story: u.id, position: u, run_id: RUN.id, project_dir: RUN.dir, plugin_root: RUN.root, tier: RUN.tier,
        limits: { ...RUN.limits, agents: per, spikes_per_run: share },
      })
    } catch (e) {
      return lost(u, String(e).slice(0, 300))
    }
  }))
  const results = rs.map((r, i) => r || lost(batch[i], 'The story run returned nothing.'))
  results.forEach((r) => { RUN.agents += r.agents || per; RUN.spikes += r.spikes || 0 })
  return { results, skipped: units.slice(batch.length) }
}

const HARD = ['critical', 'high']
const REACH = {
  spec: ['unit', 'integration', 'system', 'spec'], plan: ['unit', 'integration', 'system', 'spec'],
  story: ['unit'], wave: ['unit', 'integration'], release: ['unit', 'integration', 'system'],
}
function judge(findings, perimeter, waived) {
  const v = { blocking: [], advisory: [], deferred: [], spec_gaps: [] }
  for (const f of findings || []) {
    // Holdout is exempt from the location and grounding conditions (blueprint 4.10).
    const placed = f.category === 'holdout' || Boolean(f.location && f.location.path)
    const hard = HARD.includes(f.severity) && (f.category === 'holdout' || Boolean(f.spec_ref || f.repro)) && placed
    if ((waived || []).includes(f.key)) v.advisory.push(f)
    else if (REACH[perimeter].includes(f.scope)) (hard ? v.blocking : v.advisory).push(f)
    else if (f.scope === 'spec') (hard ? v.spec_gaps : v.advisory).push(f)
    else v.deferred.push(f)
  }
  return v
}

// c.review(pass, claims, gaps) -> { ok, head, findings, claim_checks, coverage_missing },
//                                 or { ok: false, blocked: true } when a reviewer returned blocked
// c.fix(open, advisory, pass)  -> { ok, resolutions: [{ key, resolution: 'fixed' | 'disputed', commit, note }] }
// c.arbitrate(open)            -> { rulings: [{ key, ruling: 'defect' | 'not_defect' | 'spec_gap', reason }] } (optional)
// c.used -> passes or rounds this unit already used in earlier runs (from the survey)
async function converge(c) {
  const raised = new Map()
  const waived = (c.waived || []).slice()
  const dismissed = []
  const disputed = new Set()
  const used = c.used || 0
  let claims = [], open = [], advisory = [], deferred = [], gaps = [], head = '', pass = 0, extra = 0
  const verdict = (outcome, reason) => ({ perimeter: c.perimeter, subject: c.subject, head, passes: pass, rounds_used: used + pass, outcome, reason, blocking: open, advisory, deferred, dismissed, coverage_missing: gaps })
  // An item that no reviewer marked checked is itself a finding, so the fixer is called and the loop can end.
  const uncovered = (item) => ({
    key: `coverage:${c.subject}:${item}`, title: `Not accounted for: ${item}`, severity: 'high', category: 'coverage',
    scope: REACH[c.perimeter][0], owner: 'test-writer', location: { path: '' }, spec_ref: item, repro: '',
    observed: 'No reviewer marked this item checked.', expected: 'A test or another evidence route covers it.',
  })
  while (used + pass < c.cap + extra) { // the cap counts across runs
    pass++
    const r = await c.review(pass, claims, gaps)
    if (r && r.blocked) return verdict('blocked', 'reviewer_blocked') // a decision opens at once; not a failed run
    if (!r || !r.ok) return verdict('failed', 'no_result')
    head = r.head
    const j = judge(r.findings, c.perimeter, waived)
    j.blocking.forEach((f) => raised.set(f.key, f))
    const checked = new Map((r.claim_checks || []).map((k) => [k.key, k.outcome]))
    for (const k of claims) if (checked.get(k.key) === 'withdrawn') dismissed.push({ key: k.key, by: 'reviewer', reason: 'Withdrawn on re-review.' })
    const still = claims.filter((k) => !['resolved', 'withdrawn'].includes(checked.get(k.key))).map((k) => raised.get(k.key))
    gaps = (r.coverage_missing || []).filter((item) => !waived.includes(`coverage:${c.subject}:${item}`))
    const cov = gaps.map(uncovered)
    cov.forEach((f) => raised.set(f.key, f))
    open = j.blocking.concat(still.filter((f) => f && f.category !== 'coverage' && !j.blocking.some((b) => b.key === f.key)), cov)
    advisory = j.advisory
    deferred = deferred.concat(j.deferred)
    if (j.spec_gaps.length) { open = j.spec_gaps; return verdict('blocked', 'spec_gap') }
    if (!open.length) return verdict('converged', 'clean')
    if (used + pass >= c.cap + extra) {
      if (extra || !c.arbitrate) break
      const a = await c.arbitrate(open)
      const ruled = new Map(((a && a.rulings) || []).map((x) => [x.key, x]))
      const upheld = []
      for (const f of open) {
        const x = ruled.get(f.key)
        if (x && x.ruling === 'spec_gap') { open = [f]; return verdict('blocked', 'spec_gap') }
        // Only a finding the builder disputed can be dismissed, and never a critical or security finding.
        if (x && x.ruling === 'not_defect' && disputed.has(f.key) && f.severity !== 'critical' && f.category !== 'security') {
          dismissed.push({ key: f.key, by: 'arbiter', reason: x.reason })
          waived.push(f.key)
        } else upheld.push(f)
      }
      open = upheld
      if (!open.length) return verdict('converged', 'clean')
      extra = 1
    }
    const fx = await c.fix(open, advisory, pass)
    if (!fx || !fx.ok) return verdict('failed', 'no_result')
    const how = new Map((fx.resolutions || []).map((x) => [x.key, x]))
    // A claim is structured data: no title, quote or fixer text; a holdout claim is the key alone.
    claims = open.map((f) => {
      const h = how.get(f.key) || {}
      if (h.resolution === 'disputed') disputed.add(f.key)
      if (f.category === 'holdout') return { key: f.key, resolution: h.resolution || 'fixed', commit: h.commit || '' }
      const where = f.location ? { path: f.location.path, line_start: f.location.line_start, line_end: f.location.line_end } : {}
      return { key: f.key, ...where, resolution: h.resolution || 'fixed', commit: h.commit || '' }
    })
  }
  return verdict('blocked', 'cap')
}

// Turn holdout scores into findings, in the shape pipeline.md 2.7 fixes. Thresholds come from
// project configuration via the survey (opts: mean_min, must_pass_min, stddev_max); the evaluator's
// symptoms are the observed and expected texts. A must-pass scenario that never ran scores 0: it
// has not demonstrated its behaviour, which is a failure of the scenario, not a pass.
function holdoutFindings(unit, scores, opts = {}) {
  const MEAN = opts.mean_min ?? 0.85, SPREAD = opts.stddev_max ?? 0.15, FLOOR = opts.must_pass_min ?? 0.6
  const cite = opts.seed ? ` rotation seed ${opts.seed}` : ''
  const symptom = (scenario, fallback) =>
    (opts.symptoms || []).find((x) => x.scenario === scenario) || fallback
  const build = (key, title, how, repro, scenario) => ({
    key, title, severity: 'high', category: 'holdout', scope: opts.scope || 'integration',
    owner: 'product-owner', location: { path: '' }, spec_ref: ((opts.contracts || {})[scenario] || []).join(', '),
    repro, observed: how.observed, expected: how.expected,
  })
  const out = []
  for (const s of scores || []) {
    if (!s.must_pass) continue
    const score = s.ran ? s.satisfaction : 0
    if (score < FLOOR) {
      out.push(build(`holdout:${s.scenario}`,
        `Hidden scenario ${s.scenario} scored ${score.toFixed(2)}, under its floor of ${FLOOR.toFixed(2)}`,
        symptom(s.scenario, { observed: `Must-pass scenario scored ${score.toFixed(2)}.`, expected: `At least ${FLOOR.toFixed(2)}.` }),
        `${s.scenario}${cite}`, s.scenario))
    }
  }
  const ran = (scores || []).filter((s) => s.ran)
  if (ran.length) {
    const mean = ran.reduce((acc, s) => acc + s.satisfaction, 0) / ran.length
    const spread = Math.sqrt(ran.reduce((acc, s) => acc + (s.satisfaction - mean) ** 2, 0) / ran.length)
    if (mean < MEAN) {
      out.push(build(`holdout:${unit}:mean`,
        `Mean hidden-scenario satisfaction ${mean.toFixed(2)}, under the threshold of ${MEAN.toFixed(2)}`,
        symptom('mean', { observed: `Mean satisfaction ${mean.toFixed(2)}.`, expected: `At least ${MEAN.toFixed(2)}.` }),
        `${ran.length} scenarios${cite}`, 'mean'))
    }
    if (spread >= SPREAD) {
      out.push(build(`holdout:${unit}:spread`,
        `Hidden-scenario spread ${spread.toFixed(2)}, over the threshold of ${SPREAD.toFixed(2)}`,
        symptom('spread', { observed: `Standard deviation ${spread.toFixed(2)}.`, expected: `Below ${SPREAD.toFixed(2)}.` }),
        `${ran.length} scenarios${cite}`, 'spread'))
    }
  }
  return out
}
