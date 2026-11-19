# Test fixtures

Fast Downward `output.sas` files copied from [`sas-plus`](https://github.com/sunsided/sas-plus/tree/main/tests/fixtures),
where their format, origin and optimal costs are documented and checked.

- `gripper-unit/`: hand-encoded, written for `sas-plus` (MIT OR Apache-2.0).
- `translated/bw-large-a/`: translator output next to its PDDL input.
- `translated/logistics-e/`: translator output next to its PDDL input.
- `translated/gripper-2/`: translator output next to its PDDL input.
- `translated/gripper-4/`: translator output next to its PDDL input.
- `translated/sussman-anomaly/`: translator output next to its PDDL input.

## Licences

The PDDL files under `translated/` come from the test data of [sunsided/pddl-rs](https://github.com/sunsided/pddl-rs) (`tests/data/`).
Each `output.sas` is derived from its inputs and falls under the same licences.

- **VHPOP** files come from the `examples/` folder of [VHPOP](https://github.com/hlsyounes/vhpop) by Håkan L. S. Younes, at commit `cf27db2ed46f8bd64cb78fe256cb5cf3ad8174cf`, under the **GNU General Public License, version 2**; see [`COPYING.vhpop`](https://github.com/sunsided/width-search/blob/main/tests/fixtures/translated/COPYING.vhpop).
- **`logistics-e/problem.pddl`** comes from the `examples/` folder of [Potoroo](https://github.com/HarrisonOates/potoroo) by Harrison Oates, at commit `1264ed6cefaf3b72233366c44a3a9ac5303b43c2`, under the **Apache License 2.0**; see [`LICENSE.potoroo`](https://github.com/sunsided/width-search/blob/main/tests/fixtures/translated/LICENSE.potoroo) and [`NOTICE.potoroo`](https://github.com/sunsided/width-search/blob/main/tests/fixtures/translated/NOTICE.potoroo).

All of this is test input only. It is not compiled into the crate and is excluded from its published package.
