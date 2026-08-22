use std::ops::Range;

pub trait Random: Sized {
    type Bound;
    fn random() -> Self;
    fn random_range(range: Range<Self::Bound>) -> Self;
}

pub fn random<T: Random>() -> T {
    T::random()
}

pub fn random_range<T: Random>(range: Range<T::Bound>) -> T {
    T::random_range(range)
}

macro_rules! impl_random_int {
    ($($t:ident),*) => {
        $(impl Random for $t {
            type Bound = $t;
            fn random() -> Self {
                fastrand::$t(..)
            }
            fn random_range(range: Range<$t>) -> Self {
                fastrand::$t(range)
            }
        })*
    };
}

impl_random_int!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, usize, isize);

impl Random for f32 {
    type Bound = f32;
    fn random() -> Self {
        fastrand::f32()
    }
    fn random_range(range: Range<f32>) -> Self {
        range.start + (range.end - range.start) * fastrand::f32()
    }
}

impl Random for f64 {
    type Bound = f64;
    fn random() -> Self {
        fastrand::f64()
    }
    fn random_range(range: Range<f64>) -> Self {
        range.start + (range.end - range.start) * fastrand::f64()
    }
}

impl<T: Random> Random for Vec<T> {
    type Bound = usize;
    fn random() -> Self {
        (0..10).map(|_| T::random()).collect()
    }
    fn random_range(range: Range<usize>) -> Self {
        let len = fastrand::usize(range);
        (0..len).map(|_| T::random()).collect()
    }
}

impl Random for String {
    type Bound = usize;
    fn random() -> Self {
        (0..10).map(|_| fastrand::alphanumeric()).collect()
    }
    fn random_range(range: Range<usize>) -> Self {
        let len = fastrand::usize(range);
        (0..len).map(|_| fastrand::alphanumeric()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{random, random_range};

    #[test]
    fn print_random_values() {
        println!("i8:     {}", random::<i8>());
        println!("i16:    {}", random::<i16>());
        println!("i32:    {}", random::<i32>());
        println!("i64:    {}", random::<i64>());
        println!("i128:   {}", random::<i128>());
        println!("u8:     {}", random::<u8>());
        println!("u16:    {}", random::<u16>());
        println!("u32:    {}", random::<u32>());
        println!("u64:    {}", random::<u64>());
        println!("u128:   {}", random::<u128>());
        println!("usize:  {}", random::<usize>());
        println!("isize:  {}", random::<isize>());
        println!("f32:    {}", random::<f32>());
        println!("f64:    {}", random::<f64>());
        println!("String: {}", random::<String>());
        println!("Vec<i32>: {:?}", random::<Vec<i32>>());
        println!("Vec<String>: {:?}", random::<Vec<String>>());
    }

    #[test]
    fn print_random_range_values() {
        println!("i8:     {}", random_range::<i8>(-50..50));
        println!("i16:    {}", random_range::<i16>(-500..500));
        println!("i32:    {}", random_range::<i32>(10..100));
        println!("i64:    {}", random_range::<i64>(10..100));
        println!("i128:   {}", random_range::<i128>(10..100));
        println!("u8:     {}", random_range::<u8>(10..200));
        println!("u16:    {}", random_range::<u16>(10..100));
        println!("u32:    {}", random_range::<u32>(10..100));
        println!("u64:    {}", random_range::<u64>(10..100));
        println!("u128:   {}", random_range::<u128>(10..100));
        println!("usize:  {}", random_range::<usize>(10..100));
        println!("isize:  {}", random_range::<isize>(10..100));
        println!("f32:    {}", random_range::<f32>(0.5..1.5));
        println!("f64:    {}", random_range::<f64>(0.5..1.5));
        println!("String: {}", random_range::<String>(2..6));
        println!("Vec<i32>: {:?}", random_range::<Vec<i32>>(3..6));
    }
}
