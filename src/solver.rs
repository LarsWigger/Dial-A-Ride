use crate::data::Config;
use crate::data::ContainerRequest;
use crate::data::Truck;

mod solver_data {
    use crate::data::Config;
    use crate::data::ContainerType;
    use crate::data::Truck;
    use std::collections::HashMap;

    struct Route {
        route: Vec<usize>,
        truck_index: usize,
    }
    pub struct KnownOptions {
        map: HashMap<u64, Route>,
    }

    impl KnownOptions {
        pub fn new() -> KnownOptions {
            let map = HashMap::new();
            return KnownOptions { map };
        }
    }

    struct PathOption {
        fuel_level: f32,
        total_distance: u32,
        total_time: u32,
        path: Vec<usize>,
        previous_index: usize,
    }

    impl PathOption {
        pub fn new(
            config: &Config,
            previous_option: &PathOption,
            previous_index: usize,
            to: usize,
        ) -> PathOption {
            let from = previous_option.path[previous_option.path.len()];
            let fuel_needed = config.fuel_needed_for_route(from, to);
            let total_distance =
                previous_option.total_distance + config.get_distance_between(from, to);
            let total_time = previous_option.total_time + config.get_time_between(from, to);
            let mut path = Vec::with_capacity(2);
            path.push(to);
            return PathOption {
                fuel_level: previous_option.fuel_level - fuel_needed,
                total_distance,
                total_time,
                path,
                previous_index,
            };
        }
    }

    enum PreviousState {
        NoPrevious,
        Previous(Box<SearchState>),
    }

    pub struct SearchState {
        ///current node in the distance/time matrix
        current_node: usize,
        path_options: Vec<PathOption>,
        container_1: ContainerType,
        container_2: ContainerType,
        requests_visisted: u64,
        previous_state: PreviousState,
    }

    impl SearchState {
        pub fn new(fuel_level: f32) -> SearchState {
            let mut path_options = Vec::with_capacity(1);
            path_options.push(PathOption {
                fuel_level,
                total_distance: 0,
                total_time: 0,
                path: Vec::with_capacity(0),
                previous_index: 0,
            });
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

        pub fn route_to_node(&self, config: &Config, node: usize) -> SearchState {
            let mut path_options = Vec::with_capacity(4);
            let fuel_needed_directly = config.fuel_needed_for_route(self.current_node, node);
            for (index, option) in self.path_options.iter().enumerate() {
                let option = &self.path_options[index];
                if option.fuel_level > fuel_needed_directly {
                    path_options.push(PathOption::new(config, &option, index, node));
                }
            }
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
