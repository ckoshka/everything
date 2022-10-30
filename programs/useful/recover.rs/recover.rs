//!
//! ```cargo
//! [dependencies]
//! term_macros = { path = "../../shared/term_macros"  }
//! fnv = "1.0.7"
//! nohash-hasher = "0.2.0" 
//! ```

use term_macros::*;
use nohash_hasher::IntMap;

fn hash_str(s: &[u8]) -> u64 {
    use fnv::FnvHasher;
    use std::hash::Hash;
    use std::hash::Hasher;
    let mut h = FnvHasher::with_key(0);
    s.hash(&mut h);
    h.finish()
}

fn open(name: &str, key_column: usize, sep: char) -> IntMap<u64, String> {
    use std::io::Read;
    let mut string = String::new();
    let _ = std::fs::File::open(&name).unwrap().read_to_string(&mut string);
    string.split("\n")
        .filter(|s| s.contains(sep))
        .map(|s| (hash_str(s.split(sep).nth(key_column).unwrap().as_bytes()), s.to_owned()))
        .collect()
}

fn main() {
    tool! {
        args:
            - file: String;
            - key_column: usize;
            - sep: String = "\t".to_string();
        ;

        body: || {
            let mappings = open(&file, key_column, sep.chars().next().unwrap());

            readin!(wtr, |line: &[u8]| {
                let hash = hash_str(&line[0..(line.len() - 1)]);
                let _ = mappings.get(&hash).map(|line| {
                    let _ = wtr.write_all(&line.as_bytes());
                    let r = wtr.write_all(b"\n");
                    if r.is_err() {
                        panic!("Unable to write")
                    }
                });
            });
        }

    };
}