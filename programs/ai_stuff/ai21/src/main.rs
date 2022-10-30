use std::time;
use term_macros::*;
use std::io::Read;
// Gets the current number of milliseconds since the Unix epoch
fn now_millis() -> u128 {
    time::SystemTime::now()
        .duration_since(time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .into()
}
// Uses now_millis to choose a random item from an array
fn random_item<T>(items: &[T]) -> &T {
    &items[(now_millis() % items.len() as u128) as usize]
}

fn get_ai21(
    prompt: &str,
    max_tokens: usize,
    temperature: f32,
    stop_sequences: Vec<String>,
    size: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let json_body = format!(
        r#"{{"prompt":{:?},"numResults":1,"maxTokens":{},"topKReturn":0,"temperature":{},"stopSequences": {:?}}}"#,
        prompt, max_tokens, temperature, stop_sequences,
    );
    let keys_str = std::fs::read_to_string(
        std::env::var("AI21_KEYS")
            .unwrap_or_else(|_| "/Users/ckoshka/programming/bash_experiments/showcase/_keys.txt".to_string()),
    )
    .expect("Could not read keys");
    let keys = keys_str.split("\n").collect::<Vec<_>>();
    let key = random_item(keys.as_slice());
    let bearer = format!("Bearer {}", &key); //We need to format the key into a bearer token
                                             //let req = Request::builder().method("POST").uri("https://api.ai21.com/studio/v1/j1-jumbo/complete").header("content-type", "application/json").header("Authorization", &bearer).body(Body::from(json_body.clone()))?;
                                             // ^ was too verbose, went with a smaller library:
    let response = minreq::post(&format!(
        "https://api.ai21.com/studio/v1/j1-{size}/complete"
    ))
    .with_header("content-type", "application/json")
    .with_header("Authorization", &bearer)
    .with_body(json_body)
    .send()?;
    //let text = response["completions"][0]["data"]["text"].as_str().unwrap();
    // we don't have a json parser now, so we just look for an occurrence of "completions":[{"data":{, etc.: and take the next string
    let text = &response
        .as_str()?
        .split(r#"completions":[{"data":{"text":""#)
        .collect::<Vec<_>>()[1];
    let text = text.split("\",\"tokens\":").next().unwrap();
    // Unescape quotes
    let text = text.replace("\\\"", "\"");
    // Unescape new lines
    let text = text.replace("\\n", "\n");
    Ok(text.to_string()) // Return the text
}

fn main() {
    tool! {
        args:
            - size: String = "jumbo".to_string();
            - max: usize = 100;
            - temp: f32 = 0.88;
            - stops: Vec<String> = vec!["305798579338993283fe0o".to_string()];
        ;

        body: || {
            let mut prompt = String::new();
            let _ = std::io::stdin().read_to_string(&mut prompt).expect("Could not read prompt");
            let text = get_ai21(&prompt, max, temp, stops, &size).unwrap();
            println!("{}", text);
        }
    }
}
