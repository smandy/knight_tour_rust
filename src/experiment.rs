use std::ops::Add;

// sum is a generic function with one type parameter, T
pub(crate) fn mysum<T>(num1: T, num2: T) -> T
    where
        T: Add<Output = T>,  // T must implement the Add trait where addition returns another T
{
    num1 + num2  // num1 + num2 is syntactic sugar for num1.add(num2) provided by the Add trait
}

fn main() {
    let result1 = mysum(10, 20);
    println!("Sum is: {}", result1); // Sum is: 30

    let result2 = mysum(10.23, 20.45);
    println!("Sum is: {}", result2); // Sum is: 30.68
}
