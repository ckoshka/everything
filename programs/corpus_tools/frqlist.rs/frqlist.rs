//!
//! ```cargo
//! [dependencies]
//! term_macros = { path = "../../shared/term_macros"  }
//! compact_str = "0.5"
//! ```

use term_macros::*;
use compact_str::CompactString;

fn no_punctuation(w: &str) -> CompactString {
    w.chars().map(|c| c.to_lowercase()).flatten().collect()
}

fn main() {

    tool! {
        args:
            - min_count: usize = 0;
        ;

        body: || {

            let mut map = std::collections::HashMap::with_capacity(1000000);
            readin!(_tx, |bytes: &[u8]| {
                let as_str = std::str::from_utf8(bytes);
                let _ = as_str.map(|s| {
                    s.split(|c: char| !c.is_alphabetic())
                        .filter(|w| w.len() > 0)
                        .for_each(|w| {
                            let key = no_punctuation(w);
                            if !(key.len() > 0) {
                                return;
                            }
                            if let Some(entry) = map.get_mut(&key) {
                                *entry += 1;
                            } else {
                                map.insert(key, 1);
                            }
                        })
                });
            });
            let mut freqs: Vec<_> = map.into_iter().filter(|(_k, v)| v > &min_count).collect();
            freqs.sort_by_key(|(_k, v)| *v as i64);

            freqs.into_iter().rev().for_each(|(k, _v)| {
                println!("{}", k);
            });
        }

    };
}