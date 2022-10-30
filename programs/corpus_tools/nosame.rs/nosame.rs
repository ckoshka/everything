use fnv::FnvHasher;
use nohash_hasher::IntSet;
use std::hash::Hash;
use std::hash::Hasher;
use term_macros::*;

fn hash_str(s: &str) -> u64 {
    let mut h = FnvHasher::with_key(0);
    s.hash(&mut h);
    h.finish() as u64
}

fn main() {
    let mut set1 = IntSet::<u64>::default();
    let mut set2 = IntSet::<u64>::default();
    let into_set = |data: &str, set: &mut IntSet<u64>| {
        data.chars()
            .filter(|c| c.is_alphabetic() || c.is_whitespace())
            .map(|c| c.to_lowercase())
            .flatten()
            .collect::<String>()
            .split(|c: char| c.is_whitespace())
            .filter(|w| w.len() > 2)
            .map(|w| hash_str(w))
            .for_each(|w| {
                set.insert(w);
            })
    };
    filter_in!(|line: &[u8]| {
        let line = std::str::from_utf8(line);
        if line.is_err() {
            return false;
        }
        let line = line.unwrap();
        let mut split = line.split(|c| c == '\t');
        let part1 = split.next();
        let part2 = split.next();
        if part2.is_none() {
            return false;
        }
        into_set(part1.unwrap(), &mut set1);
        into_set(part2.unwrap(), &mut set2);
        let passed_check = set1.intersection(&set2).next().is_none();
        set1.clear();
        set2.clear();
        passed_check
    });
}
