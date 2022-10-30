pub use owo_colors;
pub use linereader;
pub use log;
pub use miniserde;
use miniserde::*;
use std::fmt::Debug;
use std::time::SystemTime;
//mod lib2;
// TODO:
//* feature-gate everything
//* implementation that supports interactive prompts
//* implementation that supports a json file as an untagged config argument
//* impl for yaml files
//* impl for json-rpc server?
//* impl for outputting an openapi spec
//* would ideally like it to be run as a wasm app, so no annoying binary crosscomp

// TODO-META:
//* instead of ambiently absorbing arguments from std::env, take them explicitly from the macro
//* impl that allows for testing different scenarios
//* ideally the "body" section would just be a function that the arguments get routed to?
//* the line -> wtr closure seems generalisable to other formats quite easily. so it seems like it'd be a good idea to explicitly have "setup/config", followed by
//* create a "mock" environment object.
//* actually allow for pattern-matching against multiple possible input-states
//* but provide a convenience option for the default case of just one script
//* meta-parameters like --buffer-size
//* find more recurring idioms in commandline apps, macroify them

pub fn random(within_range: u128) -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        % within_range
}

pub const KAOMOJI: &[&'static str] = &["(╥_╥)", "(个_个)", "(╥﹏╥)", "(T_T)"];

pub trait Tap: Debug {
    fn tap(self, msg: &str) -> Self
    where
        Self: Sized,
    {
        println!("{msg} {:#?}", self);
        return self;
    }
}

impl<T> Tap for T where T: Debug {}

pub fn find_arg<T>(argname: &str) -> Option<T>
where
    T: Deserialize,
{
    std::env::args()
        .position(|a| a == argname)
        .map(|p| {
            std::env::args()
                .skip(p + 1)
                .take_while(|el| !el.starts_with("-")) // can't be - otherwise it wouldn't work for negative numbers?
                .collect::<Vec<_>>()
                .join(" ")
        })
        .map(|s| match s.len() {
            0 => None,
            _ => Some(s),
        })
        .flatten()
        .map(|s| {
            json::from_str(&s)
                .ok()
                .or_else(|| json::from_str(&format!("{:#?}", s)).ok())
        })
        .flatten()
        .flatten()
        .or_else(|| {
            if argname.len() == 2 {
                return None;
            } else {
                let first_char = argname.chars().filter(|c| c != &'-').next().unwrap();
                let shortened_arg = format!("-{}", first_char);
                find_arg(&shortened_arg)
            }
        })
}

pub fn find_flag(flagname: &str) -> bool {
    let arg = std::env::args().position(|a| a == flagname);
    match arg {
        None => false,
        Some(_) => true,
    }
}

#[macro_export]
macro_rules! tool {
    (args: $(- $identname:ident $(: $typeof:ty )? $( = $default:expr)?; $( ? $cond:expr )? $( => $lit:literal )? )+; body: $main_body:expr) => {
        use std::convert::TryInto;
        let case1 = (|| {
            if (find_flag("-h") || find_flag("-H") || find_flag("--help") || find_flag("help") || find_flag("-help") ) {
                return Err("Here's some help!".to_string());
            }
            $(
                let argflag = format!("--{}", stringify!($identname));
                tool!(- $identname argflag $(: $typeof )? $( = $default)?; $( ? $cond )? $( => $lit )?);
            )+;
            if 1 != 1 {
                return Err("The universe is broken".to_string());
            }

            ($main_body)();

            Ok(())
        })();

        match case1 {
            Err(e1) => {
                use term_macros::owo_colors::OwoColorize;
                eprintln!("{}", e1);
                eprintln!("⚘⚘⚘ Help: ⚘⚘⚘\n");
                $(
                    eprintln!("--{} ⚘", stringify!($identname).magenta());
                    $(
                        eprintln!("  type: {}", stringify!($typeof).bold().italic().magenta());
                    )?
                    $(
                        eprintln!("  default: {}", stringify!($default).cyan().bold());
                    )?
                    $(
                        eprintln!("  fails if: {}", stringify!($cond).green());
                    )?
                    $(
                        eprintln!("      because: {}", $lit.green());
                    )?
                    eprintln!("");
                )+;
            },
            Ok(_) => {
                return;
            }
            _ => {}
        };
    };
    (- $argname:ident $argflag:ident : $typ:ty; $($remaining_tokens:tt)*) => {
        let $argname = find_arg::<$typ>(&$argflag).ok_or_else(|| format!("{} is a required argument", $argflag))?;
        tool!($($remaining_tokens)*);
    };
    (- $argname:ident $argflag:ident : $typ:ty = $default:expr; $($remaining_tokens:tt)*) => {
        let $argname = find_arg::<$typ>(&$argflag).unwrap_or_else(|| $default);
        tool!($($remaining_tokens)*);
    };
    (- $argname:ident $argflag:ident; $($remaining_tokens:tt)*) => {
        let $argname = find_flag(&$argflag);
        tool!($($remaining_tokens)*);
    };
    (? $evalfn:expr => $err:literal) => {
        if ($evalfn) {
            return Err($err.to_string());
        }
    };
    (? $evalfn:expr) => {
        if ($evalfn) {
            return Err(stringify!($evalfn).to_string());
        }
    };
    () => {};
}

// todo: create another macro using mpsc sync and linereader for those that don't require transformations of the original string, i.e ones that can just operate on byte-slices.

/*#[macro_export]
macro_rules! readin {
    ($tx:ident, $closure:expr) => {
        use std::io::prelude::*;
        use corosensei::{Coroutine, CoroutineResult, Yielder};
        //use linereader::LineReader;
        //use std::thread;
        //use std::sync::mpsc::sync_channel;
        let mut reader = std::io::BufReader::new(std::io::stdin());
        let mut writer = std::io::BufWriter::new(std::io::stdout().lock());
        //let ($tx, rx) = sync_channel(1000);

        let mut coroutine = Coroutine::new(move |$tx, _| {
            let mut line = String::new();
            while reader.read_line(&mut line)? > 0 {
                ($closure)(line.as_str());
                line.clear();
            };
            Ok(()) as Result<(), std::io::Error>
        });

        loop {
            match coroutine.resume(0) {
                CoroutineResult::Yield(msg) => {
                    let res = writer.write_all(&msg.as_bytes());
                    let _ = writer.write_all(b"\n");
                    if res.is_err() {
                        break;
                    }
                },
                CoroutineResult::Return(_) => break,
            };
        }
    }
}*/

#[macro_export]
macro_rules! filter_in {
    ($closure:expr) => {
        use linereader::LineReader;
        use std::io::prelude::*;
        let sync_mode = find_arg::<bool>("--sync_mode").unwrap_or_else(|| false);
        if sync_mode {
            let stdin = std::io::stdin();
            let mut lock = stdin.lock();
            let mut s = String::new();
            while let Ok(_) = lock.read_line(&mut s) {
                if ($closure)(s.as_bytes()) {
                    println!("{}", s);
                }
            };
            return;
        }



        let mut reader = LineReader::new(std::io::stdin());
        let mut writer = std::io::BufWriter::new(std::io::stdout());

        let _ = reader.for_each(|line| {
            if ($closure)(line) {
                let res = writer.write_all(line);
                //let _ = writer.write_all(b"\n");
                if res.is_err() {
                    return Ok(false);
                }
            }
            Ok(true)
        });
    };
}

#[macro_export]
macro_rules! readin {
    ($writer:ident, $closure:expr) => {
        use linereader::LineReader;
        use std::io::prelude::*;

        let sync_mode = find_arg::<bool>("--sync_mode").unwrap_or_else(|| false);

        if sync_mode {
            let stdin = std::io::stdin();
            let mut lock = stdin.lock();
            let mut s = String::new();
            let stdout = std::io::stdout();
            let mut $writer = stdout.lock();
            while let Ok(_) = lock.read_line(&mut s) {
                ($closure)(s.as_bytes());
                let _ = $writer.flush();
            };
            return;
        }

        let mut reader = LineReader::new(std::io::stdin());
        let mut $writer = std::io::BufWriter::new(std::io::stdout());

        let _ = reader.for_each(|line| {
            ($closure)(line);
            Ok(true)
        });
    };
}

// give ownership of the line? to avoid repeated allocation? or use a stackful generator to allow for yielding unowned data?

#[macro_export]
macro_rules! then {
    ($ex:expr, |$arg:ident| $($closure:expr);+) => {
        $ex
        $(
            .and_then(|$arg| $closure)
        )+
    }
}

#[macro_export]
macro_rules! mmap {
    ($filename:ident) => {
        unsafe {
            use memmap::MmapOptions;
            std::fs::File::open(&$filename).ok().and_then(|m| MmapOptions::new().map(&m).ok()).unwrap()
        }
    }
}

#[macro_export]
macro_rules! mmap_str {
    ($filename:ident) => {
        let mmap_data = unsafe {
            use memmap::MmapOptions;
            std::fs::File::open(&$filename).ok().and_then(|m| MmapOptions::new().map(&m).ok()).unwrap()
        };
        std::str::from_utf8(&mmap_data[..]).unwrap()
    }
}


#[macro_export]
macro_rules! mkfile {
    ($filename:ident) => {
        std::io::BufWriter::new(std::fs::File::create($filename).unwrap())
    }
}

#[macro_export]
macro_rules! chan {
    ($txname:ident, $rxname:ident) => {
        let ($txname, $rxname) = smol::channel::unbounded();
    };
}

#[macro_export]
macro_rules! spawn {
    ($executor:ident @ $($cloned_obj:ident),* => $($task:tt)+) => {
        {
            $(
                let $cloned_obj = $cloned_obj.clone();
            )*
            $executor.spawn(async move{
                    $($task)+
            }).detach();
        }
    }
}

#[macro_export]
macro_rules! whileok {
    ($tx:ident => $msgname:ident { $($task:tt)+ }) => {
        while let Ok($msgname) = $tx.recv().await { $($task)+ }
    }
}

#[macro_export]
macro_rules! senditer {
    ($iter:expr => $txname:ident => $rxname:ident) => {
        let ($txname, $rxname) = smol::channel::unbounded();
        let __tx_cloned = $txname.clone();
        spawn!( =>
            for __item in $iter {
                let _ = __tx_cloned.send(__item).await;
            }
        );
    }
}

#[macro_export]
macro_rules! pipeline {
    ($($num_workers:literal * $source_channel:ident => $closure:expr => $txname:ident => $rxname:ident),+) => {
        $(
            let ($txname, $rxname) = smol::channel::unbounded();
            for _ in (0..($num_workers)) {
                let __rx = $source_channel.clone();
                let new_tx = $txname.clone();
                smol::spawn(async move {
                    loop {
                        while let Ok(item) = __rx.recv().await {
                            if let Some(result) = ($closure)(item).await {
                                let _ = new_tx.send(result).await;
                            }
                        }
                    }
                });
            }
        )+
    };
}

#[macro_export]
macro_rules! pipe {
    ($start:expr => $( $fn_name:ident $($arg:expr)* ) => +) => {
        {
            let x = $start;
            $(
                let x = $fn_name($($arg),*, x);
            )*
            x
        }
    }
}

#[macro_export]
macro_rules! pipemap {
    ($start:expr => $( $fn_name:ident $($arg:expr)* ) => +) => {
        { 
            $start$(
                .map(|x| $fn_name($($arg),*, x)).flatten()
            )*
        }
    }
}
// also do currying? double 4 _ 6
#[test]
fn pipe_test() {
    let double = |x: i32, y: i32| x * 2 + y;
    let halve = |min: i32, x: i32| if x > min { None } else { Some(x / 2) };
    pipe!(
        5 
        => double 4
    );
    pipemap!(Some(20) => halve 5 => halve 3);
}

#[macro_export]
macro_rules! open {
    ($filename:expr) => {
        {
            let mut data = String::new();
            std::fs::File::open($filename)
                .expect(&format!("Could not open {}", $filename))
                .read_to_string(&mut data)
                .expect(&format!("Could not read {} to string", $filename));
            data
        }
    }
}

