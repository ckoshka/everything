use std::sync::Arc;
use std::collections::HashMap;

pub struct Segment {
    pub name: Option<&'static str>,
    pub checker: Arc<dyn Fn(&str) -> bool>
}

pub struct Matcha {
    fns: Vec<Segment>
}

impl Matcha {
    pub fn new() -> Self {
        Matcha {
            fns: Vec::new()
        }
    }

    pub fn add(&mut self, checker: Arc<dyn Fn(&str) -> bool>, name: Option<&'static str>) {
        self.fns.push(Segment {
            name,
            checker
        });
    }

    pub fn try_match<'a>(&self, string: &'a str) -> HashMap<&'static str, &'a str> {
        //string.find
        // what about {} dog {}
        // means it'll consume the entie string which isn't what we want
        // so we'd need to get match_indices to find dog and see if it overlaps?
        // therefore overlapping should be allowed?
        // but since {} matches on anything, it'll stop after the first character
        // so we need to take context into account...
        // longest matching substring? then check for ordering and absence of gaps, or whether
        // a match was found at all
        todo!()
    }

    pub fn is_match(&self, string: &str) -> bool {
        todo!()
    }
}

macro_rules! matcha {
    (@ $($right:tt)*) => {
        {
            let mtch = Matcha::new();
            matcha!(mtch, $($right)*);
            mtch
        }
    };

    ($mtch:ident, { $name:ident }, $($right:tt)*) => {
        $mtch.add(Arc::from(|_| true), Some(stringify!($name)));
        matcha!($($right)*)
    };

    ($mtch:ident, { $name:ident: $checker:expr }, $($right:tt)*) => {
        $mtch.add(Arc::from(|s| ($checker)(s)), Some(stringify!($name)));
        matcha!($($right)*)
    };

    ($mtch:ident, $words:literal, $($right:tt)*) => {
        $mtch.add(Arc::from(|s| s == $words), None);
        matcha!($($right)*)
    };

    ($mtch:ident, { $checker:expr }, $($right:tt)*) => {
        $mtch.add(Arc::from(|s| ($checker)(s)), None);
        matcha!($($right)*)
    };

    ($mtch:ident, $checker:expr, $($right:tt)*) => {
        $mtch.add(Arc::from(|s| s == $checker), None);
        matcha!($($right)*)
    };
}

fn compile_check() {
    let my_str = "My favourite colour is #00ffee and I am 192 years old.";
    let is_color = |s: &str| s.chars().map(|c| i64::from_str_radix(c.to_string().as_str(), 16).is_ok()).all(|x| x);
    let x = matcha!(@ "My favourite colour is #", {color: is_color}, " and I am ", {age}, "years old.");
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
