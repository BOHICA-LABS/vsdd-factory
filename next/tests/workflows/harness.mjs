// The workflow harness: runs a workflow script the way the platform does, with stubs
// (packaging.md 2.6, layer 2). Node built-ins only.
//
//   import { runWorkflow, runWorkflowFile } from './harness.mjs'
//   const run = await runWorkflow('toy-build', { args: {...}, agents: {...}, maxAgents: 80 })
//
// `run` carries `result`, `error`, `calls`, `phases` and `agentsUsed`. The agent counter is shared
// by a parent and the children it starts, so a wave whose children overspend is caught here.
//
// Platform behaviour copied here, one row per line of the harness table:
// - no file, shell, module or network access: the context has no require, process, fetch or import();
// - Date.now(), Math.random() and argument-less new Date() throw;
// - agent() with a schema returns a validated object; without one, a string; null on a dead agent;
// - a schema needs type: 'object' and properties at the root, and required within properties; a bad
//   schema throws;
// - parallel() turns a throwing thunk into null and never rejects; pipeline() drops an item to null;
// - workflow() runs a child once; a child that calls workflow() throws; nesting is one level;
// - at most 1,000 agents in a run, counted across parent and children; a test lowers the cap.

import fs from 'node:fs'
import path from 'node:path'
import vm from 'node:vm'
import { fileURLToPath } from 'node:url'

export const PLATFORM_AGENT_CAP = 1000
export const NESTING_MESSAGE = 'Nesting is limited to one level.'

export class DeadAgentError extends Error {}

const TOYS_DIRECTORY = path.join(path.dirname(fileURLToPath(import.meta.url)), 'toys')

// ----------------------------------------------------------------- schema validation

export function validateValue(schema, value, where) {
  if (schema === true || schema === undefined) return
  if (schema === false) throw new Error(`${where}: schema forbids any value`)
  if (schema.enum) {
    // enum is type-sensitive: true never matches 1.
    const hit = schema.enum.some((option) => typeof option === typeof value && option === value)
    if (!hit) throw new Error(`${where}: ${JSON.stringify(value)} is not one of ${JSON.stringify(schema.enum)}`)
    return
  }
  const type = schema.type
  if (type === 'object') {
    if (typeof value !== 'object' || value === null || Array.isArray(value)) {
      throw new Error(`${where}: expected an object`)
    }
    for (const key of schema.required || []) {
      if (!(key in value)) throw new Error(`${where}: missing required property "${key}"`)
    }
    for (const [key, sub] of Object.entries(schema.properties || {})) {
      if (key in value && value[key] !== undefined) validateValue(sub, value[key], `${where}.${key}`)
    }
    return
  }
  if (type === 'array') {
    if (!Array.isArray(value)) throw new Error(`${where}: expected an array`)
    if (schema.maxItems != null && value.length > schema.maxItems) throw new Error(`${where}: more than ${schema.maxItems} items`)
    for (const [i, item] of value.entries()) validateValue(schema.items || true, item, `${where}[${i}]`)
    return
  }
  if (type === 'string') {
    if (typeof value !== 'string') throw new Error(`${where}: expected a string`)
    if (schema.maxLength != null && value.length > schema.maxLength) throw new Error(`${where}: longer than ${schema.maxLength}`)
    return
  }
  if (type === 'integer') {
    if (!Number.isInteger(value)) throw new Error(`${where}: expected an integer`)
    checkRange(schema, value, where)
    return
  }
  if (type === 'number') {
    if (typeof value !== 'number' || Number.isNaN(value)) throw new Error(`${where}: expected a number`)
    checkRange(schema, value, where)
    return
  }
  if (type === 'boolean') {
    if (typeof value !== 'boolean') throw new Error(`${where}: expected a boolean`)
    return
  }
  if (type === 'null') {
    if (value !== null) throw new Error(`${where}: expected null`)
    return
  }
  throw new Error(`${where}: unsupported schema type ${JSON.stringify(type)}`)
}

function checkRange(schema, value, where) {
  if (schema.minimum != null && value < schema.minimum) throw new Error(`${where}: below minimum ${schema.minimum}`)
  if (schema.maximum != null && value > schema.maximum) throw new Error(`${where}: above maximum ${schema.maximum}`)
}

