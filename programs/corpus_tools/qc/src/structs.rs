use crate::{SentenceId, SortPositions};
use itertools::Itertools;
use rayon::prelude::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;

pub struct Side {
    pub content: Arc<str>,
    pub words: Vec<Arc<str>>,
}

impl Side {
    pub fn new(content: &str) -> Side {
        Side {
            content: Arc::from(content),
            words: content.unicode_words().map(|s| Arc::from(s)).collect(),
        }
    }
}

pub struct Translation {
    pub id: SentenceId,
    pub sides: (Side, Side),
}

impl Translation {
    pub fn new(c1: &str, c2: &str, id: SentenceId) -> Self {
        Translation {
            sides: (Side::new(c1), Side::new(c2)),
            id,
        }
    }
}

pub trait ScoreOne {
    fn score_one(&self, tx: &Side) -> f64;
}

pub trait ScoreBoth {
    fn score_both(&self, tx: &Translation) -> f64;
}

impl<T> ScoreBoth for T
where
    T: ScoreOne,
{
    fn score_both(&self, tx: &Translation) -> f64 {
        self.score_one(&tx.sides.0) + self.score_one(&tx.sides.1)
    }
}

pub trait Sorter {
    fn sort(&self, txs: &[Translation]) -> SortPositions;
}

impl<T> Sorter for T
where
    T: ScoreBoth + Send + Sync,
{
    fn sort(&self, txs: &[Translation]) -> SortPositions {
        let mut scored_txs: Vec<_> = txs
            .iter()
            .map(|tx| (tx, self.score_both(&tx)))
            .collect();
        scored_txs.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or_else(|| std::cmp::Ordering::Equal)
        });
        scored_txs
            .into_iter()
            .enumerate()
            .map(|(i, (tx, _))| (tx.id, i))
            .collect()
    }
}

// 1 is good, 0 is bad

pub struct Redundant;

impl ScoreOne for Redundant {
    fn score_one(&self, tx: &Side) -> f64 {
        tx.words.iter().dedup().count() as f64 / tx.words.len() as f64
    }
}

pub struct Untranslated;

impl ScoreBoth for Untranslated {
    fn score_both(&self, tx: &Translation) -> f64 {
        let side_1 = &tx.sides.0.words;
        let side_2 = &tx.sides.1.words;
        1.0 - (side_1.iter().chain(side_2.iter()).dedup().count() as f64
            / (side_1.len() + side_2.len()) as f64)
    }
}

pub struct OutOfFrequency {
    freqs: HashSet<Arc<str>>,
}

impl OutOfFrequency {
    pub fn from_txs(txs: &Vec<Translation>, cutoff: usize) -> OutOfFrequency {
        let mut map = HashMap::new();
        txs.iter().for_each(|tx| {
            tx.sides.0.words.iter().for_each(|w| {
                let prev = map.get(w).map(|i| *i).unwrap_or_else(|| 0);
                map.insert(Arc::from(w.to_lowercase().as_str()), prev + 1);
            });
            tx.sides.1.words.iter().for_each(|w| {
                let prev = map.get(w).map(|i| *i).unwrap_or_else(|| 0);
                map.insert(Arc::from(w.to_lowercase().as_str()), prev + 1);
            });
        });

        let mut freqs: Vec<_> = map.into_iter().collect();
        freqs.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or_else(|| std::cmp::Ordering::Equal)
        });
        OutOfFrequency {
            freqs: freqs.into_iter().take(cutoff).map(|(w, _)| w).collect(),
        }
    }
}

impl ScoreOne for OutOfFrequency {
    fn score_one(&self, tx: &Side) -> f64 {
        tx.words.iter().filter(|w| self.freqs.contains(*w)).count() as f64 / tx.words.len() as f64
    }
}

pub struct NonAlphabetic;

