// #![cfg(feature = "stats")]
//
// use std::collections::BTreeMap;
//
// use log::trace;
// use parking_lot::Mutex;
//
// static STATS: Mutex<BTreeMap<String, Stat>> = Mutex::new(BTreeMap::new());
//
// #[derive(Clone, Default)]
// pub struct Stat {
//     pub type_name: String,
//     pub count:     i64,
// }
//
// impl Stat {
//     fn new(type_name: impl ToString) -> Self {
//         Self {
//             type_name: type_name.to_string(),
//             count:     0,
//         }
//     }
// }
//
// pub(crate) fn adjust_stat(name: &str, change: i64) {
//     let mut stats = STATS.lock();
//
//     let stat = if let Some(stat) = stats.get_mut(name) {
//         stat
//     } else {
//         stats.insert(name.to_string(), Stat::new(name.to_string()));
//         stats.get_mut(name).unwrap()
//     };
//
//     trace!("Stat change for {name}: change: {change}, count: {}",
// stat.count,);
//
//     stat.count += change;
//
//     debug_assert!(stat.count >= 0);
// }
//
// pub fn dump_ref_stats() {
//     let stats = STATS.lock();
//
//     if stats.is_empty() {
//         println!("No managed objects.");
//     }
//
//     let mut total_count = 0;
//
//     println!("==================Memory stats==================");
//     for (name, stat) in stats.iter() {
//         let name = clear_name(name);
//         let count = stat.count;
//         total_count += count;
//         println!("Type: {name}, count: {count}");
//     }
//     println!("Total count: {total_count}");
//     println!("================================================");
// }
//
// fn clear_name(name: &str) -> String {
//     if let Some(last) = name.rfind(':') {
//         name[last + 1..].to_string()
//     } else {
//         name.to_string()
//     }
// }
//
// #[cfg(test)]
// mod test {
//     use serial_test::serial;
//
//     use crate::deps::refs::{
//         Own,
//         stats::{STATS, Stat, clear_name},
//     };
//
//     pub(crate) fn get_stat<T>() -> Stat {
//         let name = std::any::type_name::<T>().to_string();
//         STATS.lock().get(&name).unwrap_or(&Stat::default()).clone()
//     }
//
//     trait Trait {}
//
//     struct Test {
//         _data: u32,
//     }
//
//     impl Trait for Test {}
//
//     #[test]
//     #[serial]
//     fn stats_count() {
//         assert_eq!(get_stat::<i32>().count, 0);
//
//         let _1 = Own::new(5);
//         let _2 = Own::new(5);
//         let _3 = Own::new(5);
//
//         assert_eq!(get_stat::<i32>().count, 3);
//         drop(_1);
//         assert_eq!(get_stat::<i32>().count, 2);
//         drop(_2);
//         assert_eq!(get_stat::<i32>().count, 1);
//         drop(_3);
//         assert_eq!(get_stat::<i32>().count, 0);
//     }
//
//     #[test]
//     #[serial]
//     fn stats_dyn() {
//         let _rf: Own<dyn Trait> = Own::<Test>::new(Test { _data: 0 });
//     }
//
//     #[test]
//     fn stats_misc() {
//         assert_eq!("clear_name",
// clear_name("crate::deps::refs::stats::clear_name"));     }
// }
