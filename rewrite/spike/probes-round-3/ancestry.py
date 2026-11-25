import os, subprocess
pids = {}
out = subprocess.run(["ps", "-axo", "pid,ppid,comm"], capture_output=True, text=True).stdout
for line in out.splitlines()[1:]:
    parts = line.strip().split(None, 2)
    if len(parts) == 3:
        try:
            pids[int(parts[0])] = (int(parts[1]), parts[2])
        except ValueError:
            pass
pid = os.getpid()
chain = []
while pid in pids and len(chain) < 40:
    ppid, comm = pids[pid]
    chain.append(f"{pid}:{os.path.basename(comm)}")
    pid = ppid
print(" -> ".join(chain))
has_claude = any("claude" in c.lower() for _, c in [pids.get(int(x.split(":")[0]), (0, "")) for x in chain])
print("CLAUDE_IN_CHAIN=%s" % any("claude" in c for _, c in pids.values() if True) if False else "CLAUDE_IN_CHAIN=%s" % ("yes" if any("claude" in seg.lower() for seg in chain) else "no"))
