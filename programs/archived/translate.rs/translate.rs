//!
//! ```cargo
//! [dependencies]
//! serde_json = "1.0"
//! miniserde = "0.1"
//! smol = "1.2"
//! surf = "2.3"
//! async-std = "*"
//! futures-lite = "*"
//! term_macros = { path = "../../shared/term_macros" }
//! ```
use term_macros::*;
use std::process;

//todo: add more translation backends like rtg, yandex, bing, etc.
use smol::io::AsyncWriteExt;
use smol::io::AsyncBufReadExt;
use smol::{
    channel::{Receiver, Sender}
};
use miniserde::{Deserialize, Serialize};
use std::{
    collections::{HashMap},
};
use surf::Url;
use smol::LocalExecutor;
use futures_lite::future;
use std::convert::TryInto;

fn main() {
    tool! {
        args:
            - from: String = "auto".to_string();
            - to: String;
            - total: usize;
            - keep;
            - sep: String = "\t".to_string();
            - concurrency: usize = 50;
            - chunk_size: usize = 4999;
                ? chunk_size > 5000
                => "Google Translate can only handle fewer than 5000 characters at a time"
        ;
        body: || {
            let local_ex = LocalExecutor::new();

            future::block_on(local_ex.run(async {
                chan!(batch_tx, batch_rx);
                chan!(translation_tx, translation_rx);
                spawn!(local_ex @ batch_tx => 
                    convert_stdin_to_batches(chunk_size, batch_tx, total).await;
                );
                for i in 0..concurrency {
                    spawn!(local_ex @ to, from, batch_rx, batch_tx, translation_tx =>
                        //async_std::task::sleep(std::time::Duration::from_millis((i * 50).try_into().unwrap())).await;
                        spawn_translation_worker(batch_rx, batch_tx, from, to, translation_tx).await;
                    );
                }
                let mut writer = smol::io::BufWriter::new(async_std::io::stdout());
                let mut lines = 0;
                whileok!(translation_rx => msg {
                    match keep {
                        true => {
                            let _ = writer.write_all(msg.translation.replace("\n", "").replace("\t", " ").as_bytes()).await;
                            let _ = writer.write_all(sep.as_bytes()).await;
                            let _ = writer.write_all(msg.original.replace("\n", "").replace("\t", " ").as_bytes()).await;
                            let _ = writer.write_all("\n".as_bytes()).await;
                        },
                        false => {
                            let _ = writer.write_all(msg.translation.replace("\n", "").as_bytes()).await;
                            let _ = writer.write_all("\n".as_bytes()).await;
                        }
                    }
                    lines += 1;
                    let _ = writer.flush().await;
                    if lines >= total {
                        break;
                    }
                    // do not break on writer close, there might still be workers translating
                });
                process::exit(0);
            }));
        }

    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Translation {
    pub original: String,
    pub translation: String,
}

#[derive(Debug)]
pub struct Transliteration(String);

impl From<String> for Transliteration {
    fn from(s: String) -> Self {
        Transliteration(s)
    }
}

#[derive(Debug)]
pub struct Response {
    pub transliteration: Option<Transliteration>,
    pub translations: Vec<Translation>,
}

async fn translate(
    text: &str,
    source_lang: &str,
    target_lang: &str,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let url = Url::parse_with_params(
        "https://translate.google.com/translate_a/single",
        &[
            ("client", "at"),
            ("dt", "t"),
            ("dt", "ld"),
            ("dt", "qca"),
            ("dt", "rm"),
            ("dt", "bd"),
            ("dj", "1"),
            ("hl", target_lang),
            ("ie", "UTF-8"),
            ("oe", "UTF-8"),
            ("inputm", "2"),
            ("otf", "2"),
            ("iid", "1dd3b944-fa62-4b55-b330-74909a99969e"),
        ],
    )
    .unwrap();

    let mut data = HashMap::new();
    data.insert("sl", source_lang);
    data.insert("tl", target_lang);
    data.insert("q", text);

    let body = surf::Body::from_form(&data)?;

    let mut response = surf::post(url)
        .header(
            "Content-Type",
            "application/x-www-form-urlencoded;charset=utf-8",
        )
        .header(
            "User-Agent",
            "AndroidTranslate/5.3.0.RC02.130475354-53000263 5.1 phone TRANSLATE_OPM5_TEST_1",
        )
        .body(body)
        .await?;

    let json: serde_json::Value = response.body_json().await?;
    let _sentences = json
        .as_object()
        .ok_or("Couldn't convert to object")?
        .get("sentences")
        .ok_or("Couldn't find the field 'sentence'")?
        .clone();
    let sentences = _sentences
        .as_array()
        .ok_or("Couldn't convert 'sentences' to an array")?;
    // Each sentence will be a dict with the fields "original" and "translation"
    let translation_vec: Vec<Translation> = sentences
        .iter()
        .rev()
        //.take(sentences.len() - 1)
        .map(|v| v.as_object().unwrap())
        .map(|v| {
            let orig = v.get("orig")?.as_str()?.to_string();
            let trans = v.get("trans")?.as_str()?.to_string();
            Some(Translation {
                original: orig,
                translation: trans,
            })
        })
        .filter(|v| v.is_some())
        .map(|v| v.unwrap())
        .rev()
        .collect();
    // A single object containing the field "translit" will be the transliteration. It comes last, after all of the sentences.
    let transliteration: Option<Transliteration> = {
        let transliteration = then!(sentences
            .into_iter()
            .last(),
            |s|
                s.as_object();
                s.get("translit");
                Some(s.to_string());
                Some(s.replace("\"", ""));
                Some(Transliteration(s))
        );
        transliteration
    };
    Ok(Response {
        transliteration,
        translations: translation_vec,
    })
}

//cargo test chunker --release -- --nocapture
pub async fn spawn_translation_worker<'a>(
    batch_rx: Receiver<String>,
    batch_tx: Sender<String>,
    source_lang: String,
    target_lang: String,
    output: Sender<Translation>,
) {
    whileok!(batch_rx => batch {
        if let Ok(translated) = translate(&batch, &source_lang, &target_lang).await {
            for tl in translated.translations.iter() {
                let _ = output.send(tl.clone()).await;
                async_std::task::sleep(std::time::Duration::from_millis(8.try_into().unwrap())).await;
            }
        } else {
            let _ = batch_tx.send(batch).await;
        }
    });
}

pub async fn convert_stdin_to_batches(chunk_size: usize, batch_tx: Sender<String>, total: usize) {
    let mut stdin = smol::io::BufReader::new(async_std::io::stdin());
    let mut current_chunk = String::new();
    let mut line = String::new();
    for i in 0..total {
        let _ = stdin.read_line(&mut line).await;
        if (current_chunk.len() + line.len() >= chunk_size) || i == (total - 1) {
            let _ = batch_tx.send(current_chunk).await;
            current_chunk = String::new();
        }
        if line.len() > 0 {
            current_chunk.push_str("\n");
            current_chunk.push_str(&line);
        };
        line.clear();
    }
    batch_tx.close();
}