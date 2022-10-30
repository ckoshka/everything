use term_macros::*;
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;
use nohash_hasher::IntSet;
use std::io::Read;
use rayon::prelude::*;
use std::hash::Hash;
use fnv::FnvHasher;
use std::hash::Hasher;
use dashmap::DashMap;
// this would be very specialised for the format we're using (tsv, tab-separated, english on one side,)
// unless...
// just provide the column in the given language?
// so dicer.rs needs to panic at invalid utf-8 otherwise line alignment will go out of whack.
// what we'd essentially be doing is:
// generate a .msgpack file
// hash incrementing up
// generate a mapping of each u32 to the word
// if we assume it's already lower-cased, we don't need a string allocation.
// doesn't really matter since this is a preproc step.

fn hash_str(s: &str) -> u32 {
    let mut h = FnvHasher::with_key(0);
    s.hash(&mut h);
    h.finish() as u32
}

fn main() {
    tool! {
        args:
            - dictionary_filename: String;
            - encodings_filename: String;
        ;
        body: || {
            let map = DashMap::new();
            let mut stdin = String::new();
            std::io::stdin().read_to_string(&mut stdin).unwrap();

            let lines: Vec<_> = stdin.par_split(|b| b == '\n')
                .map(|line| {
                    let line = line.to_lowercase();
                    line.unicode_words().map(|w| {
                        if map.contains_key(w) {
                            //
                        } else {
                            map.insert(w.to_string(), hash_str(w));
                        }
                        *map.get(w).unwrap()
                    }).collect::<IntSet<_>>()
                })
                .collect();

            let mut dict_file = std::io::BufWriter::new(std::fs::File::create(dictionary_filename).unwrap());
            let map: HashMap<_, _> = map.into_iter().collect();
            rmp_serde::encode::write(&mut dict_file, &map).unwrap();

            let mut encodings_file = std::io::BufWriter::new(std::fs::File::create(encodings_filename).unwrap());
            rmp_serde::encode::write(&mut encodings_file, &lines).unwrap();
        }
    }
}
