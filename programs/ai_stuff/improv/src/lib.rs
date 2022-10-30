use cmd_lib::*;
use miniserde::Deserialize;
use miniserde::Serialize;
use std::collections::HashMap;
use std::io::Write;
use std::marker::PhantomData;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;

use std::time;
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
    max_tokens: i32,
    temperature: f32,
    stop_sequences: Vec<String>,
    size: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {

    
    let json_body = format!(
        r#"{{"prompt":{:?},"numResults":1,"maxTokens":{},"topKReturn":0,"temperature":{},"stopSequences": {:?}}}"#,
        prompt, max_tokens, temperature, stop_sequences,
    );
    let keys_str = std::fs::read_to_string("_keys.txt").expect("Could not read keys");
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

pub struct Example {
    keyword_args: HashMap<Arc<str>, Arc<str>>,
    expected_output: Arc<str>,
}

impl Example {
    pub fn new<T: Serialize>(expected_output: T) -> Self {
        Example {
            keyword_args: HashMap::new(),
            expected_output: Arc::from(miniserde::json::to_string(&expected_output)),
        }
    }
    pub fn add_kw(self, key: &str, val: &str) -> Self {
        let new_map = self
            .keyword_args
            .into_iter()
            .chain(vec![(Arc::from(key), Arc::from(val))].into_iter())
            .collect();
        Self {
            keyword_args: new_map,
            expected_output: self.expected_output.clone(),
        }
    }
    pub fn to_string(&self, function_name: &str) -> String {
        let template = |arg_str: &str, output: &str| {
            format!(
                "{}({})\n# Returns: \"\"\"{}\"\"\"",
                function_name, arg_str, output
            )
        };
        let arg_str = self
            .keyword_args
            .iter()
            .fold(String::new(), |acc, (key, val)| {
                if acc.len() > 0 {
                    format!("{acc}, {key}=\"{val}\"")
                } else {
                    format!("{key}=\"{val}\"")
                }
            });
        template(&arg_str, &self.expected_output)
    }
}

pub struct PromptBuilder<T: Deserialize> {
    function_name: Arc<str>,
    examples: Vec<Example>,
    _parsed_into: PhantomData<T>,
}

impl<T> PromptBuilder<T>
where
    T: Deserialize,
{
    pub fn new(name: &str, examples: Option<Vec<Example>>) -> Self {
        Self {
            function_name: Arc::from(name),
            examples: examples.unwrap_or_else(|| Vec::new()),
            _parsed_into: PhantomData,
        }
    }
    pub fn add_example(mut self, ex: Example) -> Self {
        self.examples.push(ex);
        self
    }
    pub fn to_string(&self) -> String {
        self.examples.iter().fold(String::new(), |acc, ex| {
            format!("{acc}{}\n", ex.to_string(&self.function_name))
        })
    }
    pub fn parse(&self, response_str: &str) -> Result<T, miniserde::Error> {
        miniserde::json::from_str(response_str)
    }
    pub fn run(&self, arg_str: &str) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
        let template = format!(
            "{}\n{}({})\n# Returns: \"\"\"",
            self.to_string(),
            self.function_name,
            arg_str
        );

        let result = get_ai21(
            &template,
            150,
            0.88,
            vec!["\n".to_string(), "\"\"\"".to_string()],
            "jumbo",
        )?;

        Ok(self.parse(&result)?)
    }
}

#[test]
fn test_builder() {
    #[derive(Deserialize, Serialize, Debug)]
    struct Name {
        name: String,
    }

    let builder = PromptBuilder::<Name>::new("generate_name", None)
        .add_example(
            Example::new(Name {
                name: "Tahir Lamija Salihović".to_string(),
            })
            .add_kw("ethnicity", "Bosnian")
            .add_kw("length", "3")
            .add_kw("rareness", "medium"),
        )
        .add_example(
            Example::new(Name {
                name: "John Thomas Watson Banks".to_string(),
            })
            .add_kw("ethnicity", "American")
            .add_kw("length", "4")
            .add_kw("rareness", "common"),
        );

    println!(
        "{:#?}",
        builder
            .run(r#"ethnicity="Sudanese", length=2, rareness="rare""#)
            .unwrap()
    )
}
// then extra args
