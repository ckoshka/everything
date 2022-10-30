use dashmap::DashMap;
use fnv::FnvHasher;
use nohash_hasher::IntSet;
use rayon::prelude::*;
use std::hash::Hash;
use std::hash::Hasher;
use std::io::Read;
use std::sync::Arc;
use term_macros::*;
use itertools::Itertools;

type PairChanceMap = DashMap<u64, DashMap<u64, f64>>;

fn add_diffs<'a>(a: &Vec<&'a u64>, b: &Vec<&'a u64>, weighting: f64, map: &PairChanceMap) {
    a.iter().for_each(|word| {
        b.iter().for_each(|word2| {
            let prev = map.get(*word);
            prev
                .map(|v| {
                    let curr_map = v.value();
                    let curr_total = curr_map.get(*word2).map(|w| *w.value()).unwrap_or_else(|| 0.0);
                    curr_map.insert(**word2, curr_total + weighting);
                })
                .unwrap_or_else(|| {
                    let inner_map = DashMap::new();
                    inner_map.insert(**word2, weighting);
                    map.insert(**word, inner_map);
                });
        })
    });
}

fn compare(
    s1_lang_a: &IntSet<u64>,
    s2_lang_a: &IntSet<u64>,
    s1_lang_b: &IntSet<u64>,
    s2_lang_b: &IntSet<u64>,
    map: &PairChanceMap,
) {
    let diff_a = s1_lang_a.difference(s2_lang_a).collect::<Vec<_>>();
    let same_a = s1_lang_a.intersection(s2_lang_a).collect::<Vec<_>>();
    let diff_b = s1_lang_b.difference(s2_lang_b).collect::<Vec<_>>();
    let same_b = s1_lang_b.intersection(s2_lang_b).collect::<Vec<_>>();
    // diff + same = set

    add_diffs(&diff_a, &diff_b, 1.0 / (diff_a.len() * diff_b.len()) as f64, map); // what if the weightings were based on lengths?
    // what does longer imply? it means that the probability of a given word being involved with the corrwsponding on the other side
    // is lower. the larger the matching intersections/diffs are, the less information it conveys.
    // if one side is asymmetric, i.e 2 words, then 6 words, then we have 2 * 6.
    add_diffs(&same_a, &same_b, 1.0 / (same_a.len() * same_b.len()) as f64, map);
    add_diffs(&diff_a, &same_b, -1.0 + (1.0 / (same_b.len() + diff_a.len()) as f64), map); //+1?
    // suppose same_b was zero. that would imply all words in language b for s1 and s2 were completely different. which means that no information content is conveyed by their pairing-together.
    // should be subtracted from the total length????
    // whereas for the counterexample? if the words don't align, it's not a property of 
    // it's the ratio of how many words were excluded vs included that constitutes the actual information content.
    // so we need to use a different equation
    // whatever it is, it has to be symmetrical i.e ordering doesn't change the result
    // which means that it must be commutative
    add_diffs(&same_a, &diff_b, -1.0 + (1.0 / (same_a.len() + diff_b.len()) as f64), map);
}

fn hash_str(s: &str) -> u64 {
    let mut h = FnvHasher::with_key(0);
    s.hash(&mut h);
    h.finish()
}

#[derive(Debug, Clone)]
struct Pair {
    pub words: Vec<u64>,
    pub set: IntSet<u64>
}

