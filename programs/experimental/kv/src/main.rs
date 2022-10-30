use std::io::{Read, Write};

use term_macros::*;
use unqlite::{UnQLite, KV, Transaction, Cursor};
type DatabaseFilename = String;
type KeyName = String;

fn main() {
    tool! {
        args:
            - db: DatabaseFilename;
            - read: bool = true;
            - write: bool = false;
            - get: Option<KeyName> = None;
            - set: Option<KeyName> = None;
            - list;
        ;
        body: || {
            let unqlite = UnQLite::create(db);

            if set.is_some() {
                let set = set.unwrap();
                let mut buf = Vec::new();
                let _ = std::io::stdin().read_to_end(&mut buf);
                unqlite.kv_store(set, buf).unwrap();
                unqlite.commit().unwrap();

            } else if get.is_some() {

                let keyname = get.unwrap();
                let mut lock = std::io::stdout().lock();
                unqlite.kv_fetch(keyname).map(|res| {
                    lock.write_all(&res).unwrap();
                }).unwrap();
                lock.flush().unwrap();

            } else if list {
                let mut entry = unqlite.first();
                loop {
                    if entry.is_none() { break; }

                    let record = entry.expect("valid entry");
                    println!("{:?}", record.key_value());
                    entry = record.next();
                }

            } else if read && !write {

                readin!(wtr, |line: &[u8]| {
                    if let Ok(data) = unqlite.kv_fetch(&line[0..(line.len() - 1)]) {
                        wtr.write_all(&data).unwrap();
                    } else {
                        wtr.write_all(b"Not found\n").unwrap();
                    }
                    wtr.flush().unwrap();
                });

            } else {

                let mut counter = 0;
                readin!(_wtr, |line: &[u8]| {
                    unqlite.kv_store(counter.to_string(), line).unwrap();
                    counter += 1;
                });

                unqlite.commit().unwrap();
            }
        }
    }
}
