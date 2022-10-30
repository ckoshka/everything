//!
//! ```cargo
//! [dependencies]
//! rayon = "1.5.3"
//! memmap = "0.7.0"
//! dashmap = "*"
//! ```

use rayon::prelude::*;
use memmap:: MmapOptions;
use dashmap::DashMap;
// only pay attention to the second tabbed column

fn no_punctuation(w: &[u8]) -> Vec<u8> {
    w.iter().cloned().filter(|c| !c.is_ascii_punctuation()).collect()
}

// assume that these have already been split by tab
fn calc_freq<'a>(items: Vec<&'a [u8]>) -> DashMap<Vec<u8>, i64> {
    let map = DashMap::with_capacity(items.len());
    if (0..5).map(|i| items[i].split(|c| c == &b' ').count()).filter(|x| *x == 1).count() > 3 {
        return map;
    };
    items.par_iter().for_each(|s| {
        s.split(|c| c == &b' ').for_each(|w| {
            let key = no_punctuation(w);
            if let Some(mut entry) = map.get_mut(&key) {
                *entry += 1;
            } else {
                map.insert(key, 1);
            }
        })
    });
    map
}

fn cardinalise(map: DashMap<Vec<u8>, i64>) -> DashMap<Vec<u8>, i64> {
    let mut freqs: Vec<_> = map.into_iter().collect();
    freqs.par_sort_by_key(|(_k, v)| *v as i64);

    freqs.into_iter().enumerate().map(|(i, (k, _v))| {
        (k, i as i64)
    }).collect()
}

fn score_line(map: &DashMap<Vec<u8>, i64>, line: &[u8]) -> i64 {
    let clean = no_punctuation(line);
    let words = clean.split(|c| c == &b' ').collect::<Vec<_>>();
    words.iter().map(|w| {
        *map.get(&w.to_vec()).unwrap() as i64
    }).max().unwrap()
}

fn main() {
    let filename = std::env::args().nth(1).expect("Provide a filename");
    let map = unsafe { MmapOptions::new().map(&std::fs::File::open(filename).unwrap()).unwrap() };
    let lines = map.par_split(|c| c == &b'\n').map(|line| {
        //let mut l = line.split(|c| c == &b'\t');
        //l.nth(1)
        Some(line)
    }).filter(|k| k.is_some()).map(|k| k.unwrap()).collect::<Vec<_>>();
    let freqs = cardinalise(calc_freq(lines));
    let mut scored_lines = map.par_split(|c| c == &b'\n').map(|line| {
        (score_line(&freqs, line), line)
    }).collect::<Vec<_>>();
    //let mut scored_lines = scored_lines.into_iter().collect::<Vec<_>>();
    scored_lines.par_sort_by_key(|(score, _p1)| *score * -1);
    for line in scored_lines.into_iter() {
        print!("{}\n", std::str::from_utf8(line.1).unwrap());
    }
}