fn main() {
    tool! {
        args:
            - divider: String = "\t".to_string();
            //- chunk_size: usize = 5;
        ;

        body: || {
            let u32_to_word: DashMap<u64, Arc<str>> = DashMap::new();

            let mut stdin = String::new();
            let _ = std::io::stdin().read_to_string(&mut stdin);

            let data = stdin
                .par_split(|c| c == '\n')
                .filter(|l| l.contains(&divider))
                .map(|line| {
                    let mut iter = line.split(&divider);
                    let part1 = iter.next().unwrap();
                    let part2 = iter.next().unwrap();
                    let words1 = part1.split(" ").filter(|w| w.len() > 0).map(|w| {
                        let h = hash_str(w);
                        u32_to_word.insert(h, Arc::from(w));
                        h
                    }).collect::<Vec<_>>();
                    let words2 = part2.split(" ").filter(|w| w.len() > 0).map(|w| {
                        let h = hash_str(w);
                        u32_to_word.insert(h, Arc::from(w));
                        h
                    }).collect::<Vec<_>>();
                    (Pair {
                        set: words1.iter().cloned().collect::<IntSet<_>>(),
                        words: words1
                    }, Pair {
                        set: words2.iter().cloned().collect::<IntSet<_>>(),
                        words: words2
                    })
                })
                .collect::<Vec<_>>();

            let megamap: DashMap<u64, DashMap<u64, f64>> = DashMap::new();
            // lookups by word would be incredibly slow
            //.filter(|j| (j*i).rem_euclid(chunk_size) == segment)
            (0..data.len()).into_par_iter().for_each(|i| {
                (i+1..data.len()).for_each(|j| {
                    let (i_a, i_b) = &data[i];
                    let (j_a, j_b) = &data[j];
                    compare(&i_a.set, &j_a.set, &i_b.set, &j_b.set, &megamap);
                });
            });

            // make a new table?

            let result = data.par_iter()
                .map(|pair| {
                    let s1 = &pair.0.words;
                    let s2 = &pair.1.words;
                    let all_possible_combos = (0..(s1.len() as u64))
                        .map(|i| (0..(s2.len() as u64))
                            .map(|j| (i, j))
                            .collect::<Vec<_>>()
                        ).flatten().collect::<Vec<_>>();

                    let mut likelihood_table = all_possible_combos.into_iter().map(|(p1, p2)| {
                        let likelihood = megamap.get(&s1[p1 as usize])
                            .map(|inner_map| (*inner_map.value())
                                .get(&s2[p2 as usize])
                                .map(|inner| *inner.value())
                                .unwrap_or_else(|| -10000.0)
                            )
                            .unwrap_or_else(|| -10000.0);
                        ((p1, p2), likelihood) //subtract the other likelihoods?
                    }).collect::<Vec<_>>();

                    /*let mut final_table: Vec<_> = likelihood_table.iter().map(|((p1, p2), likelihood)| {
                        let mut likelihood = likelihood.clone();
                        let other_possible_p1s = (0..(s1.len() as u64)).filter(|i| *i != *p1).map(|i| (i, *p2));
                        let other_possible_p2s = (0..(s2.len() as u64)).filter(|i| *i != *p2).map(|i| (*p1, i));
                        let mut closure = |other_pair| {
                            likelihood /= likelihood_table.get(&other_pair).unwrap(); // leading to bad results
                        };
                        other_possible_p1s.for_each(&mut closure);
                        other_possible_p2s.for_each(&mut closure);
                        ((*p1, *p2), likelihood)
                    }).collect();*/

                    likelihood_table.sort_by_key(|(_, likelihood)| (*likelihood * -1000000.0) as i64);

                    let into_moses_fmt = |(p1, p2): (&u64, &u64)| format!("{p1}-{p2}");

                     /*let mut deduped = Vec::new();

                    for p1 in 0..(s1.len() as u64) {
                        all_possible_combos.iter()
                        .filter()
                            .find(|pxs| pxs.0 == p1)
                            .map(|pxs| pxs.1)
                            .unwrap()
                    }*/
                    // needs pxs.1 to be filtered by whether a match was already found, so we need
                            // a loop and an accumulator

                    let deduped = likelihood_table
                        .iter()
                        .map(|((p1, p2), _)| (p1, p2))
                        .unique_by(|(p1, _)| *p1)
                        .unique_by(|(_, p2)| *p2)
                        .map(into_moses_fmt)
                        .collect::<Vec<_>>();

                    deduped

                }).collect::<Vec<_>>();        

            result.iter().for_each(|alignments| {
                println!("{}", alignments.join(" "));
            });
        }
    }
}
