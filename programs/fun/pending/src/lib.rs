use crossbeam_utils::thread;
pub mod partial;
pub mod partial2;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::{Arc, Mutex};

pub struct WriteTask<T> {
    pub func: Arc<dyn Fn(&mut T) + Send + Sync>,
    pub repr: Arc<str>,
    pub before: Option<Arc<str>>,
    pub after: Option<Arc<str>>,
}

impl<T> std::fmt::Debug for WriteTask<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WriteTask({r})", r = self.repr)
    }
}

pub struct ReadTask<T, Ret> {
    pub func: Arc<dyn FnMut(&mut T) -> Ret>,
}

impl<T, Ret> std::fmt::Debug for ReadTask<T, Ret> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ReadTask(FnMut(String) -> String)")
    }
}

#[derive(Clone)]
pub struct Defer<T>
where
    T: Serialize + DeserializeOwned + std::fmt::Debug,
{
    inner: Arc<Mutex<T>>,
    pub history: Arc<Mutex<Vec<WriteTask<T>>>>,
    pub deferred_writes: Arc<Mutex<Vec<WriteTask<T>>>>,
}

impl<T> std::fmt::Debug for Defer<T>
where
    T: Serialize + DeserializeOwned + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{i:?}", i = self.inner.lock().unwrap())
    }
}

impl<T> Defer<T>
where
    T: Serialize + DeserializeOwned + std::fmt::Debug + Send + Sync,
{
    pub fn new(initial: T) -> Defer<T> {
        Defer {
            inner: Arc::from(Mutex::new(initial)),
            history: Arc::new(Mutex::new(Vec::new())),
            deferred_writes: Arc::new(Mutex::new(Vec::new())),
        }
    }
    pub fn queue_write(
        &self,
        func: impl Fn(&mut T) + 'static + Send + Sync,
        repr: &str,
        push_to_start: bool,
    ) {
        let mut d = self.deferred_writes.lock().unwrap();
        let wt = WriteTask {
            func: Arc::new(func),
            repr: Arc::from(repr),
            before: None,
            after: None,
        };
        if push_to_start {
            d.insert(0, wt);
        } else {
            d.push(wt);
        }
    }
    pub fn apply_all_writes<'a>(&'a self) {
        let mut data = self.inner.lock().unwrap();
        let mut d = self.deferred_writes.lock().unwrap();
        let mut h = self.history.lock().unwrap();
        for mut wt in d.drain(0..) {
            wt.before = Some(Arc::from(serde_json::to_string(&*data).unwrap().as_str()));
            (wt.func)(&mut data);
            wt.after = Some(Arc::from(serde_json::to_string(&*data).unwrap().as_str()));
            h.push(wt);
        }
    }
    pub fn get_state_before<'a>(&'a self, wt_repr: &str) -> Result<T, Box<dyn std::error::Error>> {
        let h = self.history.lock().unwrap();
        for wt in h.iter() {
            if wt.repr.contains(wt_repr) {
                let data: T = serde_json::from_str(&wt.before.clone().unwrap()).unwrap();
                return Ok(data);
            }
        }
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("No state found for {:?}", wt_repr),
        )))
    }
    pub fn get_position_of_history(
        &self,
        wt_repr: &str,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let mut h = self.history.lock().unwrap();
        let mut pos = 0;
        for wt in h.iter_mut() {
            if wt.repr.contains(wt_repr) {
                return Ok(pos);
            }
            pos += 1;
        }
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("No state found for {:?}", wt_repr),
        )))
    }
    pub fn get_position_of_pending(&self, wt_repr: &str) -> usize {
        let mut h = self.deferred_writes.lock().unwrap();
        let mut pos = 0;
        for wt in h.iter_mut() {
            if wt.repr.contains(wt_repr) {
                return pos;
            }
            pos += 1;
        }
        pos
    }
    pub fn remove_from_pending(&self, wt_repr: &str) {
        let mut h = self.deferred_writes.lock().unwrap();
        let pos = self.get_position_of_pending(wt_repr);
        for wt in h.iter_mut() {
            if wt.repr.contains(wt_repr) {
                h.remove(pos);
                return;
            }
        }
    }
    pub fn remove_last_pending(&self) {
        let mut h = self.deferred_writes.lock().unwrap();
        let h_len = h.len();
        if h_len > 0 {
            h.remove(h_len - 1);
        }
    }
    // Accepts wt_repr. Returns T as if the historical write task had never been applied. All changes occurring after this are applied sequentially.
    pub fn remove_from_history(&self, wt_repr: &str) {
        let pos = self.get_position_of_history(wt_repr).unwrap();
        let mut h = self.history.lock().unwrap();
        let mut pending_tasks = self
            .deferred_writes
            .lock()
            .unwrap()
            .drain(0..)
            .collect::<Vec<_>>();
        let mut d = self.deferred_writes.lock().unwrap();
        let occurred_after = h.drain((pos + 1)..).collect::<Vec<_>>();
        let _wt = h.pop().unwrap(); // Pop the write that we are removing
        for wt in occurred_after.into_iter() {
            d.push(wt);
        }
        let mut inner = self.inner.lock().unwrap();
        *inner = serde_json::from_str(&_wt.before.clone().unwrap()).unwrap();
        drop(inner);
        drop(d);
        drop(h);
        self.apply_all_writes();
        let mut d = self.deferred_writes.lock().unwrap();
        d.append(&mut pending_tasks);
    }
    pub fn swap_with(
        &self,
        wt_repr: &str,
        func: impl Fn(&mut T) + 'static + Send + Sync,
        new_repr: &str,
    ) {
        let pos = self.get_position_of_history(wt_repr).unwrap();
        let mut h = self.history.lock().unwrap();
        let mut pending_tasks = self
            .deferred_writes
            .lock()
            .unwrap()
            .drain(0..)
            .collect::<Vec<_>>();
        let mut occurred_after = h.drain((pos + 1)..).collect::<Vec<_>>();
        let _wt = h.pop().unwrap(); // Pop the write that we are removing
        for wt in occurred_after.into_iter() {
            self.deferred_writes.lock().unwrap().push(wt);
        }
        let mut wt = WriteTask {
            func: Arc::new(func),
            repr: Arc::from(new_repr),
            before: None,
            after: None,
        };
        let mut inner = self.inner.lock().unwrap();
        *inner = serde_json::from_str(&_wt.before.clone().unwrap()).unwrap();
        self.deferred_writes.lock().unwrap().insert(0, wt);
        drop(inner);
        drop(h);
        self.apply_all_writes();
        let mut d = self.deferred_writes.lock().unwrap();
        d.append(&mut pending_tasks);
    }
}

