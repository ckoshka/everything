//!
//! ```cargo
//! [dependencies]
//! term_macros = { path = "../../shared/term_macros"  }
//! rayon = "1.5.3"
//! nohash-hasher = "0.2.0" 
//! fnv = "1.0.7"
//! mimalloc = { version = "0.1", default-features = false }
//! ```

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
use fnv::FnvHasher;
use nohash_hasher::{IntSet};
use rayon::prelude::*;
use term_macros::*;
use std::hash::Hasher;
use std::hash::Hash;
use std::io::Read;

struct Sentence<'a> {
    repr: &'a str,
    unknowns: IntSet<u64>,
    skip: bool
}

type Filename = String;

fn open(name: &Filename) -> Vec<String> {
    let mut string = String::new();
    let _ = std::fs::File::open(&name).unwrap().read_to_string(&mut string);
    string.split("\n").map(|s| s.to_owned()).collect()
}

fn no_punctuation(w: &str) -> String {
    w.chars().map(|c| c.to_lowercase()).flatten().filter(|c| c.is_alphabetic() || (c.is_whitespace() && c != &'\n')).collect()
}

fn hash_str(s: &str) -> u64 {
    let mut h = FnvHasher::with_key(0);
    s.hash(&mut h);
    h.finish()
}

fn main() {
    tool! {
        args:
            - wordlist: Filename;
            - restrict_search_to: usize = 50;
            - per_word: usize = 3;
            - sort_longest: bool = false;
        ;
        body: || {
            let words = open(&wordlist).into_iter().map(|w| hash_str(&w)).collect::<Vec<_>>();
            let mut sentences_str = String::new();
            std::io::stdin().read_to_string(&mut sentences_str).unwrap();
            
            let mut sentences: Vec<Sentence> = sentences_str.par_split(|c| c == '\n').map(|s| 
                Sentence {
                    repr: s,
                    unknowns: no_punctuation(&s).split(" ").map(|w| hash_str(w)).collect(),
                    skip: false
                }
            ).collect();
            
            for word in words.iter() { // instead of iterating over the words here, we could have them piped in, then the sentences specified within a file. we should clone this and adapt it elsewhere.
                // and we also need to abstract this to accept msgpack 
                let mut subset: Vec<_> = sentences.iter()
                    .filter(|s| s.skip == false)
                    .filter(|s| s.unknowns.len() == 1 && s.unknowns.iter().next().unwrap() == word)
                    .take(restrict_search_to)
                    .collect();
                if sort_longest {
                    subset.sort_by_key(|a| a.repr.len());
                }
                subset.iter().take(per_word).rev().for_each(|s| {
                    println!("{}", s.repr);
                });
                sentences.par_iter_mut()
                .filter(|s| s.skip == false)
                    .for_each(|s| {
                        s.unknowns.remove(&word);
                    });
                sentences.par_iter_mut().filter(|s| s.skip == false).for_each(|s| if s.unknowns.len() == 0 { s.skip = true });
            }
        }
    }
}