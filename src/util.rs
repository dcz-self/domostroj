/*
 * SPDX-License-Identifier: LGPL-3.0-or-later
 */

use std::time::Instant;

#[macro_export]
macro_rules! time {
    ($e:expr) => {
        // match is Following dbg! recommendation
        // https://stackoverflow.com/a/48732525/1063961
        {
            let start = Instant::now();
            match $e {
                e => {
                    let end = Instant::now();
                    eprintln!("[{}:{}] {} elapsed ms: {}",
                        file!(), line!(), stringify!($e),
                        (end - start).as_millis(),
                    );
                    e
                }
            }
        }
    }
}