macro_rules! deferred {
    ($target:expr) => {
        Defer::new($target)
    };
}

macro_rules! defer {
    ($target:ident, $closure:expr) => {
        $target.queue_write(|$target| $closure, stringify!($closure), false)
    };
}

macro_rules! apply {
    ($target:ident) => {
        $target.apply_all_writes()
    };
}

macro_rules! remove {
    ($target:ident, $closure:expr) => {
        $target.remove_from_history(stringify!($closure))
    };
}

macro_rules! swap {
    ($target:ident, $repr:expr, $closure:expr) => {
        $target.swap_with(stringify!($repr), |$target| $closure, stringify!($closure))
    };
}

//remove_from_pending
macro_rules! undo {
    ($target:ident, $closure:expr) => {
        $target.remove_from_pending(stringify!($closure))
    };
}

//remove_last_pending
macro_rules! undo_last {
    ($target:ident) => {
        $target.remove_last_pending()
    };
}

macro_rules! revert_to {
    ($target:ident, $closure:expr) => {
        $target.get_state_before(stringify!($closure))
    };
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Plate {
    dumplings: usize,
    review: String,
    extras: Vec<String>,
}

#[test]
fn test() {
    // Let's visit a restaurant where the order of time is a little bit... malleable.
    let plate = deferred!(Plate {
        dumplings: 15,
        review: "".to_string(),
        extras: vec!["sweet&sour fish".to_string(), "soy sauce".to_string()],
    });
    // Looks good! Let's dig in.
    defer!(plate, plate.dumplings -= 5);
    // Sumptuous.
    defer!(plate, plate.review = "So far okayish I guess".to_string());
    // And we'll leave a good review.
    // But we forgot to include our signature:
    defer!(plate, plate.review.push_str(" ~~ review by ckoshka"));
    // And why not convert everything to uppercase, to show our enthusiasm?
    defer!(plate, plate.review = plate.review.to_uppercase());
    // But wait, what if we're really indecisive and hungry?

    thread::scope(|s| {
        let plate_copy = plate.clone();
        s.spawn(move |_| {
            for _ in 1..100 {
                defer!(
                    plate_copy,
                    plate_copy.extras.append(&mut vec![
                        "steamed pork buns".to_string(),
                        "chow mein".to_string()
                    ])
                );
            }
            // Oh dear, that's a bit too much. Let's undo that.
        });
        let plate_copy = plate.clone();
        s.spawn(move |_| {
            for _ in 1..99 {
                undo_last!(plate_copy)
            }
            // That should be enough to sate our appetite.
        });
    })
    .unwrap();

    // Oh, and by the way – none of this actually happened.
    println!(
        "Before applying, the underlying struct is unchanged: {:#?}",
        plate
    );
    // Plate { dumplings: 15, review: "", extras: ["sweet&sour fish", "soy sauce"] }
    apply!(plate);
    // Not until after we apply the changes:
    println!("Applied: {:#?}", plate);
    // Plate Plate { dumplings: 10, review: "SO FAR OKAYISH I GUESS ~~ REVIEW BY CKOSHKA", extras: ["sweet&sour fish", "soy sauce", "steamed pork buns", "chow mein"] }

    // What if we wanted to edit history a little?
    // Let's uneat 5 dumplings.
    remove!(plate, plate.dumplings -= 5);
    // And let's increase the positivity of our review.
    swap!(
        plate,
        plate.review = "So far okayish I guess".to_string(),
        plate.review = "Absolutely divine. 5 stars.".to_string()
    );
    println!("After the swap: {:#?}", plate);
    // Plate { dumplings: 15, review: "ABSOLUTELY DIVINE. 5 STARS. ~~ REVIEW BY CKOSHKA", extras: ["sweet&sour fish", "soy sauce", "steamed pork buns", "chow mein"] }
}
