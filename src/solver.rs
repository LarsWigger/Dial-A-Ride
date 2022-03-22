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
        ///Creates a new path option that is reached from the state described in `config` and `previous_option`
        /// (as well as its index, which needs to be passed separately) to the node `to`. If this is not possible due to little fuel, `None` is returned.
        pub fn next_path_option(
            config: &Config,
            previous_option: &PathOption,
            previous_index: usize,
            to: usize,
        ) -> Option<PathOption> {
            let from = previous_option.path[previous_option.path.len()];
            let fuel_needed = config.fuel_needed_for_route(from, to);
            let fuel_level = previous_option.fuel_level - fuel_needed;
            if fuel_level <= 0.0 {
                return None;
            }
            let total_distance =
                previous_option.total_distance + config.get_distance_between(from, to);
            let total_time = previous_option.total_time + config.get_time_between(from, to);
            let mut path = Vec::with_capacity(2);
            path.push(to);
            return Some(PathOption {
                fuel_level,
                total_distance,
                total_time,
                path,
                previous_index,
            });
        }

        ///If `other` is at the same node and none of its `fuel_level`, `total_distance` or `total_time` is better than that of `self`, true is returned, otherwise false
        pub fn completely_superior_to(&self, other: &PathOption) -> bool {
            return (self.fuel_level >= other.fuel_level)
                && (self.total_distance <= other.total_distance)
                && (self.total_time <= other.total_time)
                && (self.path[self.path.len() - 1] == other.path[other.path.len() - 1]);
        }

        ///If `other` is at the same node and one of its `fuel_level`, `total_distance` or `total_time` is better than that of `self`, true is returned, otherwise false
        pub fn partly_superior_to(&self, other: &PathOption) -> bool {
            return (self.path[self.path.len() - 1] == other.path[other.path.len() - 1])
                && ((self.fuel_level < other.fuel_level)
                    || (self.total_distance > other.total_distance)
                    || (self.total_time > other.total_time));
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
            //first step, use previous ones to go directly to node
            for (index, option) in self.path_options.iter().enumerate() {
                let new_option = PathOption::next_path_option(config, option, index, node);
                SearchState::possibly_add_to_path_options(&mut path_options, new_option);
            }
            panic!("Unimplemented");
        }

        fn possibly_add_to_path_options(
            path_options: &mut Vec<PathOption>,
            new_option: Option<PathOption>,
        ) {
            let unpacked_option = match new_option {
                None => return,
                Some(item) => item,
            };
            //to detect whether something was removed
            let original_length = path_options.len();
            //remove the entries that are completely inferior to the new one (CAN THIS EVEN HAPPEN?)
            path_options.retain(|x| !unpacked_option.completely_superior_to(&x));
            if (original_length != path_options.len()) {
                //something was removed
                path_options.push(unpacked_option);
                return;
            } else {
                //check whether unpacked_option is at least partially superior to one of the existing ones
                for i in 0..path_options.len() {
                    if unpacked_option.partly_superior_to(&path_options[i]) {
                        path_options.push(unpacked_option);
                        return;
                    }
                }
            }
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
