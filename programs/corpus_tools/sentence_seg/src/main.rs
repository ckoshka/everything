use term_macros::*;
use unicode_segmentation::UnicodeSegmentation;

fn main() {
    readin!(writer, |lns: &[u8]| {

        let _ = std::str::from_utf8(lns)

            .map(|lns| {
                lns.unicode_sentences().for_each(|line| {
                    writer.write(line.replace("\\n", " ").replace("\n", " ").replace("\\\"", "").as_bytes()).unwrap();
                    writer.write("\n".as_bytes()).unwrap();
                });
            });
    });
}
