

use std::{sync::Arc};
use rayon::prelude::*;
use term_macros::*;
use dashmap::DashMap;
use memmap::MmapOptions;
use std::io::Write;
pub fn main() {
    tool! {
        args:
            - filename: String;
            - max_ngram_size: usize = 5;
            - min_ngram_size: usize = 1;
            - filter_below: i32 = 1;
            - top_n: usize = 30000;
        ;
        body: || {
            let file = std::fs::File::open(filename).unwrap();
            let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };

            let map: DashMap<Arc<[u8]>, i32> = DashMap::with_capacity(1000000);

            let lines: Vec<_> = mmap[..].split(|c| c == &b'\n').collect();

            lines.into_par_iter().for_each(|byteline: &[u8]| {
                let words: Vec<_> = byteline.split(|c| c == &b' ').collect();
                (min_ngram_size..max_ngram_size).for_each(|n| {
                    if words.len() < n {
                        return;
                    }
                    (0..(words.len() - n)).for_each(|k| {
                        let phrase = &words[k..k+n];
                        let joined = phrase.join(vec![b' '].as_slice());
                        if let Some(mut prev_count) = map.get_mut(joined.as_slice()) {
                            *prev_count += 1;
                        } else {
                            map.insert(Arc::from(joined), 1);
                        }
                    })
                })
                /* .for_each(|word| {
                    if let Some(prev_count) = map.get_mut(word) {
                        *prev_count += 1;
                    } else {
                        map.insert(Arc::from(word), 1);
                    }
                })*/
            });
            let mut v: Vec<(Arc<[u8]>, i32)> = map.into_iter().collect();
            v.sort_by_key(|(_word, count)| -count);
            let v = v.into_iter().filter(|(_word, count)| *count > filter_below).take(top_n);
            let mut wtr = std::io::BufWriter::new(std::io::stdout());
            for (word, _count) in v {
                let result = wtr.write_all(&word);
                let _ = wtr.write_all(b"\n");
                if result.is_err() {
                    break;
                }
            }
        }
    }
}