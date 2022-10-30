// ignores unused macros

#![allow(unused_macros)]

pub mod macros {
    use paste::paste;
    use futures::future::{FutureExt};


    macro_rules! boxed {
        ($name:ident: $target:ty, $( $closure:expr ),*) => {
            vec![$(Box::new(macros::enclose!($name: $target, $closure))),*]
        };
    }
    pub(crate) use boxed;

    macro_rules! arclock {
        ($($obj:tt)*) => {
            Arc::new(parking_lot::RwLock::new($($obj)*))
        };
    }
    pub(crate) use arclock;

    macro_rules! enclose {
        ($name:ident: $target:ty, {$($inner:expr)*} ) => {
            move |$name: $target| {
                $($inner);*
            }
        };
        ($name:ident: $target:ty, $inner:expr) => {
            move |$name: $target| {
                $inner($name);
                Ok(())
            }
        };
    }
    pub(crate) use enclose;

    macro_rules! flow {
        ($d:tt, $macroname:ident, $queue_name:ident, $target_name:ident: $target_type:ty, $($inner:expr)*) => {
            paste! {
                pub fn [<__internal_macro_ $macroname>]<Parent: Send + Sync + Copy>(mut $queue_name: Vec<Box<dyn FnMut(Parent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + Send + Sync>>, mut $target_name: Parent) -> Result<(), Box<dyn std::error::Error+ Send + Sync>> {
                    $($inner)*
                }

                macro_rules! $macroname {
                    ($name:ident: &mut $target:ty, $d($d inner:tt)*) => {
                        move |$name: &mut $target| {
                            let boxes: Vec<Box<(dyn FnMut(&mut $target) -> Result<(), Box<(dyn std::error::Error + Send + Sync)>> + Send + Sync)>> = boxed!($name: &mut $target, $d ($d inner)*);
                            [<__internal_macro_ $macroname>]::<&mut $target>(boxes, $name)
                        }
                    };
                    ($name:ident: &$target:ty, $d($d inner:tt)*) => {
                        move |$name: &$target| {
                            let boxes: Vec<Box<(dyn FnMut(&$target) -> Result<(), Box<(dyn std::error::Error + Send + Sync)>> + Send + Sync)>> = macros::boxed!($name: &$target, $d ($d inner)*);
                            macros::[<__internal_macro_ $macroname>]::<&$target>(boxes, $name)
                        }
                    };
                    ($d($d inner:tt)*) => {
                        move || {
                            let boxes: Vec<Box<(dyn FnMut(_) -> Result<(), Box<(dyn std::error::Error + Send + Sync)>> + Send + Sync)>> = boxed!(_unused: (), $d ($d inner)*);
                            [<__internal_macro_ $macroname>](boxes, ())
                        }
                    };
                }
            }
        };
    }
    pub(crate) use flow;

    flow!(
        $,
        do_all,
        actions,
        target: &mut Parent,
        {
            for mut action in actions.into_iter() {
                let _res = action(target);
            }
            Ok(())
        }
    );
    pub(crate) use do_all;

    flow!(
        $,
        try_each,
        actions,
        target: &mut Parent,
        {
            let mut ret = Ok(());
            for mut action in actions.into_iter() {
                if ret.is_err() {
                    ret = action(target);
                } else {
                    break;
                }
            }
            ret
        }
    );
    pub(crate) use try_each;


    flow!(
        $,
        attempt_all,
        actions,
        target: &mut Parent,
        {
            for mut action in actions.into_iter() {
                action(target)?
            }
            Ok(())
        }
    );
    pub(crate) use attempt_all;

    flow!(
        $,
        do_threads,
        actions,
        target: &Parent,
        {
            crossbeam_utils::thread::scope(|s| {
                let mut handles = vec![];
                for mut action in actions.iter_mut() {
                    let t = s.spawn(|_| {
                        action(target)
                    });
                    handles.push(t);
                };
                for handle in handles {
                    handle.join().unwrap();
                }
            }).unwrap();
            Ok(())
        }
    );
    pub(crate) use do_threads;

    flow!(
        $,
        catch,
        actions,
        target: &mut Parent,
        {
            for mut action in actions.into_iter() {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    action(target)
                })).unwrap();
                if result.is_err() {
                    return Err("Panic".into());
                }
            }
            Ok(())
        }
    );
    pub(crate) use catch;

}

#[test]
fn test() {
    // Some silly examples
    use std::sync::Arc;
    #[derive(Debug)]
    struct Sandwich {
        contents: Arc<parking_lot::RwLock<Vec<String>>>,
        paid_for: Arc<parking_lot::RwLock<bool>>,
    }

    impl Sandwich {
        fn new() -> Self {
            Self {
                contents: macros::arclock!(Vec::new()),
                paid_for: macros::arclock!(false),
            }
        }
        fn add_topping(&self, topping: &str) -> Result<(), &str> {
            if self.contents.read().len() > 7 {
                return Err("Too many toppings");
            }
            self.contents.write().push(topping.to_string());
            Ok(())
        }
        fn remove_topping(&self, topping: &str) -> Result<(), &str> {
            // Panic if there's no such topping
            let topping = self
                .contents
                .read()
                .iter()
                .position(|t| t == topping)
                .unwrap();
            self.contents.write().remove(topping);
            Ok(())
        }
        fn add_sauce(&self, sauce: &str) -> Result<(), &str> {
            let prohibited_sauces = vec!["ketchup", "mustard", "mayonnaise"];
            if prohibited_sauces.contains(&sauce) {
                return Err("Prohibited sauce");
            }
            self.contents.write().push(sauce.to_string());
            Ok(())
        }
        fn pay_for(&self) -> Result<(), &str> {
            let mut paid_for = self.paid_for.write();
            if *paid_for {
                return Err("Already paid for");
            }
            *paid_for = true;
            Ok(())
        }
    }
    let mysandwich = Sandwich::new();
    let secret_sauce = "ketchup";
    let sequence = macros::do_all!(
        sandwich: &Sandwich,
        {
            macros::do_threads!(
                sandwich: &Sandwich,
                {
                    sandwich.add_topping("lettuce")?;
                },
                {
                    sandwich.add_topping("tomato")?;
                },
                {
                    sandwich.add_topping("onion")?;
                },
                {
                    sandwich.add_topping("mustard")?;
                },
                {
                    sandwich.add_topping("mayonnaise")?;
                }
            )(&sandwich)?;

            println!("Finishd adding toppings");
        },
        macros::catch!(
            sandwich: &Sandwich,
            {
                println!("Statements can be arbitrarily nested and recombined, while still capturing variables appropriately.");
            },
            {
                sandwich.add_sauce(secret_sauce)?;
                sandwich.add_sauce("permitted sauce")?;
                sandwich.remove_topping("beetroot")?;
            }
        ),
        {
            sandwich.pay_for()?;
        },
        macros::try_each!(
            sandwich: &Sandwich,
            {
                sandwich.pay_for()?;
            },
            {
                println!(
                    "Payment failed. Here's the sandwich: {:?}",
                    sandwich.contents.read()
                );
            }
        )
    );
    sequence(&mysandwich).unwrap();
    println!("{:#?}", mysandwich);
}
