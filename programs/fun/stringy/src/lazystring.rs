use std::{sync::{Mutex, Arc}, ops::{Deref, DerefMut}, fmt::{Formatter, Display}};
use rayon::prelude::*;
#[derive(Clone)]
pub struct StringOp {
    pub name: String,
    pub op: Arc<dyn Fn(String) -> String>,
}

impl std::fmt::Debug for StringOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.name)
    }
}

// A string wrapper that lazily stores methods called on it, and only executes them when the string is actually used or dereferenced.
#[derive(Clone, Debug)]
pub struct LazyString {
    pub inner: String,
    pub history: Arc<Mutex<Vec<StringOp>>>,
    pub pending: Arc<Mutex<Vec<StringOp>>>,
}

impl Display for LazyString {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.inner)
    }
}

impl PartialEq for LazyString {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl LazyString {
    fn new(inner: String) -> Self {
        LazyString {
            inner,
            history: Arc::new(Mutex::new(Vec::new())),
            pending: Arc::new(Mutex::new(Vec::new())),
        }
    }
    fn new_with_history(inner: String, history: Arc<Mutex<Vec<StringOp>>>) -> Self {
        LazyString {
            inner,
            history: history,
            pending: Arc::new(Mutex::new(Vec::new())),
        }
    }
    fn add(&self, name: &str, func: impl Fn(String) -> String + 'static) {
        self.pending.lock().unwrap().push(StringOp {
            name: name.to_string(),
            op: Arc::new(func),
        });
    }
    fn undo(&self) {
        let mut pending = self.pending.lock().unwrap();
        if pending.len() > 0 {
            let op = pending.pop().unwrap();
        }
    }
    fn apply(&self) -> LazyString {
        let mut inner = self.inner.clone();
        for op in self.pending.lock().unwrap().iter() {
            inner = (op.op)(inner);
        }
        LazyString::new_with_history(inner, Arc::clone(&self.history))
    }
    fn exec(self) -> Self {
        self.apply()
    }
}

impl Deref for LazyString {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<String> for LazyString {
    fn from(inner: String) -> Self {
        LazyString::new(inner)
    }
}

#[test]
fn test_lazy_string() {
    let s = LazyString::new("Hello222".to_string());
    s.add("to_upper", |s| s.to_uppercase());
    s.add("remove_numbers", |s| s.par_chars().filter(|c| !c.is_numeric()).collect());
    println!("Originally: {}", *s);
    println!("When applied: {}", s.apply());
    s.undo();
    println!("After undo: {}", s.apply());
    let s = s.exec();
    println!("After exec: {}", *s);
}
