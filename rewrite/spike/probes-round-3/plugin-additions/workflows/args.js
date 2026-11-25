export const meta = {
  name: 'args',
  description: 'Platform probe round 3 item 31: what typed-after-the-command text reaches the script',
  phases: [{ title: 'Args' }],
}

phase('Args')
return {
  typeof_args: typeof args,
  json_args: JSON.stringify(args).slice(0, 300),
  keys: args && typeof args === 'object' ? Object.keys(args) : [],
}
