use miniserde::{json, Deserialize};
use std::fmt::Debug;
use std::sync::Arc;

#[derive(Debug, Default, Eq, PartialEq, Hash)]
pub struct Args(Vec<String>);

impl Args {
    pub fn new<S: Into<String>>(args: impl IntoIterator<Item = S>) -> Self {
        Args(args.into_iter().map(|s| s.into()).collect())
    }

    pub fn get_value_of<'a>(&'a self, flag: &str) -> Option<Arc<str>> {
        self.0
            .iter()
            .position(|a| a == flag)
            .map(|p| {
                self.0
                    .iter()
                    .nth(p + 1)
                    .filter(|a| !a.starts_with("--"))
                    .map(|a| Arc::from(a.as_str()))
            })
            .or_else(|| {
                self.0
                    .iter()
                    .find(|a| a.starts_with(flag))
                    .map(|a| a.split("=").nth(1).map(Arc::from))
            })
            .flatten()
    }

    pub fn check_flag(&self, flag: &str) -> bool {
        self.0.iter().find(|a| a.as_str() == flag).is_some()
    }

    pub fn parse_arg<T: Deserialize>(arg: &str) -> Option<T> {
        json::from_str(arg)
            .ok()
            .or_else(|| json::from_str(&format!("{:#?}", arg)).ok())
    }
}

// - parse with eq - DONE
// - parse with space as sep - DONE
// - parse boolean flag - DONE
// - parse into type T
// - macro creating a custom struct which gets the value of specified arguments
// - generalised type that can hold useful information about defaults, errors, expected types, etc.
// so in this case we would want an arc-wrapped series of validation functions

#[derive(Debug)]
pub enum ErrAccum {
    Good,
    Bad(Vec<Box<dyn Debug + Send + Sync + 'static>>),
}

impl ErrAccum {
    pub fn new() -> ErrAccum {
        ErrAccum::Good
    }
    pub fn and<E: Debug + Send + Sync + 'static>(
        self,
        validator_fn: impl FnOnce() -> Result<(), E>,
    ) -> Self {
        match (self, validator_fn()) {
            (ErrAccum::Good, Ok(_)) => ErrAccum::Good,
            (ErrAccum::Good, Err(e)) => ErrAccum::Bad(vec![Box::from(e)]),
            (ErrAccum::Bad(existing_errs), result) => match result {
                Err(e) => ErrAccum::Bad(
                    existing_errs
                        .into_iter()
                        .chain(
                            vec![Box::from(e) as Box<dyn Debug + Send + Sync + 'static>]
                                .into_iter(),
                        )
                        .collect(),
                ),
                Ok(_) => ErrAccum::Bad(existing_errs),
            },
        }
    }
    pub fn merge(other1: ErrAccum, other2: ErrAccum) -> Self {
        let mut errs = vec![];
        let mut matcher = |target: ErrAccum| match target {
            ErrAccum::Bad(other_errs) => errs.extend(other_errs.into_iter()),
            _ => {}
        };
        matcher(other1);
        matcher(other2);
        match errs.len() {
            0 => ErrAccum::Good,
            _ => ErrAccum::Bad(errs),
        }
    }

    pub fn flat_and(self, validator_fn: impl FnOnce() -> ErrAccum) -> Self {
        match (self, validator_fn()) {
            (ErrAccum::Good, ErrAccum::Good) => ErrAccum::Good,
            (e1, e2) => ErrAccum::merge(e1, e2),
        }
    }

    pub fn is_good(&self) -> bool {
        match self {
            ErrAccum::Good => false,
            ErrAccum::Bad(_) => true,
        }
    }

    pub fn is_bad(&self) -> bool {
        !self.is_good()
    }
}

pub type ValidatorFn<T> = Arc<dyn Fn(&T) -> bool + Send + Sync>;

pub struct Validator<E: std::error::Error + Send + Sync> {
    err: Box<dyn Fn() -> E>,
}

#[test]
fn test_err_accum() {
    let num = 17;
    let result = ErrAccum::new()
        .and(|| match num % 2 {
            0 => Ok(()),
            _ => Err("Not divisible by two"),
        })
        .and(|| match num > 20 {
            true => Ok(()),
            false => Err("Needs to be over twenty"),
        });
    println!("{:#?}", result);
}

#[test]
fn test_1() {
    let args = vec!["--arg1=on", "--arg2", "off", "-flag"];
    let parsed = Args::new(args);
    assert_eq!(parsed.get_value_of("--arg1"), Some(Arc::from("on")));
    assert_eq!(parsed.get_value_of("--arg2"), Some(Arc::from("off")));
    assert_eq!(parsed.check_flag("-flag"), true);
}
