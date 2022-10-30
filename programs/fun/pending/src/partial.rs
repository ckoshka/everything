use paste::paste;
use std::{
    collections::{HashMap, VecDeque},
    marker::PhantomData,
    sync::Arc,
};

use serde::{
    de::{DeserializeOwned, Error},
    Deserialize, Serialize,
};
use serde_json::{from_str, json, to_string, Map, Value};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
// Allows for the initialisation or arbitrary serialisable structs in any order.
struct Partial {
    val: HashMap<String, Value>,
}

impl Partial {
    pub fn new() -> Partial {
        Partial {
            val: HashMap::new(),
        }
    }
    pub fn from_template<T>(t: &T) -> Result<Partial, Box<dyn std::error::Error>>
    where
        T: Serialize + DeserializeOwned,
    {
        let mut map = HashMap::default();
        let v: Value = json!(&t);
        // Iterate over the keys and insert them into the HashMap along with their string-serialised values
        for (k, v) in v
            .as_object()
            .ok_or("Unable to convert the value into an object")?
            .iter()
        {
            map.insert(k.to_string(), v.clone());
        }
        Ok(Partial { val: map })
    }
    pub fn insert<V: Serialize + DeserializeOwned>(
        &mut self,
        key: &str,
        value: V,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.val.insert(key.into(), json!(&value));
        Ok(())
    }
    pub fn remove<V: Serialize + DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.val.remove(key).ok_or("No value found for the key")?;
        Ok(())
    }
    pub fn build<T>(&self) -> Result<T, Box<dyn std::error::Error>>
    where
        T: DeserializeOwned + Serialize,
    {
        let serialised = to_string(&self.val)?;
        let target: T = from_str(&serialised)?;
        Ok(target)
    }
}

type Checker<Val> = Arc<dyn Fn(&Val) -> bool + Send + Sync + 'static>;

struct KeyVal<Val> {
    val: Option<Val>,
    check: Option<Checker<Val>>,
}

impl<Val> KeyVal<Val> {
    pub fn new_with_defaults(val: Val, check: Option<Checker<Val>>) -> Self {
        KeyVal {
            val: Some(val),
            check,
        }
    }
    pub fn new_with_blank(check: Option<Checker<Val>>) -> Self {
        KeyVal { val: None, check }
    }
    pub fn set(&mut self, val: Val) -> Result<Val, String> {
        if let Some(checker) = &self.check {
            if !checker(&val) {
                return Err("Invalid value".into());
            }
        }
        Ok(val)
    }
}
// Usage examples for partial! //
///
///```rust
/// struct Person {
///     name: String,
///     age: u8,
///     is_teacher: bool,
///     keycode: Option<String>,
///     homework: Option<Vec<Homework>>,
/// }
/// let name_validator = |name: String| val.len() > 0 && val.chars().filter(|c| c.is_whitespace()).count() >= 3;
/// let teacher = partial_fixed!(
///     Person,
///     name: String = _ => name_validator,
///     age: u8 = _ => v > 18,
///     is_teacher: bool = true,
///     locker_code: Option<String> = None,
///     homework: Option<Vec<Homework>> = None
/// );
/// // Attempts to change the default values will result in an error at compile-time (no method for it).
/// let student = partial!(
///     name: _ => name_validator,
///     age: _ => v <= 18,
///     is_teacher: false => v === false,
///     locker_code: Some("00000".into()) => v.len() == 5,
///     homework: vec![]
/// );
/// let t1 = set!(teacher, name = "Bob".into());
/// let t1a = set!(t1, age = 53);
/// let t1b = set!(t2, age = 22);
/// ```

