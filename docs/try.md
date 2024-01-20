# Try Guide

Try runs allows to build and test changes on GitHub CI without requiring code to be reviewed and landed.

## Triggering try runs

You can trigger try runs via:

- adding `T-` labels on PR (servo organization members only)
- dispatching workflows from GitHub UI on personal fork
- running `mach try $presets` command that will send git `HEAD` (patches that are committed in current checkout) to try branch on personal fork.

### Presets

- `linux` (does not run any wpt tests)
- `mac`
- `win`/`windows`
- `wpt`/`linux-wpt` (runs wpt tests for `both` layouts on linux)
- `webgpu` (runs WebGPU CTS on linux)
- `wpt-2013` or `linux-wpt-2013` (runs wpt tests on `2013` layout)
- `wpt-2020` or `linux-wpt-2020` (runs wpt tests on `2020` layout)
- `mac-wpt` (runs wpt tests for `both` layouts on mac)
- `mac-wpt-2013`
- `mac-wpt-2020`
