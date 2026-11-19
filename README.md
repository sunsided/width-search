# width-search

Width-based search (Lipovetzky & Geffner): novelty tables, IW(k), iterated width, and BFWS(⟨w_{#g}, #g⟩).

Not tied to PDDL. Anything that can enumerate successors and describe states as integer features implements `WidthProblem`: planning atoms, grid cells, RAM bytes. The `sas-plus` feature adds an adapter for `sas_plus::Task`.

```bash
cargo run --example key_door         # IW(1) fails, IW(2) succeeds
cargo test --all-features
```

**Tested:** end to end on a Fast Downward `output.sas` (gripper: IW needs width 3 for the conjunctive goal, finding the optimal 5-step plan). Also: IW with k = #variables is complete and optimal on unit-cost random tasks; IW and BFWS plans are validated.

**Roadmap:** SIW, BFWS with `#r`, Rollout IW, node-count comparison against LAPKT.

References: Lipovetzky & Geffner, ECAI 2012 and AAAI 2017.
