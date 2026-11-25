# Script as run (rounded into one block; repoE is the clean .git/hooks leg, repoC the core.hooksPath leg,
# repoA/B demonstrate that an automatic `git merge --no-ff` runs no pre-commit).
mk() {
  R=$1; mkdir -p $R; cd $R
  git init -q -b main; git config user.email p@e.invalid; git config user.name p
  echo base > file.txt; git add file.txt; git commit -qm base
  mkdir -p scripts .githooks; echo "1.0" > scripts/version
  cat > .githooks/pre-commit <<EOF
#!/bin/sh
{
echo "hook: pwd=\$(pwd) GIT_DIR=\${GIT_DIR:-unset} GIT_WORK_TREE=\${GIT_WORK_TREE:-unset} toplevel=\$(git rev-parse --show-toplevel 2>&1)"
echo "hook: tracked=[\$(cat scripts/version 2>&1)] untracked=[\$(cat data.gen 2>&1)]"
if [ ! -f data.gen ]; then echo "hook: FAILING - data.gen (a file from the main checkout) is missing"; echo END; exit 1; fi
echo END
} >> $1.log.txt
exit 0
EOF
  chmod +x .githooks/pre-commit
  git add file.txt scripts/version .githooks/pre-commit; git commit -qm setup
  git checkout -qb feature; echo feat > feat.txt; git add feat.txt; git commit -qm feat; git checkout -q main
  git worktree add .worktrees/_integration -b integration
}
mk /tmp/pr4-q10/repoE
cp /tmp/pr4-q10/repoE/.githooks/pre-commit /tmp/pr4-q10/repoE/.git/hooks/pre-commit
cd /tmp/pr4-q10/repoE/.worktrees/_integration
git merge --no-ff --no-commit feature && git commit -qm "merge feature"   # hook runs: cwd=worktree, GIT_DIR=.git/worktrees/_integration
# repoC: same flow with `git config core.hooksPath .githooks`; hook exit 1 blocks the commit until data.gen exists
