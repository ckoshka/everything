//!
//! ```cargo
//! [dependencies]
//! term_macros = { path = "../../shared/term_macros"  }
//! fnv = "1.0.7"
//! nohash-hasher = "0.2.0"
//! ```

use fnv::FnvHasher;
use nohash_hasher::IntSet;
use std::hash::Hash;
use std::hash::Hasher;
use term_macros::*;

fn hash_str(s: &str) -> u64 {
    let mut h = FnvHasher::with_key(0);
    s.hash(&mut h);
    h.finish()
}

fn clean(w: &str) -> IntSet<u64> {
    w.chars()
        .filter(|c| c.is_whitespace() || c.is_alphabetic())
        .map(|c| c.to_lowercase())
        .flatten()
        .collect::<String>()
        .split(char::is_whitespace)
        .filter(|s| s.len() > 0)
        .map(|s| hash_str(s))
        .collect()
}

// only known words
fn only_known<'a>(
    known_words: &IntSet<u64>,
    sentence: &IntSet<u64>,
    maximum_unknown: usize,
) -> bool {
    sentence.difference(&known_words).count() <= maximum_unknown
}

fn main() {
    tool! {
        args:
            - allowed_words: String;
                ? !std::path::Path::new(&allowed_words).exists()
                => "the file you entered doesn't exist"
            - truncate: Option<usize> = None;
            - max_unknown: usize = 0;
            - column: usize = 0;
            - sep: String = "\t".to_string();
        ;

        body: || {

            let mut words = String::with_capacity(5000);
            std::fs::File::open(&allowed_words).expect("").read_to_string(&mut words)
                .expect("Was unable to read the file for some reason");
            let mut allowed: IntSet<u64> = IntSet::default();
            words.split("\n").take(truncate.unwrap_or_else(|| usize::MAX)).for_each(|s| {allowed.insert(hash_str(s));});

            filter_in!(|sentence: &[u8]| {
                std::str::from_utf8(sentence).ok().map(|line|
                    line.split(&sep).nth(column).map(|col| {
                        let cleaned = clean(&col);
                        only_known(&allowed, &cleaned, max_unknown)
                    })
                ).flatten().unwrap_or_else(|| false)
            });
        }

    };
}
