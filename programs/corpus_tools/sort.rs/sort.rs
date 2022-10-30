//!
//! ```cargo
//! [dependencies]
//! term_macros = { path = "../../shared/term_macros"  }
//! ```

#![allow(dead_code)]
#![allow(unused_parens)]
#![allow(unused_macros)]
#![allow(unused_imports)]

fn main() {

    // ideally it should be flexible enough to just take regex, compile or reject them, and then perform calculations based on using them as filters.
    // seems like fundamental compositional pattern is: divide into units (bytes, chars, words), measure properties of each, normalise to btwn 0 and 1
    // awk?

    tool! {

        args: 
            - min: f64 = 0.94;
            - variety_filter: bool = true;
        ;

        body: || {
            let mut scorer = SoftScorer::default();
            scorer.add_scorer(Uppercased { weight: 1.0 });
            scorer.add_scorer(Noisy { weight: 1.0 });
            if variety_filter {
                scorer.add_scorer(WordLengthVariety { weight: 1.0 });
            }


            filter_in!(|line: &[u8]| {
                let score = scorer.score(std::str::from_utf8(line).unwrap());
                if score > min {
                    return true;
                }
                false
            });
        }
    };
}

use log::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::iter::FromIterator;
use std::io::prelude::*;
use core::fmt::Debug;
use term_macros::*;
// info on top 20%
// so two modes: provide statistics, and actually do the filtering
// could use a chunked sorting approach

enum Polarity {
    HigherBetter,
    LowerBetter
}

trait Scorer: Debug {
    fn polarity(&self) -> Polarity;
    fn score(&self, sentence: &str) -> f64;
    fn name(&self) -> &str;
}
// initialisation needs to be handled case-by-case. scorerfactory?
/*

fn uppercased(&self) -> Policy<f64>;
fn long(&self) -> Policy<f64>;
fn noisy(&self) -> Policy<f64>;
fn gzip_against_average(&self) -> Policy<f64>; // how gzippable is it by itself compared to the average?
fn gzip_against_reference(&self) -> Policy<f64>; // how gzippable is it compared to the reference corpus? <-  this is more powerful than the threshold based language detector and more likely to yield good results because it will take relatives into account.
fn infrequent(&self) -> Policy<f64>;
fn weird_whitespace(&self) -> Policy<f64>;
fn irregular_charset

longest word?

*/

macro_rules! scorer {
    ($name:ident { $($prop:ident: $typ:ty),* }, $polar:expr, $($fnbody:tt)+) => {
        #[derive(Debug)]
        struct $name {
            $($prop: $typ),*
        }
        impl $name {
            fn new($($prop: $typ),*) -> Self {
                Self {
                    $($prop),*
                }
            }
        }
        impl Scorer for $name {
            fn name(&self) -> &str {
                stringify!($name)
            }
            fn polarity(&self) -> Polarity {
                $polar
            }
            $(
                    $fnbody
            )+
        }
    }
}

scorer! {
    Uppercased {
        weight: f64
    },
    Polarity::HigherBetter,
    fn score(&self, sentence: &str) -> f64 {
        (1.0 / sentence.chars().filter(|c| c.is_uppercase()).count() as f64) * self.weight
    }
}

scorer! {
    Noisy {
        weight: f64
    },
    Polarity::HigherBetter,
    fn score(&self, sentence: &str) -> f64 {
        (sentence.chars().filter(|c| c.is_whitespace() || c.is_alphabetic()).count() as f64 / sentence.chars().count() as f64) * self.weight
    }
}

scorer! {
    WordLengthVariety {
        weight: f64
    },
    Polarity::HigherBetter,
    fn score(&self, sentence: &str) -> f64 {
        let mut word_lengths = sentence.split(" ").map(|l| l.len()).collect::<Vec<_>>();
        word_lengths.sort();
        let diff = (*word_lengths.iter().next().unwrap() as f64 - *word_lengths.iter().rev().next().unwrap() as f64).abs();
        1.0 - (1.0 / (1.0 + diff))
    }
}

// todo: penalty for upper-case chars in the middle of words. would penalise swahili though?
// contextual meanings: the f64 returned can be an "ideal" number, or a cutoff. enum?
// they should return floats between 0 and 1. boolean?
// activate vs inactive, on vs off, off -> discard, irrelevant.

// several policies: lazily apply hard binary cutoff ranges, or score all of them and sort all of them
// possibly time each of them to see which one is the most computationally expensive, and apply the cheap ones first?
// weights?
trait Metascorer<T> {
    fn score(&self, sentence: &str) -> T;
}

struct HardScorer {
    scorers: HashMap<String, Arc<dyn Scorer + Send + Sync>>,
    cutoffs: HashMap<String, f64>
}

impl HardScorer {
    fn add_scorer<S: Scorer + Send + Sync + 'static>(&mut self, scorer: S, cutoff: f64) {
        self.cutoffs.insert(scorer.name().into(), cutoff);
        self.scorers.insert(scorer.name().into(), Arc::from(scorer));
    }
}

impl Metascorer<bool> for HardScorer {

    fn score(&self, sentence: &str) -> bool {
        let iter = self.scorers.values().map(|s| {
            let polarity = s.polarity();
            let score = s.score(sentence);
            let cutoff = self.cutoffs.get(s.name()).unwrap();
            match polarity {
                Polarity::HigherBetter => score > *cutoff,
                Polarity::LowerBetter => score < *cutoff,
            } 
        });
        for boolean in iter {
            match boolean {
                false => return false,
                true => continue
            }
        }
        true
    }
}

#[derive(Default, Debug)]
struct SoftScorer {
    scorers: HashMap<String, Arc<dyn Scorer + Send + Sync>>
}


impl SoftScorer {
    fn add_scorer<S: Scorer + Send + Sync + 'static>(&mut self, scorer: S) {
        self.scorers.insert(scorer.name().into(), Arc::from(scorer));
    }
}

impl Metascorer<f64> for SoftScorer {
    fn score(&self, sentence: &str) -> f64 {
        self.scorers.values().map(|s| {
            let polarity = s.polarity();
            let score = s.score(sentence);
            //println!("Scored {} as {}", sentence, score);
            match polarity {
                Polarity::HigherBetter => score,
                Polarity::LowerBetter => score * -1.0,
            } 
        }).sum::<f64>() / self.scorers.len() as f64
    }
}

// since this is working within a synchronous loop, we can afford internal mutability
// takes a scorer and derives a final score, or a boolean?