export function checkSchemaShape(schema, where) {
  if (!schema || typeof schema !== 'object') throw new Error(`${where}: the schema is not an object`)
  // The platform rule: type object and properties at the root; required is
  // optional, and every name in it must be a known property — a required
  // ghost property would make every result fail.
  if (schema.type !== 'object' || !schema.properties) {
    throw new Error(`${where}: a schema needs type: 'object' and properties at the root`)
  }
  const checkRequired = (node, at) => {
    if (typeof node !== 'object' || node === null) return
    for (const name of node.required || []) {
      if (!(name in (node.properties || {}))) {
        throw new Error(`${at}: required names the unknown property "${name}"`)
      }
    }
    for (const [key, sub] of Object.entries(node.properties || {})) {
      if (sub && typeof sub === 'object') checkRequired(sub, `${at}.${key}`)
    }
  }
  checkRequired(schema, where)
}

// The smallest object a schema allows (universal test 4) and "the other way" (universal test 5).
export function minimal(schema) {
  if (!schema || typeof schema !== 'object') return undefined
  if (schema.enum) return schema.enum[0]
  switch (schema.type) {
    case 'object': {
      const out = {}
      for (const [key, sub] of Object.entries(schema.properties || {})) out[key] = minimal(sub)
      return out
    }
    case 'array':
      return []
    case 'string':
      return ''
    case 'boolean':
      return false
    case 'integer':
    case 'number':
      return schema.minimum ?? 0
    case 'null':
      return null
    default:
      return undefined
  }
}

export function alternate(schema) {
  if (!schema || typeof schema !== 'object') return undefined
  if (schema.enum) return schema.enum[schema.enum.length - 1]
  switch (schema.type) {
    case 'object': {
      const out = {}
      for (const [key, sub] of Object.entries(schema.properties || {})) {
        out[key] = alternate(sub) ?? minimal(sub)
      }
      return out
    }
    case 'array': {
      const item = minimal(schema.items || true)
      return item === undefined ? [] : [item]
    }
    case 'string':
      return 'x'
    case 'boolean':
      return true
    case 'integer':
    case 'number':
      return (schema.minimum ?? 0) === 0 ? 1 : (schema.minimum ?? 0)
    case 'null':
      return null
    default:
      return undefined
  }
}

// ----------------------------------------------------------------- script loading

// Line 1 is `export const meta = {`; the object ends at the first line that is exactly `}`
// (packaging.md 2.5). readMeta evaluates only the meta literal.
export function readMeta(source) {
  const lines = source.split('\n')
  if (lines[0] !== 'export const meta = {') throw new Error('line 1 is not "export const meta = {"')
  const end = lines.findIndex((line, i) => i > 0 && line === '}')
  if (end < 0) throw new Error('the meta object never closes on a lone "}" line')
  return new Function(`${lines.slice(0, end + 1).join('\n').replace('export const meta', 'const meta')}; return meta`)()
}

export function stripStamped(source) {
  const start = source.indexOf('// >>> stamped:prelude')
  const end = source.indexOf('// <<< stamped:prelude')
  if (start < 0 || end < 0) return source
  return source.slice(0, start) + source.slice(end + '// <<< stamped:prelude'.length)
}

// ----------------------------------------------------------------- the realm

