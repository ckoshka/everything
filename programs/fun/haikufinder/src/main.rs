mod regexer;
mod summariser;
use regexer::generate_haikus;
mod search;
fn main() {
    let iter = generate_haikus("./data/file.txt").into_iter();
    for sentence in iter.take(10) {
        println!("{}\n", sentence);
    }
}