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
    pub fn new() -> Self {
        Partial {
            val: HashMap::new(),
        }
    }
    pub fn from_val(val: HashMap<String, Value>) -> Self {
        Partial { val }
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
    pub fn has(&self, key: &str) -> bool {
        self.val.contains_key(key)
    }
    pub fn insert<V: Serialize + DeserializeOwned>(
        &self,
        key: &str,
        value: V,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut new = self.clone();
        new.val.insert(key.into(), json!(&value));
        Ok(new)
    }
    pub fn remove<V: Serialize + DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut new = self.clone();
        new.val.remove(key).ok_or("No value found for the key")?;
        Ok(new)
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

macro_rules! checker {
    ($type:ty, $id:ident, $c:tt) => {
        |$id: $type| $c
    };
    ($type:ty, $id:ident, $c:ident) => {
        $c
    };
}

macro_rules! field {
    ($typeof:ty, $id:ident => $check:expr) => {
        |map: Partial| {
            move |$id: Option<$typeof>| -> Result<Partial, Box<dyn std::error::Error>> {
                if let Some(arg) = $id {
                    if checker!($typeof, $id, $check)(arg.clone()) {
                        map.insert(stringify!($id), arg)?;
                        return Ok(map);
                    } else {
                        let err = format!(
                            "The argument ({arg}) was found to be invalid by {checker}.",
                            arg = arg,
                            checker = stringify!($check)
                        );
                        return Err(err.into());
                    }
                } else {
                    return Err(format!("No argument was supplied for {}", stringify!($id)).into());
                }
            }
        }
    };
    //($type:ty, $id:ident = _) => {
    //KeyVal::new_with_blank(None, None)
    //};
    ($typeof:ty, $id:ident $val:expr) => {
        |map: Partial| {
            |arg: Option<$typeof>| -> Result<Partial, Box<dyn std::error::Error>> {
                let value: $typeof = $val;
                if let Some(inner) = arg {
                    map.insert(stringify!($id), inner)?;
                } else {
                    map.insert(stringify!($id), value)?;
                }
                Ok(map)
            }
        }
    };
}

macro_rules! extract {
    ($typeof:ty, $id:ident => $check:expr) => {
        None
    };
    //($type:ty, $id:ident = _) => {
    //KeyVal::new_with_blank(None, None)
    //};
    ($typeof:ty, $id:ident $val:expr) => {
        Some($val)
    };
}

macro_rules! partial {
    ($($id:ident: $typeof:ty { $($prop:tt)+ } ),+) => {
        paste! {
            |map: Partial| -> Result<Partial, Box<dyn std::error::Error>> {
                $(
                    let $id: Option<$typeof> = if map.has(stringify!($id)) {
                        Some(from_str(&map.val[stringify!($id)].to_string())?)
                    } else {
                        extract!($typeof, $id $($prop)+)
                    };
                )+
                $(
                    let [<closure_ $id>] = field!($typeof, $id $($prop)+);
                )+
                $(
                    let map = [<closure_ $id>](map)($id)?;
                )+
                return Ok(map);
            }
        }
    };
}

macro_rules! with {
    ($mapfn:ident, $($key:ident = $val:expr),+) => {
        |map: Partial| -> Result<Partial, Box<dyn std::error::Error>> {
            $(
            map.insert(stringify!($key), $val)?;
            )+

            let result = $mapfn(map)?;
            Ok(result)
        }
    }
}

macro_rules! into {
    ($mapfn:ident as $type:ty, $($key:ident = $val:expr)+) => {
        (|map: Partial| -> Result<Partial, Box<dyn std::error::Error>> {
            $(
            map.insert(stringify!($key), $val)?;
            )+

            return $mapfn(map);
        })(Partial::new()).map(|r: Partial| r.build::<$type>())
    }
}

macro_rules! join_with {
    ($first:ident, $closure_expr:expr) => {
        |map: Partial| -> Result<Partial, Box<dyn std::error::Error>> {
            let map = $first(map)?;
            let result = $closure_expr(map)?;
            Ok(result)
        }
    };
}

#[test]
fn test() {
    #[derive(Serialize, Deserialize)]
    struct Person {
        name: String,
        age: u8,
        is_teacher: bool,
        keycode: Option<String>,
        homework: Option<Vec<String>>,
    }

    let normal = partial!(
        name: String { => name.len() > 0
            && name.chars().filter(|c| c.is_whitespace()).count() >= 3 
        },
        age: u8 { => age > 0 && age < 120}
    );

    let teacher = join_with!(
        normal,
        partial!(
            age: u8 { => age > 18 },
            is_teacher: bool { true },
            keycode: String { "1234567890".to_string() }
        )
    );

    let student = join_with!(
        normal,
        partial!(
            age: u8 { => age < 18 },
            is_teacher: bool { false },
            homework: Vec<String> { vec![] }
        )
    );

    let some_teacher_dude = with!(
        teacher, 
        age = 53, 
        name = "Prof. Garry Garrison".to_string()
    );

    let person = into!(some_teacher_dude as Person, is_teacher = true);
}