macro_rules! checker {
    ($type:ty, $id:ident, $c:tt) => {
        Some(Arc::from(| $id: &$type | { $c }) as Arc<dyn Fn(&$type) -> bool + Send + Sync + 'static>)
    };
    ($type:ty, $id:ident, $c:ident) => {
        Some(Arc::from($c))
    };
}

macro_rules! field {
    ($type:ty, $id:ident = _ => $check:expr) => {
        KeyVal::new_with_blank(checker!($type, $id, $check))
    };
    ($type:ty, $id:ident = $val:expr => $check:expr) => {
        KeyVal::new_with_defaults($val, checker!($type, $id, $check))
    };
    ($type:ty, $id:ident = _) => {
        KeyVal::new_with_blank(None, None)
    };
    ($type:ty, $id:ident = $val:expr) => {
        KeyVal::new_with_defaults($val, None)
    };
}

macro_rules! partial {
    ($target:ty, $($id:ident: $typeof:ty = { $($prop:tt)+ } ),+) => {
        | $($id: Option<$typeof>),+ | {
            $(
                paste! { let mut [<kv_ $id>] = field!($typeof, $id = $($prop)+); }
            );+

            move | $(paste! {[<final_ $id>]}: $typeof),+ | -> Result<$target, Box<dyn std::error::Error>> {
                $(
                    paste!{
                        let [<result_ $id>] = [<kv_ $id>].set([<final_ $id>])?;
                    }
                );+
                let mut __map = Partial::new();
                $(
                    paste!{__map.insert(stringify!($id), [<result_ $id>])?;}
                );+
                //let final_type: $target = __map.convert::<$target>()?;
                //let final_type = Partial::from_template(&__map)?;
                __map.build()
            }
        }
    };
}

//TODO: Create a set! macro

#[test]
fn test_closures2() {
    #[derive(Serialize, Deserialize)]
    struct Person {
        name: String,
        age: u8,
        is_teacher: bool,
        keycode: Option<String>,
        homework: Option<Vec<String>>,
    }

    let teacher = partial!(
        Person,
        age: u8 = { _ => *age < 8 },
        name: String = { "bob".to_string() }
    );
}

trait Convertible: Serialize + DeserializeOwned {
    fn convert<Target: Serialize + DeserializeOwned>(
        &self,
    ) -> Result<Target, Box<dyn std::error::Error>>
    where
        Self: DeserializeOwned,
    {
        let as_partial = Partial::from_template(self)?;
        as_partial.build()
    }
}

impl<T> Convertible for T where T: Serialize + DeserializeOwned {}

#[derive(Serialize, Deserialize, Debug)]
struct PhonebookEntry1 {
    name: Option<String>,
    phone: Vec<PhoneNumber>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct PhoneNumber(u8);

#[derive(Serialize, Deserialize, Debug)]
struct PhonebookEntry2 {
    name: String,
    phone: VecDeque<i64>,
}

#[test]
fn test_conversion() {
    let p1 = PhonebookEntry1 {
        name: Some("Bob".into()),
        phone: vec![PhoneNumber(1), PhoneNumber(2)],
    };
    println!("{:?}", p1.convert::<PhonebookEntry2>().unwrap());
}

#[test]
fn test_closures() {
    enum Prop {
        Age(i32),
        Name(String),
    }
    let props = |argname: &str| {
        let name = KeyVal::new_with_blank(None);
        let age = KeyVal::new_with_blank(None);
        match argname {
            "name" => Box::new(|new_value| {
                let mut map: HashMap<String, KeyVal<Prop>> = HashMap::new();
                map.insert("name".into(), KeyVal::new_with_defaults(new_value, None));
                map.insert("age".into(), age);
                return map;
            }) as Box<dyn FnOnce(Prop) -> HashMap<String, KeyVal<Prop>>>,
            "age" => Box::new(|new_value| {
                let mut map: HashMap<String, KeyVal<Prop>> = HashMap::new();
                map.insert("name".into(), name);
                map.insert("age".into(), KeyVal::new_with_defaults(new_value, None));
                return map;
            }) as Box<dyn FnOnce(Prop) -> HashMap<String, KeyVal<Prop>>>,
            _ => panic!("Invalid argument"),
        }
    };

    let template = props("name")(Prop::Name("Bob".into()));
}
