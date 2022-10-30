use serde::{Serialize, Deserialize};
use serde_json::{Value, json};

// Serialises objects into a string, then searches for a particular value.
trait Searchable {
    fn search_inner<'a, T: for<'de> serde::Deserialize<'de> + Serialize>(
        &self,
        json: &Value,
        valfinder: &dyn Fn(&T) -> bool,
    ) -> Option<T> {
        //println!("The value is: {:#?}", json);
        if let Ok(v) = serde_json::from_value::<T>(json.clone()) {
            if valfinder(&v) {
                return Some(v);
            }
        }
        // Otherwise, if it's a list, recurse on each value.
        if let Some(array) = json.as_array() {
            for val in array {
                if let Some(v) = self.search_inner::<T>(val, valfinder) {
                    return Some(v);
                }
            }
        }
        if let Some(obj) = json.as_object() {
            for (_, val) in obj {
                if let Some(v) = self.search_inner::<T>(val, valfinder) {
                    return Some(v);
                }
                // Otherwise, parse the value as T, and return it if it matches the closure.
                if let Ok(v) = serde_json::from_value::<T>(val.clone()) {
                    if valfinder(&v) {
                        return Some(v);
                    }
                }
            }
            return None;
        } else {
            return None;
        }
    }
    fn search<'a, T: for<'de> serde::Deserialize<'de> + Serialize>(&self, valfinder: &dyn Fn(&T) -> bool) -> Option<T> where Self: Serialize {
        let self_as_val = json!(self);
        self.search_inner(&self_as_val, valfinder)
    }
}

impl<T> Searchable for T where T: Serialize {}

#[derive(Serialize, Deserialize, Debug)]
struct Restaurant {
    name: String,
    dishes: Vec<Dish>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Dish {
    name: String,
    price: f32,
    ratings: Vec<Rating>
}

#[derive(Serialize, Deserialize, Debug)]
struct Rating {
    stars: u8
}

#[test]
fn test_search() {
    let restaurant = Restaurant {
        name: "Luigi's Dubious Italian Restaurant".to_string(),
        dishes: vec![
            Dish {
                name: "Spaghetti that definitely isn't made of mealworms".to_string(),
                price: 5.0,
                ratings: vec![
                    Rating {
                        stars: 4
                    },
                    Rating {
                        stars: 5
                    }
                ]
            },
            Dish {
                name: "Mystery pizza".to_string(),
                price: 10.0,
                ratings: vec![
                    Rating {
                        stars: 1
                    },
                    Rating {
                        stars: 3
                    }
                ]
            }
        ]
    };
    let rating_filter = |r: Rating| r.stars > 3;
    let result = restaurant.search::<Rating>(&rating_filter);
    println!("{:#?}", result.unwrap());
}
