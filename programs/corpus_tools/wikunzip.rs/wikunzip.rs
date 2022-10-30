//!
//! ```cargo
//! [dependencies]
//! term_macros = { path = "../../shared/term_macros"  }
//! quick-xml = "0.23.0"
//! crossbeam = "0.8.1"
//! parse_wiki_text = "0.1.5"
//! ```

use crossbeam::{
    channel::{bounded, Receiver, Sender}
};
use parse_wiki_text::{Configuration, Node};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::{BufReader};
use std::io::Write;


fn get_stream(sender: Sender<String>) {
    let mut reader = Reader::from_reader(BufReader::new(std::io::stdin()));
    reader.trim_text(true);
    let mut buf = Vec::new();
    loop {
        let res = match reader.read_event(&mut buf) {
            Ok(Event::Text(e)) => sender.send(e.unescape_and_decode(&reader).unwrap()),
            Ok(Event::Eof) => break,
            _ => Ok({}),
        };
        if res.is_err() {
            break;
        }
        buf.clear();
    }
}

fn write_stream_to_file(rcv: Receiver<String>) -> Result<(), String> {
    let mut lock = std::io::BufWriter::new(std::io::stdout().lock());
    loop {
        let line = rcv.recv();
        if line.is_err() {
            continue;
        }
        let line = line.unwrap();

        let result = Configuration::default().parse(&line);
        for node in result.nodes {
            if let Node::Text { value, .. } = node {
                let r = write!(lock, "{}", value);
                if r.is_err() {
                    panic!("Unable to write")
                }
            } else if let Node::Link { end, start, .. } = node {
                let slice = &line[(start)..(end)];
                let stripped = slice.replace("[[", "").replace("]]", "");
                let first = stripped.split("|").last().unwrap();
                if !first.contains("[[") {
                    let _ = write!(lock, "{}", first);
                }
            } else if let Node::ParagraphBreak { .. } = node {
                let _ = write!(lock, "\n");
            } else if let Node::Bold { end, start, .. } = node {
                let slice = &line[(start)..(end)];
                let _ = write!(lock, "{}", slice);
            } else if let Node::Italic { end, start, .. } = node {
                let slice = &line[(start)..(end)];
                let _ = write!(lock, "{}", slice);
            } else if let Node::BoldItalic { end, start, .. } = node {
                let slice = &line[(start)..(end)];
                let _ = write!(lock, "{}", slice);
            }
        }
    } // ew
}

fn main() {
    let (tx, rx) = bounded::<String>(10000);
    std::thread::spawn(move || {
        get_stream(tx);
    });
    let _ = write_stream_to_file(rx);
}