use rake::*;
use term_macros::*;

const STOPWORDS: &'static [&'static str] = include!("./stopwords.txt");

fn main() {
    tool! {
        args:
            - stopwords_file: Option<String> = None;
            - minimum_score: Option<f64> = None;
            - top_n: Option<usize> = None;
            //- print_score;
        ;

        body: || {

            let stops = stopwords_file.map(std::fs::read_to_string)
                .map(|s| s.ok()
                    .map(|s1| 
                        s1.split("\n").map(|s| s.to_string()).collect::<Vec<_>>()
                    )
                )
                .flatten()
                .unwrap_or_else(|| Vec::from(STOPWORDS).into_iter().map(|s| s.to_string()).collect());

            let mut stopwords = StopWords::new();
            stops.iter().for_each(|s| {
                stopwords.insert(s.to_string());
            });
            let r = Rake::new(stopwords);

            readin!(_wtr, |line: &[u8]| {
                let _ = std::str::from_utf8(line).map(|result| {
                    let keywords = r.run(result);
                    keywords.iter()
                        .take(top_n.unwrap_or_else(|| keywords.len()))
                        .filter(|kw| minimum_score.map(|min| min < kw.score).unwrap_or_else(|| true))
                        .filter(|kw| kw.keyword.len() > 7)
                        .filter(|kw| 
                            !kw.keyword.split(" ").next().unwrap().ends_with("s") &&
                            !kw.keyword.split(" ").filter(|s| s.ends_with("ed")).next().is_some() && 
                            !kw.keyword.split(" ").filter(|s| s.ends_with("es")).next().is_some() && 
                            !kw.keyword.split(" ").filter(|s| s.ends_with("ly")).next().is_some() && 
                            !kw.keyword.split(" ").filter(|s| s.ends_with("ing")).next().is_some()
                        )
                        .for_each(|s| print!("{},", s.keyword));
                });
                print!("\n");
                let _ = std::io::stdout().flush();
            });

        }

    };

}
