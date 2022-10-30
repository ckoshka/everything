/// Macros for accessing heavily-nested hashmaps via dot-notation. Example usage:
/// ```rust
/// use dotmap::{map, dot};
/// let mut map = map!(HashMap: {a: {b: {c: 1}}});
/// assert_eq!(dot!(map.a.b.c), Some(1));
/// assert_eq!(dot!(map.a.b.d), None);
/// ```

macro_rules! map {
    // First, let's start with the basics and create a matcher for literal expressions that just does nothing or clones idents.
    ($maptype:ident: $token:literal) => {
        $token
    };
    ($maptype:ident: $token:ident) => {
        $token
    };
    // This might look somewhat pointless, but it means we can parse dicts like the above by calling map! on each value. Recursion is pretty neat, huh?
    // Now in the next pattern, we'll match more interesting things like {"c": 1, "d": 2, "e": 3}.
    // The basic pattern here is just "$key: $value, " and we can use the * operator to repeat as many times as we want.
    // What we'll do first is just create a default version of a provided parsed type (i.e HashMap::default() or MyStruct::default()) called default_map. Then for each key/value pair, we'll call map! on the value, and then insert the key/value pair into default_map.
    // Finally, we'll return default_map.
    ($maptype:ident: {$($key:tt: $value:tt),+}) => {
        {
            let mut default_map = $maptype::new();
            $(default_map.insert(map!($maptype: $key), map!($maptype: $value));)+
            default_map
        }
    };
    // We also need to handle empty dicts.
    ($maptype:ident: {}) => {
        $maptype::new()
    };
    // And we need to handle lists.
    ($maptype:ident: [$($value:tt),+]) => {
        vec![$(map!($maptype: $value)),+]
    };
    // And finally, we need to handle expressions like 1 + 1 or download_as_string(website).
    ($maptype:ident: $token:expr) => {
        $token
    }
}

// Let's do dot! now.
// Surprisingly, this is actually way simpler. We treat the first token as an ident, and then use the + matcher to match the rest of the tokens as ".$token". We could go recursive, but here it's simpler to just do a loop digging deeper into the structure until we see the key we want.
// There's a caveat, though. We have two different types of keys:
// - idents, like a or b without quotes - we convert these to strings
// - literals, like "a" or "b" with quotes or 1 and 2 - we leave these alone
// that means we first need to create two pattern matchers for both of these cases, and in our final user-facing macro, we use tt to match either one of these patterns.
/// assert_eq!(dot!(map.a.b.c), Some(1));
/// assert_eq!(dot!(map.a.b.d), None);
macro_rules! dot {
    ($token:ident) => {
        stringify!($token)
    };
    // But if it's an ident surrounded by {}, we assume that the user wants it to be the actual value of that key.
    ({$token:ident}) => {
        $token
    };
    ($token:literal) => {
        $token
    };
    ($strc:ident$(.$token:tt)*) => {
        {
            loop {
                let current_struct = &$strc;
                $(
                    let key = dot!($token);
                    let value = current_struct.get(&key);
                    if value.is_none() {
                        break None;
                    }
                    let value = value.unwrap();
                    let current_struct = value;
                )*
                break Some(current_struct);
            }
        }
    };
}

mod search;

use std::collections::HashMap;

#[test]
fn test() {
    let even_numbers = false;
    let special_num = 4;
    let map = map!(HashMap: {"a": {"b": {special_num: {
        if even_numbers {
            vec![0, 2, 4, 6, 8]
        } else {
            vec![1, 3, 5, 7]
        }
    }}}});
    let _result = dot!(map.a.b.{special_num});
}
