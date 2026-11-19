#!/usr/bin/env bash
# Runs LAPKT's width-based planners on the translated test fixtures and prints
# their node counts, for the comparison table in README.md.
#
# Needs `lapkt_cmd.py` on PATH, for example from a virtualenv:
#   python3 -m venv .venv && .venv/bin/pip install lapkt==0.2.1 clingo
#   PATH=.venv/bin:$PATH scripts/lapkt-compare.sh
# clingo provides the gringo grounder that LAPKT's default (Tarski) front end needs.
set -euo pipefail

fixtures="$(cd "$(dirname "$0")/../tests/fixtures/translated" && pwd)"
lapkt="$(command -v lapkt_cmd.py)"
python="$(dirname "$lapkt")/python"
[ -x "$python" ] || python=python3
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cd "$work"

planners=(
    "IW_Planner --iw_bound 1"
    "IW_Planner --iw_bound 2"
    "SIW_Planner --iw_bound 2"
    "BFWS --search_type BFWS-goalcount-only"
    "BFWS --search_type BFWS-f5"
)

printf '%-16s %-40s %9s %9s %9s %5s\n' task planner expanded generated pruned cost
for task in gripper-2 sussman-anomaly gripper-4 bw-large-a logistics-e; do
    for planner in "${planners[@]}"; do
        # shellcheck disable=SC2086 # the planner string holds its own arguments
        out="$(timeout 600 "$python" "$lapkt" $planner \
            -d "$fixtures/$task/domain.pddl" -p "$fixtures/$task/problem.pddl" 2>&1 || true)"
        field() { grep -m1 -oP "$1\K[0-9.e+]+" <<<"$out" || echo -; }
        printf '%-16s %-40s %9s %9s %9s %5s\n' "$task" "$planner" \
            "$(field 'Nodes expanded during search: ')" \
            "$(field 'Nodes generated during search: ')" \
            "$(field 'Nodes pruned by bound: ')" \
            "$(field 'Plan found with cost: ')"
    done
done
