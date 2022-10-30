use lz4_flex::compress_prepend_size;
use std::io::{Error, ErrorKind};
use std::{collections::HashMap};
use wasm_bindgen::prelude::*;
use miniserde::{json, Serialize};
use std::sync::Arc;

pub struct LanguageDoc {
    pub data: Arc<[u8]>,
    pub compressed_size: usize,
}

impl LanguageDoc {
    pub fn new(data: &[u8]) -> LanguageDoc {
        LanguageDoc { data: Arc::from(data), compressed_size: compress_prepend_size(data).len() }
    }
}

#[wasm_bindgen]
pub struct Detector {
    docs: HashMap<String, LanguageDoc>
}

#[derive(Serialize)]
struct Summary {
    language_name: String,
    likelihood: f64
}

#[wasm_bindgen]
impl Detector {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Detector {
        Detector { docs: HashMap::new() }
    }

    pub fn add_str(&mut self, data: &str, name: String) {
        self.docs.insert(name, LanguageDoc::new(data.as_bytes()));
    }

    pub fn add_bytes(&mut self, data: &[u8], name: String) {
        self.docs.insert(name, LanguageDoc::new(data));
    }

    pub fn detect(&self, text: &str) -> String {
        json::to_string(&get_likelihood_of_lang(&self.docs, text.as_bytes()).into_iter()
            .map(|(language_name, likelihood)| Summary {language_name, likelihood}).collect::<Vec<_>>())
    }
}

fn compression_ratio(
    f1: &[u8],
    f2: &[u8],
    compr_f1_len: usize,
    compr_f2_len: usize,
) -> Result<f64, Error> {
    if f1.len() + f2.len() == 0 {
        return Err(Error::new(ErrorKind::InvalidData, "File is empty"));
    }
    let compressed_together =
        compress_prepend_size(&f1.iter().chain(f2.iter()).copied().collect::<Vec<u8>>());

    let actual = (compressed_together.len()) as f64 / (compr_f1_len + compr_f2_len) as f64;
    Ok(actual)
}

pub fn get_likelihood_of_lang<'a>(
    docs: &'a HashMap<String, LanguageDoc>, // assumes non-empty
    input_bytes: &[u8]
) -> Vec<(String, f64)> {
    let compressed_length = compress_prepend_size(input_bytes).len();
    let average_length = docs.iter().map(|d| d.1.compressed_size as f64).sum::<f64>();

    let compression_ratios = docs
        .iter()
        .map(|(lang, doc)| {
            let f1 = &doc.data;
            let compr_f1_len = doc.compressed_size;
            let f2 = input_bytes;
            let result = compression_ratio(&f1, f2, compr_f1_len, compressed_length).unwrap();
            (result, lang)
        })
        .collect::<Vec<_>>();

    let mut adjusted_ratios: Vec<_> = compression_ratios
        .iter()
        .map(|(confidence, lang)| ((1.0 - confidence), lang))
        .map(|(confidence, lang)| {
            let length_ratio = docs.get(lang.as_str()).unwrap().compressed_size as f64 / average_length;
            let confidence = confidence * length_ratio;
            (lang.to_string(), confidence)
        })
        .collect();

    adjusted_ratios.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    adjusted_ratios
}