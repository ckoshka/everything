use std::io::{Read, Write};

use jammdb::DB;
use term_macros::*;

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
            - bucket_name: String = "data".to_string();
        ;
        body: || {
            let db = DB::open(&db).unwrap();
            let tx = db.tx(true).unwrap();

            let bucket = tx.get_or_create_bucket(&bucket_name).unwrap();
            
            if set.is_some() {
                let set = set.unwrap();
                let mut buf = Vec::new();
                let _ = std::io::stdin().read_to_end(&mut buf);
                bucket.put(set, buf).unwrap();

            } else if get.is_some() {

                let keyname = get.unwrap();
                let mut lock = std::io::stdout().lock();
                bucket.get(keyname).map(|res| {
                    lock.write_all(res.kv().value()).unwrap();
                }).unwrap();
                lock.flush().unwrap();

            } else if list {

                let mut lock = std::io::stdout().lock();
                bucket.kv_pairs().for_each(|kv| {
                    let _ = lock.write_all(kv.key());
                });
                lock.flush().unwrap();

            } else if read && !write {

                readin!(wtr, |line: &[u8]| {
                    if let Some(data) = bucket.get(&line[0..(line.len() - 1)]) {
                        wtr.write_all(data.kv().value()).unwrap();
                    } else {
                        wtr.write_all(b"Not found\n").unwrap();
                    }
                    wtr.flush().unwrap();
                });

            } else {
                let mut counter = 0;
                readin!(_wtr, |line: &[u8]| {
                    bucket.put(counter.to_string(), line).unwrap();
                    counter += 1;
                });

            }

            tx.commit().unwrap();
        }
    }
}
