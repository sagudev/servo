/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::sync::atomic::Ordering;

use ui_test::{run_tests, Config};

fn main() -> ui_test::color_eyre::Result<()> {
    let config = Config::rustc("tests/tests");
    let abort_check = config.abort_check.clone();
    ctrlc::set_handler(move || abort_check.store(true, Ordering::Relaxed))?;

    // Compile all `.rs` files in the given directory (relative to your
    // Cargo.toml) and compare their output against the corresponding
    // `.stderr` files.
    run_tests(config)
}