function buildRealm(shared, opts) {
  const sandbox = Object.create(null)
  const total = opts.budget?.total ?? null

  const spend = () => {
    // The token budget is a hard ceiling: an agent cannot start once spent
    // reaches total (pipeline.md 2.5), so the counter stops at total.
    if (total != null && shared.counter >= total) {
      throw new Error(`The token budget (${total}) is spent; the run is stopped.`)
    }
    shared.counter += 1
    if (shared.counter > shared.maxAgents) {
      throw new Error(`The agent cap (${shared.maxAgents}) is spent; the run is stopped.`)
    }
  }

  sandbox.agent = async (prompt, agentOpts = {}) => {
    spend()
    const label = agentOpts.label || ''
    const call = {
      label,
      agentType: agentOpts.agentType || '',
      model: agentOpts.model || '',
      effort: agentOpts.effort || '',
      isolation: agentOpts.isolation || '',
      phase: agentOpts.phase || '',
      prompt,
      schema: agentOpts.schema || null,
      result: null,
    }
    shared.calls.push(call)
    const table = shared.agents
    const stub = label in table ? table[label] : agentOpts.agentType in table ? table[agentOpts.agentType] : table['*']
    if (stub === undefined) throw new Error(`No agent stub for ${agentOpts.agentType || label || '(unlabelled)'}.`)
    let answer
    try {
      answer = typeof stub === 'function' ? await stub(prompt, agentOpts) : structuredClone(stub)
    } catch (e) {
      if (e instanceof DeadAgentError) answer = null
      else throw e
    }
    call.result = answer
    if (answer === null) return null // a dead agent
    if (!agentOpts.schema) {
      if (typeof answer !== 'string') throw new Error(`Agent ${label || agentOpts.agentType} returned a non-string with no schema.`)
      return answer
    }
    checkSchemaShape(agentOpts.schema, `schema of ${label || agentOpts.agentType}`)
    validateValue(agentOpts.schema, answer, `result of ${label || agentOpts.agentType}`)
    return structuredClone(answer)
  }

  sandbox.parallel = async (items) =>
    Promise.all(items.map((item) => Promise.resolve().then(typeof item === 'function' ? item : () => item).catch(() => null)))

  // The platform shape: pipeline(items, stage1, ...) runs each item through
  // its own chain; each stage is called (prev, originalItem, index); a stage
  // that throws drops that item to null and skips its remaining stages.
  sandbox.pipeline = async (items, ...stages) => {
    const results = new Array(items.length)
    await Promise.all(items.map(async (item, index) => {
      let prev = item
      for (let s = 0; s < stages.length; s++) {
        try {
          prev = await stages[s](prev, item, index)
        } catch {
          results[index] = null // a thrown stage nulls the item and skips the rest
          return
        }
      }
      results[index] = prev
    }))
    return results
  }

  sandbox.workflow = async (name, args = {}) => {
    if (opts.depth >= 1) throw new Error(NESTING_MESSAGE)
    // The workflow() call itself is not an agent; the child's agent calls
    // spend on the shared counter.
    const file = shared.workflows[name]
    if (!file) throw new Error(`No child workflow is registered under "${name}".`)
    const { result, error } = await runFile(file, { args, shared, depth: opts.depth + 1 })
    if (error) throw error // the platform's workflow() throws when the child threw
    return result
  }

  sandbox.phase = (title) => {
    shared.phases.push(title)
  }

  sandbox.log = (message) => {
    shared.logs.push(String(message))
  }

  // The platform token budget, driven by the test's `budget` option
  // ({ total }): remaining() is Infinity without a total; agent() throws once
  // spent reaches total, on the shared counter.
  sandbox.budget = {
    total,
    spent: () => shared.counter,
    remaining: () => (total == null ? Infinity : total - shared.counter),
  }

  // Deterministic script text: the two clocks the platform denies.
  const ThrowingDate = class extends Date {
    constructor(...rest) {
      if (rest.length === 0) throw new Error('Argument-less new Date() is not available to a workflow script.')
      super(...rest)
    }
    static now() {
      throw new Error('Date.now() is not available to a workflow script.')
    }
  }
  sandbox.Date = ThrowingDate
  sandbox.Math = Object.create(Math)
  sandbox.Math.random = () => {
    throw new Error('Math.random() is not available to a workflow script.')
  }

  return vm.createContext(sandbox, { name: 'workflow' })
}

// ----------------------------------------------------------------- the run

// Run one workflow by file name (without .js) from tests/workflows/toys, or from `options.directory`.
export async function runWorkflow(name, options = {}) {
  const dir = options.directory || TOYS_DIRECTORY
  const file = path.join(dir, `${name}.js`)
  if (!fs.existsSync(file)) throw new Error(`No workflow file ${file}`)
  return runWorkflowFile(file, options)
}

// Run one workflow file with a fresh shared state. `options.workflows` maps the qualified child names
// ("vsdd-factory:story") to child workflow files; `options.agents` maps label, then agentType, then
// '*', to a stub value, a stub function, or a function that throws DeadAgentError for a dead agent.
export async function runWorkflowFile(file, options = {}) {
  const shared = {
    agents: options.agents || {},
    workflows: options.workflows || {},
    maxAgents: options.maxAgents ?? PLATFORM_AGENT_CAP,
    budget: options.budget || null,
    counter: 0,
    calls: [],
    phases: [],
    logs: [],
  }
  const run = await runFile(file, { args: options.args ?? {}, shared, depth: options.depth ?? 0 })
  return {
    result: run.result,
    error: run.error,
    calls: shared.calls,
    phases: shared.phases,
    logs: shared.logs,
    agentsUsed: shared.counter,
    shared,
  }
}

async function runFile(file, opts) {
  const source = fs.readFileSync(file, 'utf8')
  // The stamped region is part of the script: it stays. Only the export is dropped, because the
  // wrapper is a plain async function, not a module.
  const body = source.replace(/export const meta/, 'const meta')
  const wrapped = `(async (args) => {\n${body}\n})`
  const realm = buildRealm(opts.shared, { depth: opts.depth, budget: opts.shared.budget })
  let result = null
  let error = null
  try {
    const fn = new vm.Script(wrapped, { filename: path.basename(file) }).runInContext(realm)
    result = await fn(opts.args)
  } catch (e) {
    error = e
  }
  return { result, error }
}
