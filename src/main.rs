mod data;
mod parser;
mod solver;

fn main() {
    let config = parser::parse(2, 2, 2, 2, 1, 2);
    println!("Hello, world!");
}
