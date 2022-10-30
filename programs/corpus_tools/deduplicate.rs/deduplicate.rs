//!
//! ```cargo
//! [dependencies]
//! fnv = "1.0.7"
//! nohash-hasher = "0.2.0"
//! term_macros = { path = "../../shared/term_macros"  }
//! ```
use term_macros::*;
//use genawaiter::stack::let_gen;
//use genawaiter::yield_;
use fnv::FnvHasher;
use std::hash::Hash;
use std::hash::Hasher;
use nohash_hasher::IntSet;

fn no_punctuation(w: &[u8]) -> Vec<u8> {
    w.iter().cloned().filter(|c| !c.is_ascii_punctuation()).collect()
}

fn hash_str(s: &[u8]) -> u64 {
    let mut h = FnvHasher::with_key(0);
    s.hash(&mut h);
    h.finish()
}

fn main() {
    tool! {
        args:
            - no_punct;
        ;
        body: || {
            let mut already_seen = IntSet::default();
            let hashfn: &dyn Fn(&[u8]) -> u64 = match no_punct {
                true => &|line: &[u8]| hash_str(&no_punctuation(line)),
                false => &|line: &[u8] | hash_str(line)
            };
            filter_in!(|line: &[u8]| {
                let hashnum = hashfn(line);
                if already_seen.contains(&hashnum) {
                    false
                } else {
                    already_seen.insert(hashnum);
                    true
                }
            });
        }
    }
}