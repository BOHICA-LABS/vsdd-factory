export const meta = {
  name: 'mcpstep',
  description: 'Platform probe round 3: an MCP call from inside a workflow step',
  phases: [{ title: 'McpStep' }],
}
phase('McpStep')
const r = await agent(
  'Call the tool mcp__tiny__ping once with the argument {"echo":"STEP"} and report exactly what comes back.',
  { agentType: 'fspike:mcpping', label: 'mcp-step', phase: 'McpStep' },
)
return { step_agent_result: String(r).slice(0, 400) }
