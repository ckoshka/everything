use rayon::prelude::*;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::io::Read;
//Imports Arc:

#[derive(Debug, PartialEq, Clone)]
pub struct Sentence<'a> {
    pub index: usize,
    pub length: usize,
    pub outgoing_connections: Option<HashMap<usize, usize>>, //first represents sentence index, second represents the number of outgoing connections
    pub text: Cow<'a, [u8]>,
    pub words: HashSet<Cow<'a, [u8]>>,
    pub number_of_connections: f32,
}

#[derive(Debug, Clone)]
pub struct Summariser<'a> {
    pub sentences: HashMap<usize, Sentence<'a>>,
    bias_list: HashSet<Cow<'a, [u8]>>,
    bias_strength: Option<f32>,
}

impl<'a> Summariser<'a> {
    pub fn from_raw_text(
        raw_text: &'a str,
        separator: &str,
        min_length: usize,
        max_length: usize,
        bias_strength: Option<f32>,
    ) -> Summariser<'a> {
        let mut sentences = HashMap::new();
        let all_sentences = raw_text.split(|c|c == '.').collect::<Vec<&str>>();
        for (i, sentence) in all_sentences.iter().enumerate() {
            if sentence.len() > min_length && sentence.len() < max_length {
                let words = HashSet::from_iter(
                    sentence
                        .split_whitespace()
                        .map(|word| Cow::Borrowed(word.as_bytes())),
                );
                let outgoing_connections = HashMap::new();
                let sentence = Sentence {
                    index: i,
                    length: sentence.len(),
                    outgoing_connections: Some(outgoing_connections),
                    text: Cow::Borrowed(sentence.as_bytes()),
                    words,
                    number_of_connections: 0.0,
                };
                sentences.insert(i, sentence.clone());
            }
        }
        Summariser {
            sentences: sentences,
            bias_list: HashSet::new(),
            bias_strength: bias_strength,
        }
    }
    pub fn top_sentences(
        &mut self,
        number_of_sentences_to_return: usize,
        length_penalty: f32,
        density: f32,
        bias_list: Option<HashSet<Cow<'a, [u8]>>>,
        bias_strength: Option<f32>,
    ) -> Vec<Sentence> {
        if bias_list.is_some() {
            self.bias_list = bias_list.clone().unwrap();
        }
        if bias_strength.is_some() {
            self.bias_strength = bias_strength.clone();
        } else {
            self.bias_strength = Some(2.0);
        }
        let length_of_sentences = self.sentences.len();
        let mut matrix = vec![vec![0.0; length_of_sentences.clone()]; length_of_sentences.clone()];
        matrix.par_iter_mut().enumerate().for_each(|(i, row)| {
            for j in i + 1..length_of_sentences {
                if let Some(sentence) = self.sentences.get(&i.clone()) {
                    row[j] = (self.number_of_word_connections(i.clone(), j.clone()) as f32)
                        .powf(density)
                        / (sentence.length as f32).powf(length_penalty); //1.1
                }
            }
        });
        let mut top_sentences = matrix
            .iter()
            .enumerate()
            .map(|(i, row)| (row.iter().sum::<f32>(), i))
            .filter(|(sum, _)| *sum > 0.0)
            .collect::<Vec<(f32, usize)>>();
        top_sentences.par_sort_unstable_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        let top_sentences_indices = top_sentences
            .iter()
            .take(number_of_sentences_to_return)
            .map(|x| x.1)
            .collect::<Vec<usize>>()
            .iter()
            .filter(|x| self.sentences.contains_key(x))
            .map(|x| self.sentences.get(x).unwrap().clone())
            .collect::<Vec<Sentence>>();
        top_sentences_indices
    }

    pub fn number_of_word_connections(
        &'a self,
        sentence_a_indx: usize,
        sentence_b_indx: usize,
    ) -> f32 {
        if let Some(sentence_a) = self.sentences.get(&sentence_a_indx) {
            if let Some(sentence_b) = self.sentences.get(&sentence_b_indx) {
                let intersection_length = sentence_a.words.intersection(&sentence_b.words).count();
                if self.bias_list.len() > 0 {
                    let overlapping_words_with_b_length = self
                        .bias_list
                        .intersection(&sentence_b.words)
                        .collect::<Vec<_>>()
                        .len()
                        + self
                            .bias_list
                            .intersection(&sentence_a.words)
                            .collect::<Vec<_>>()
                            .len();
                    return intersection_length as f32
                        * (1.0
                            + ((overlapping_words_with_b_length as f32 * 3.0)
                                .powf(self.bias_strength.unwrap())
                                / (sentence_a.length as f32).powf(0.64)));
                }
                return intersection_length as f32;
            } else {
                return 0.0;
            }
        } else {
            return 0.0;
        }
    }
}

pub fn summarise_file<'a>(bias_list: HashSet<Cow<'a, [u8]>>, filename: &str) -> Vec<String> {
    // Read the raw bytes
    let mut f = std::fs::File::open(filename).unwrap();
    let mut reader = std::io::BufReader::with_capacity(99999, &mut f);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).unwrap();
    let raw_text = std::str::from_utf8(&buffer).unwrap();
    let mut summariser = Summariser::from_raw_text(
        raw_text,
        ".",
        11,
        170,
        Some(2.0),
    );
    let sentences = summariser.top_sentences(500000, 0.75, 2.6, Some(bias_list), None);
    unsafe { sentences.into_iter().map(|sentence| String::from_utf8_unchecked(sentence.text.to_vec())).collect::<Vec<_>>() }
}