#!/bin/sh
echo "== sandbox probe =="
echo "HOME_WRITE: $( (echo canary > "$HOME/fs4-canary.txt" && echo ok) 2>&1 )"
echo "-- git ops --"
git worktree add .worktrees/x -b story/x 2>&1 || echo "WORKTREE-FAIL rc=$?"
sleep 0; echo x > .worktrees/x/file.txt 2>&1
git -C .worktrees/x add file.txt 2>&1 || echo "ADD-FAIL rc=$?"
git -C .worktrees/x commit -m probe 2>&1 || echo "COMMIT-FAIL rc=$?"
git update-ref refs/heads/probe-ref HEAD 2>&1 || echo "UPDATEREF-FAIL rc=$?"
echo "REF: $(git rev-parse refs/heads/probe-ref 2>&1)"
echo "-- worktree list --"
git worktree list 2>&1
