use std::{borrow::Cow, future::Future};
use genawaiter::sync::gen;
use rayon::prelude::*;
use super::summariser::summarise_file;
use regex::Regex;
use once_cell::sync::Lazy;
use rand::Rng;
use genawaiter::yield_;

static REMOVE_NONALPHABETIC: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[^\w\s]").unwrap()
});

static VOWEL_GROUP_COUNTER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([aeiuo]+)").unwrap()
});

static CONSONANT_GROUP_COUNTER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([dlmnrstz]y)").unwrap()
});

static AXE_COUNTER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([aiou][b-df-hj-np-rs-v-z]e)").unwrap()
});

static AXE_EXCEPTION_COUNTER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([aiou][cs]es)").unwrap()
});

static ASTE_COUNTER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(aste)").unwrap()
});

static APSE_COUNTER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(apse)").unwrap()
});

static TED_COUNTER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([^b-df-hj-np-tv-z]ted$)").unwrap()
});

static E_COUNTER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(e[rsvy]e)").unwrap()
});

static D_COUNTER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(d[nv])").unwrap()
});

static ELVE_COUNTER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(elve[^t])").unwrap()
});

static EING_COUNTER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(eing)").unwrap()
});

pub fn count_syllables(word: &str) -> usize {
    let word = REMOVE_NONALPHABETIC.replace_all(word, "").to_lowercase();
    let mut count = VOWEL_GROUP_COUNTER.captures_iter(&word).count();
    count += CONSONANT_GROUP_COUNTER.captures_iter(&word).count();
    count -= AXE_COUNTER.captures_iter(&word).count();
    count += AXE_EXCEPTION_COUNTER.captures_iter(&word).count();
    count -= ASTE_COUNTER.captures_iter(&word).count();
    count -= APSE_COUNTER.captures_iter(&word).count();
    count += TED_COUNTER.captures_iter(&word).count();
    count -= E_COUNTER.captures_iter(&word).count();
    count += D_COUNTER.captures_iter(&word).count();
    count -= ELVE_COUNTER.captures_iter(&word).count();
    count += EING_COUNTER.captures_iter(&word).count();
    count
}

pub fn count_sentence_syllables(sentence: &str) -> Vec<usize> {
    sentence.split_whitespace().map(count_syllables).collect()
}

pub struct Line {
    text: String,
    syllables: Vec<usize>,
}

// Tokenise into sentences via non-whitespace punctuation
pub fn tokenise_sentences(sentence: &str) -> Vec<Line> {
    sentence.as_parallel_string().par_split(|c: char| c == ',' || c == '.' || c == ';').filter(|sen| sen.len() < 60 || sen.len() > 7).map(|sen| {
        Line {
            text: sen.to_lowercase().chars().filter(|c|c.is_alphabetic() || c.is_whitespace()).collect(),
            syllables: count_sentence_syllables(sen),
        }
    }).collect()
}

// Filter a vec of sentences by syllable size
pub fn filter_sentences(sentences: &Vec<Line>, syllables: usize) -> Vec<& Line> {
    sentences.into_iter().filter(|sen| sen.syllables.iter().sum::<usize>() == syllables).collect()
}

pub fn generate_haikus(filename: &str) -> genawaiter::sync::Gen<String, (), impl Future<Output = ()>> {
    let filename = filename.to_string();
    gen!({
        let sentences = summarise_file(std::collections::HashSet::from_iter(vec![]), &filename).into_iter().map(|sentence| {
            Line {
                text: sentence.to_lowercase().chars().filter(|c|c.is_alphabetic() || c.is_whitespace()).collect(),
                syllables: count_sentence_syllables(&sentence),
            }
        }).collect();
        let sentence_ref = &sentences;
        let mut haikus = filter_sentences(sentence_ref, 17).into_iter();
        loop {
            // What we're looking for is a very specific pattern. At this point, the 'syllables' field of each sentence will look something like [2, 3, 8, 1, 1, 1] and so on.
            let mut haiku = String::new();
            let curr_sentence = haikus.next();
            if curr_sentence.is_none() {
                break;
            }
            let curr_sentence = curr_sentence.unwrap();
            let mut syllable_total = 0;
            let mut last_idx_5_1 = 0;
            let words = curr_sentence.text.split_whitespace().collect::<Vec<_>>();
            let mut stage_1_failed = false;
            for (idx, word) in words.iter().enumerate() {
                syllable_total += curr_sentence.syllables[idx];
                if syllable_total == 5 {
                    last_idx_5_1 = idx;
                    haiku.push_str(&words[..=last_idx_5_1].join(" "));
                    haiku.push_str("\n");
                    break;
                }
                if syllable_total > 5 {
                    stage_1_failed = true;
                    break;
                }
            }
            if stage_1_failed {
                continue;
            }
            let mut syllable_total = 0;
            let mut stage_1_failed = false;
            let mut last_idx_7 = 0;
            for (idx, word) in words.iter().enumerate().skip(last_idx_5_1 + 1) {
                syllable_total += curr_sentence.syllables[idx];
                if syllable_total == 7 {
                    last_idx_7 = idx;
                    haiku.push_str(&words[last_idx_5_1 + 1..=last_idx_7].join(" "));
                    haiku.push_str("\n");
                    break;
                }
                if syllable_total > 7 {
                    stage_1_failed = true;
                    break;
                }
            }
            if stage_1_failed {
                continue;
            }
            let mut syllable_total = 0;
            let mut stage_1_failed = false;
            let mut last_idx_5_2 = 0;
            for (idx, word) in words.iter().enumerate().skip(last_idx_7 + 1) {
                syllable_total += curr_sentence.syllables[idx];
                if syllable_total == 5 {
                    last_idx_5_2 = idx;
                    haiku.push_str(&words[last_idx_7 + 1..=last_idx_5_2].join(" "));
                    haiku.push_str("\n");
                    break;
                }
                if syllable_total > 5 {
                    stage_1_failed = true;
                    break;
                }
            }
            if stage_1_failed {
                continue;
            }
            if haiku.chars().filter(|c| *c == '\n').count() != 3 {
                continue;
            }
            yield_!(haiku);
        }
    })
}


// Bash command for testing haikufinder/src/regexer/test_count_syllables:
// ```
// cargo test -- --nocapture --test regexer::test_count_syllables