use dashmap::DashMap;
use nohash_hasher::{IntMap, IntSet, NoHashHasher};
use rustc_hash::FxHasher;
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    error::Error,
    fs::File,
    hash::{Hash, Hasher},
    io::{Read, Write},
    path::Path,
    sync::Arc,
};
use rayon::prelude::*;

fn hash<T: std::hash::Hash>(value: T) -> u64 {
    let mut hasher = FxHasher::default();
    value.hash(&mut hasher);
    hasher.finish()
}

#[derive(Clone)]
struct Sentence {
    filtered_out: bool,
    id: u64,
    words: IntSet<u64>,
    original: Arc<str>,
    overlaps: DashMap<u64, f32>,
    source: Arc<str>,
}

#[derive(Serialize, Deserialize)]
struct SerializableSentence {
    filtered_out: bool,
    id: u64,
    words: Vec<u64>,
    original: String,
    overlaps: HashMap<u64, f32>,
    source: String,
}

impl From<SerializableSentence> for Sentence {
    fn from(sentence: SerializableSentence) -> Self {
        Sentence {
            filtered_out: sentence.filtered_out,
            id: sentence.id,
            words: sentence.words.into_iter().collect(),
            original: Arc::from(sentence.original),
            overlaps: sentence.overlaps.into_iter().collect(),
            source: Arc::from(sentence.source),
        }
    }
}

impl From<&Sentence> for SerializableSentence {
    fn from(sentence: &Sentence) -> Self {
        SerializableSentence {
            filtered_out: sentence.filtered_out,
            id: sentence.id,
            words: sentence.words.iter().cloned().collect(),
            original: sentence.original.to_string(),
            overlaps: sentence
                .overlaps
                .iter()
                .map(|kv| (kv.key().clone(), kv.value().clone()))
                .collect(),
            source: sentence.source.to_string(),
        }
    }
}

impl Sentence {
    pub fn calc_overlap(&self, other: &IntSet<u64>) -> f32 {
        (self.words.intersection(&other).count() + 1) as f32// / (self.words.difference(other).count() + 1) as f32
    }


    pub fn set_overlap(&self, id: u64, overlap: f32) {
        self.overlaps.insert(id, overlap);
    }


    pub fn mark_as_filtered(&mut self) {
        self.filtered_out = true;
    }
}

/// Takes a range, generates a random number within the range, if the number is equal to end, it returns true.
fn random_in_range_or_end(start: usize, end: usize) -> bool {
    use rand::{thread_rng, Rng, RngCore};
    let range = (start..end).collect::<Vec<usize>>();
    let mut rng = rand::thread_rng();
    let random_number = rng.gen_range(0..range.len());
    random_number == range.len() - 1
}

struct Engine {
    db: HashMap<u64, Sentence>,
}

impl Engine {
    pub fn new() -> Engine {
        Engine {
            db: HashMap::new()
        }
    }
    pub fn best(&self, query: IntSet<u64>) -> Vec<(u64, f32)> {
        let now = std::time::Instant::now();
        let commonalities: IntMap<u64, f32> = self
            .db
            .par_iter()
            .map(|kv| {
                let value = kv.1;
                let overlap = value.calc_overlap(&query).powf(3.0);
                (kv.0.clone(), overlap as f32)
            })
            .collect();
        let mut ids_by_best: Vec<(u64, f32)> = self
            .db
            .par_iter()
            .map(|kv| {
                let sentence = kv.1;
                let total_score = sentence
                    .overlaps
                    .iter()
                    .map(|kv| {
                        (commonalities.get(kv.key()).unwrap())
                            / (sentence.original.len() as f32)
                            //+  ( 1 + self.db.get(kv.key()).unwrap().original.len()) as f32
                    })
                    .sum::<f32>();// / ((1 + sentence.words.len()) as f32);
                (kv.0.clone(), total_score)
            })
            .filter(|(k, _)| self.db.get(k).unwrap().filtered_out == false)
            .collect();
        ids_by_best.par_sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        println!(
            "-- best search took {} ms",
            now.elapsed().as_millis()
        );
        ids_by_best
    }


    pub fn sentence_to_set(sentence: &str) -> IntSet<u64> {
        sentence
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .filter(|s| !s.is_empty())
            .map(|s| hash(&s))
            .collect()
    }


    pub fn split_text_to_sentences(
        &self,
        text: &str,
        filtered_out: bool,
        source: &str,
    ) -> Vec<Sentence> {
        text.split(".")
            .filter(|s| s.len() > 100)
            .map(|s| Sentence {
                filtered_out,
                id: hash(s),
                original: Arc::from(s),
                words: Engine::sentence_to_set(&s),
                overlaps: Default::default(),
                source: Arc::from(source),
            })
            .collect()
    }


    pub fn insert_new_sentences(&mut self, sentences: Vec<Sentence>) {
        sentences.par_iter().for_each(|sen| {
            self.db.iter().for_each(|kv| {
                let other = kv.1;
                // If the overlap is already calculated, we can skip this calculation.
                if let Some(_) = other.overlaps.get(&sen.id) {
                    return;
                }
                let overlap = sen.calc_overlap(&other.words);
                sen.set_overlap(other.id, overlap);
                other.set_overlap(sen.id, overlap);
            });
            sentences.iter().for_each(|other| {
                // If the sentences are equal, we can skip this calculation.
                if sen.id == other.id || other.overlaps.get(&sen.id).is_some() {
                    return;
                }
                let overlap = sen.calc_overlap(&other.words);
                sen.set_overlap(other.id, overlap);
                other.set_overlap(sen.id, overlap);
            });
        });
        sentences.into_iter().for_each(|sen| {
            self.db.insert(sen.id, sen);
        });
        println!("done");
    }


    pub fn split_and_insert_sentences(
        &mut self,
        text: &str,
        filtered_out: bool,
        source: &str,
    ) {
        let sentences = self.split_text_to_sentences(text, filtered_out, source);
        self.insert_new_sentences(sentences);
    }


    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let mut file = File::create(path)?;
        let serializable_sentences: Vec<SerializableSentence> = self
            .db
            .iter()
            .map(|s| SerializableSentence::from(s.1))
            .collect();
        let data = rmp_serde::to_vec(&serializable_sentences)?;
        file.write_all(&data)?;
        Ok(())
    }


    pub fn from_file<P: AsRef<Path>>(
        sentences_path: P,
        interner_path: P,
    ) -> Result<Self, Box<dyn Error>> {
        let mut file = File::open(sentences_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        let serializable_sentences: Vec<SerializableSentence> = rmp_serde::from_slice(&data)?;
        let engine = Engine {
            db: serializable_sentences
                .into_iter()
                .map(|s| (s.id, s.into()))
                .collect(),
        };
        Ok(engine)
    }

    pub fn sentence_id_to_text(&self, id: u64) -> Option<Arc<str>> {
        self.db.get(&id).map(|s| s.original.clone())
    }
}

#[test]
fn open_file() {
    let filename = "data/file.txt";
    let text = std::fs::read_to_string(filename).unwrap();
    let mut engine = Engine::new();
    engine.split_and_insert_sentences(&text, false, filename);
    loop {
        let mut query_sentence = String::new();
        std::io::stdin()
            .read_line(&mut query_sentence)
            .expect("Failed to read query");
        let query = Engine::sentence_to_set(&query_sentence);
        let best = engine.best(query);
        for (sentence_id, score) in best.into_iter().take(10) {
            print!("{} {:.6}\n\n", engine.sentence_id_to_text(sentence_id).unwrap().to_string(), score);
        }
    }
}