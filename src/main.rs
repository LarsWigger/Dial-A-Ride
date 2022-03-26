mod data;
mod parser;
mod solver;

fn main() {
    let config = parser::parse(2, 2, 2, 2, 1, 2);
    let solution = solver::solve(config);
    println!("Hello, world!");
}
