use heed::types::*;
use heed::*;
use miniserde::{json, Deserialize, Serialize};
use std::io::Read;
use term_macros::*;

#[derive(Deserialize)]
struct Get {
    key: String,
}

#[derive(Deserialize)]
struct Set {
    key: String,
    value: String,
}

#[derive(Serialize)]
struct Success {
    was_success: bool,
    err: Option<String>,
    val: Option<String>,
}

fn main() {
    tool! {
        // remember to enable --sync_mode
        args:
            - db_filename: String;
            - list;
        ;
        body: || {
            std::fs::create_dir_all(&db_filename).unwrap();
            let env = EnvOpenOptions::new().open(db_filename).unwrap();
            let db: Database<Str, Str> = env.create_database(None).unwrap();

            let mut line = String::new();

            let wtxn = env.write_txn().unwrap();
            if list {
                db.iter(&wtxn).unwrap().for_each(|res| {
                    if let Ok(inner) = res {
                        println!("{}", json::to_string(&inner));
                    }
                });
                return;
            }
            wtxn.commit().unwrap();

            loop {
                let mut wtxn = env.write_txn().unwrap();
                let rtxn = env.read_txn().unwrap();
                line.clear();
                std::io::stdin().read_line(&mut line).unwrap();
                let set: std::result::Result<Set, _> = json::from_str(&line.replace("\n", ""));
                if let Ok(val) = set {
                    db.put(&mut wtxn, &val.key, &val.value).unwrap();
                    println!(
                        "{}",
                        json::to_string(&Success {
                            was_success: true,
                            err: None,
                            val: None
                        })
                    );
                    wtxn.commit().unwrap();
                } else {
                    let get: std::result::Result<Get, _> = json::from_str(&line.replace("\n", ""));
                    if let Ok(val) = get {
                        db.get(&rtxn, &val.key)
                            .unwrap()
                            .map(|res| {
                                println!(
                                    "{}",
                                    json::to_string(&Success {
                                        was_success: true,
                                        err: None,
                                        val: Some(res.to_string())
                                    })
                                )
                            })
                            .unwrap_or_else(|| {
                                println!(
                                    "{}",
                                    json::to_string(&Success {
                                        was_success: false,
                                        err: Some("Key doesn't exist".to_string()),
                                        val: None
                                    })
                                )
                            });
                    } else {
                        println!(
                            "{}",
                            json::to_string(&Success {
                                was_success: false,
                                err: Some("Unable to parse json".to_string()),
                                val: None
                            })
                        );
                        println!("{}", line);
                    }
                }
            }
        }
    }
}
