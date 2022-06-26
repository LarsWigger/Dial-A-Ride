mod data;
mod parser;
mod solver;

use std::env::args;
use std::env::var;

fn main() {
    //parse arguments
    let full_pickup;
    let empty_pickup;
    let empty_delivery;
    let afs;
    let sample_number;
    let scenario;
    let mut arg_offset = 0;
    let verbose;
    if args().nth(1).expect("No arguments found") == String::from("--verbose") {
        arg_offset += 1;
        verbose = true;
    } else {
        verbose = false;
    }
    full_pickup = args()
        .nth(1 + arg_offset)
        .expect("No number of full pickups specified")
        .parse()
        .expect("Not a number");
    empty_pickup = args()
        .nth(2 + arg_offset)
        .expect("No number of empty pickups specified")
        .parse()
        .expect("Not a number");
    empty_delivery = args()
        .nth(3 + arg_offset)
        .expect("No number of empty deliveries specified")
        .parse()
        .expect("Not a number");
    afs = args()
        .nth(4 + arg_offset)
        .expect("No number of AFS specified")
        .parse()
        .expect("Not a number");
    sample_number = args()
        .nth(5 + arg_offset)
        .expect("No sample number specified")
        .parse()
        .expect("Not a number");
    scenario = args()
        .nth(6 + arg_offset)
        .expect("No scenario specified")
        .parse()
        .expect("Not a number");
    let base_path = var("DAR_BASE_PATH").unwrap();
    let config = parser::parse(
        &base_path,
        full_pickup,
        empty_pickup,
        empty_delivery,
        afs,
        sample_number,
        scenario,
        verbose,
    );
    let solution = solver::solve(config, verbose);
    if verbose {
        //separate output from rest
        println!("");
    }
    println!("{}", solution.display());
}
