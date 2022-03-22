use crate::data::Config;
use crate::data::ContainerRequest;
use crate::data::Truck;

mod solver_data {
    use crate::data::Config;
    use crate::data::ContainerType;
    use crate::data::Truck;
    use std::collections::HashMap;

    pub struct KnownOptions {
        map: HashMap<u64, Route>,
    }

    impl KnownOptions {
        pub fn new() -> KnownOptions {
            let map = HashMap::new();
            return KnownOptions { map };
        }
    }

    struct Route {
        route: Vec<usize>,
        truck_index: usize,
    }

    enum PreviousState {
        NoPrevious,
        Previous(Box<SearchState>),
    }

    pub struct SearchState {
        current_node: usize,
        path_options: Vec<PathOption>,
        container_1: ContainerType,
        container_2: ContainerType,
        requests_visisted: u64,
        previous_state: PreviousState,
    }

    pub struct PathOption {
        fuel_level: f32,
        total_distance: u32,
        total_time: u32,
        path: Vec<usize>,
    }

    impl PathOption {
        pub fn new(
            fuel_level: f32,
            total_distance: u32,
            total_time: u32,
            path: Vec<usize>,
        ) -> PathOption {
            return PathOption {
                fuel_level,
                total_distance,
                total_time,
                path,
            };
        }
    }

    impl SearchState {
        pub fn new(fuel_level: f32) -> SearchState {
            let path_options = Vec::with_capacity(4);
            let search_state = SearchState {
                current_node: 0,
                path_options,
                container_1: ContainerType::NoContainer,
                container_2: ContainerType::NoContainer,
                requests_visisted: 0,
                previous_state: PreviousState::NoPrevious,
            };
            return search_state;
        }

        pub fn get_request_visited(&self, request: usize) -> bool {
            //request binary is all 0 with 1 at the request index from the right
            let request_binary: u64 = 1 << request;
            let request_result = self.requests_visisted & request_binary;
            return request_result != 0;
        }

        pub fn set_request_visisted(&mut self, request: usize) {
            //request binary is all 0 with 1 at the request index from the right
            let request_binary: u64 = 1 << request;
            self.requests_visisted = self.requests_visisted | request_binary;
        }

        pub fn next_step(&self, path_options: Vec<PathOption>) -> SearchState {
            panic!("Placeholder next_step()");
        }

        pub fn route_to_node(&self, config: &Config, node: usize) -> SearchState {
            panic!("Unimplemented");
        }
    }
}
use solver_data::*;

pub fn solve(config: &Config) {}

///Calculates all the known options for truck at given index
fn solve_for_truck(config: &Config, truck_index: usize) -> KnownOptions {
    let mut known_options = KnownOptions::new();
    let truck = config.get_truck(truck_index);
    let root_state = SearchState::new(truck.get_fuel());
    solve_for_truck_recursive(config, &truck, &mut known_options, &root_state);
    return known_options;
}

fn solve_for_truck_recursive(
    config: &Config,
    truck: &Truck,
    known_options: &mut KnownOptions,
    search_state: &SearchState,
) {
    panic!("UNIMPLEMENTED");
}

#[cfg(test)]
mod routing_tests {
    use crate::parser;
    use crate::solver::*;
    #[test]
    fn route_0_to_1() {
        let config = parser::parse(2, 2, 2, 2, 1, 2);
        let search_state = SearchState::new(config.get_truck(0).get_fuel());
        let path_options = search_state.route_to_node(&config, 1);
    }
}
