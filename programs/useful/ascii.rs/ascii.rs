//!
//! ```cargo
//! [dependencies]
//! term_macros = { path = "../../shared/term_macros"  }
//! any_ascii = "0.1.6"
//! ```

use term_macros::*;
use any_ascii::any_ascii;

fn main() {
    readin!(wtr, |line: &[u8]| {
        let _ = std::str::from_utf8(line).map(|s| {
            let r = wtr.write_all(any_ascii(s).as_bytes());
            if r.is_err() {
                panic!("Unable to write")
            }
        });
    });
}