impl ScoreOne for NonAlphabetic {
    fn score_one(&self, tx: &Side) -> f64 {
        tx.content.chars().filter(|c| c.is_alphabetic() || c.is_whitespace()).count() as f64 / tx.content.len() as f64
    }
}

pub struct Capitals;

impl ScoreOne for Capitals {
    fn score_one(&self, tx: &Side) -> f64 {
        (1.0 + tx
            .content
            .chars()
            .skip(1)
            .filter(|c| c.is_lowercase())
            .count() as f64)
            / tx.content.len() as f64
    }
}

pub struct CharRange {
    side_1: (usize, usize),
    side_2: (usize, usize),
}

fn abs_ratio(n1: f64, n2: f64) -> f64 {
    (1.0 - (n1 / n2)).abs()
}

fn score_char_range(range: &(usize, usize), content: &Arc<str>) -> f64 {
    let chars = get_codepoints(content);
    (abs_ratio(
        chars.iter().min().map(|x| *x).unwrap_or_else(|| range.0) as f64,
        range.0 as f64,
    ) + abs_ratio(
        chars.iter().max().map(|x| *x).unwrap_or_else(|| range.1) as f64,
        range.1 as f64,
    )) / 2.0
}

fn get_codepoints(s: &str) -> Vec<usize> {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .map(|c| c as usize)
        .collect()
}

impl CharRange {
    fn calc_range(txs: &Vec<Translation>, side: usize) -> (usize, usize) {
        let _summed_1: (usize, usize) = txs
            .par_iter()
            .map(|tx| {
                let chars = if side == 0 {
                    get_codepoints(&tx.sides.0.content)
                } else {
                    get_codepoints(&tx.sides.1.content)
                };
                (
                    chars.iter().max().map(|x| *x),
                    chars.iter().min().map(|x| *x),
                )
            })
            .filter(|(c1, c2)| c1.is_some() && c2.is_some())
            .map(|(c1, c2)| (c1.unwrap(), c2.unwrap()))
            .reduce(|| (0, 0), |prev, curr| (prev.0 + curr.0, prev.1 + curr.1));
        (_summed_1.0 / txs.len(), _summed_1.1 / txs.len())
    }
    pub fn from_txs(txs: &Vec<Translation>) -> CharRange {
        CharRange {
            side_1: CharRange::calc_range(txs, 0),
            side_2: CharRange::calc_range(txs, 1),
        }
    }
}

impl ScoreBoth for CharRange {
    fn score_both(&self, tx: &Translation) -> f64 {
        let side_1 = &tx.sides.0.content;
        let side_2 = &tx.sides.1.content;
        score_char_range(&self.side_1, side_1) + score_char_range(&self.side_2, side_2)
    }
}

pub struct WellFormed;

impl ScoreOne for WellFormed {
    fn score_one(&self, tx: &Side) -> f64 {
        let mut score = 0.0;
        if tx
            .content
            .chars()
            .next()
            .unwrap_or_else(|| 'a')
            .is_uppercase()
        {
            score += 0.5;
        }
        if tx
            .content
            .chars()
            .last()
            .unwrap_or_else(|| 'a')
            .is_ascii_punctuation()
        {
            score += 0.5;
        }
        score
    }
}

pub struct IdealLength(pub f64);

impl ScoreOne for IdealLength {
    fn score_one(&self, tx: &Side) -> f64 {
        1.0 / (tx.words.len() as f64 - self.0).abs()
    }
}

pub struct LengthDifference;

impl ScoreBoth for LengthDifference {
    fn score_both(&self, tx: &Translation) -> f64 {
        1.0 - (tx.sides.0.words.len() as f64 / tx.sides.1.words.len() as f64).abs()
    }
}

#[test]
fn test_trait() {
    let tx = Translation::new(
        "Gorbachev's new finance minister promises an end to economic planning",
        "Finanz ministra gorbachevu apromisa encor por planettement economique",
        2,
    );
    let red = Redundant;
    let vec = vec![tx];
    red.sort(&vec);
}
