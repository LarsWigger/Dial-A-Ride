use crate::data::Config;
use crate::data::Truck;

mod solver_data {
    use crate::data::Config;
    use crate::data::ContainerRequest;
    use crate::data::Truck;
    use std::collections::HashMap;

    #[derive(Eq, PartialEq)]
    enum ContainerType {
        ///a full 20 foot container for the specific full container pickup request
        Full20(usize),
        ///any empty 20 foot container, which one makes no difference
        Empty20,
        ///a full 40 foot container for the specific full container pickup request
        Full40(usize),
        ///an empty 40 foot container, which one makes no difference
        Empty40,
        ///no container at all
        NoContainer,
    }

    impl ContainerType {
        pub fn isNoContainer(&self) -> bool {
            return match self {
                NoContainer => true,
                _ => false,
            };
        }
    }

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

        pub fn get_current_node(&self) -> usize {
            return self.path[self.path.len() - 1];
        }
    }

    enum NextState {
        NoNext,
        Next(Box<SearchState>),
    }

    pub struct SearchState {
        ///current node in the distance/time matrix
        current_node: usize,
        path_options: Vec<PathOption>,
        container_1: ContainerType,
        container_2: ContainerType,
        requests_visisted: u64,
        next_state: NextState,
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
                next_state: NextState::NoNext,
            };
            return search_state;
        }

        fn create_next_one_changing_containers(
            &self,
            config: &Config,
            path_options: Vec<PathOption>,
        ) -> &SearchState {
            //this function should only be called when at least one path exists
            let current_node = path_options[0].get_current_node();
            let mut new_state = SearchState {
                current_node,
                path_options,
                container_1: self.container_1,
                container_2: self.container_2,
                requests_visisted: self.requests_visisted,
                next_state: NextState::NoNext,
            };
            //do containers need to be changed/is the new node a request?
            if current_node < config.get_first_afs() {
                new_state.handle_request_containers(config);
            }
            self.next_state = NextState::Next(Box::new(new_state));
            return &new_state;
        }

        ///Adjusts the containers at the request node the state is currently at and marks this request as visited afterwards
        /// Must only be called when the current node is a request (unchecked) and the request has not been visited before (checked)
        fn handle_request_containers(&mut self, config: &Config) {
            //request no already handled:
            assert!(!self.get_request_visited(self.current_node));
            let request = config.get_request_at_node(self.current_node);
            //handle deload first in case load and deload happen at the same node (should not happen)
            if request.full_20 < 0 {
                let unloaded_container = ContainerType::Empty20;
                self.unload_container(unloaded_container, request.full_20);
            }
            if request.full_40 < 0 {
                let unloaded_container = ContainerType::Empty20;
                self.unload_container(unloaded_container, request.full_20);
            }
            if request.empty_20 < 0 {
                let unloaded_container = ContainerType::Empty20;
                self.unload_container(unloaded_container, request.full_20);
            }
            if request.empty_40 < 0 {
                let unloaded_container = ContainerType::Empty20;
                self.unload_container(unloaded_container, request.full_20);
            }
            //handle load
            if request.full_20 > 0 {
                let container = ContainerType::Full20(self.current_node);
                self.load_container(container, request.full_20);
            }
            if request.full_40 > 0 {
                let container = ContainerType::Full40(self.current_node);
                self.load_container(container, request.full_40);
            }
            if request.empty_20 > 0 {
                let container = ContainerType::Empty20;
                self.load_container(container, request.empty_20);
            }
            if request.empty_40 > 0 {
                let container = ContainerType::Empty40;
                self.load_container(container, request.empty_40);
            }
            self.set_request_visited(self.current_node);
        }

        ///Unloads `-number` (it is negative, the inversion is done in the method) containers of type `unload_container`
        fn unload_container(&mut self, unload_container: ContainerType, number: i32) {
            assert!(number < 0);
            for _ in 0..-number {
                if self.container_1 == unload_container {
                    self.container_1 = ContainerType::NoContainer;
                } else if self.container_2 == unload_container {
                    self.container_2 = ContainerType::NoContainer;
                } else {
                    panic!("INVALID STATE");
                }
            }
        }

        ///Loads `number` containers of type `load_container`
        fn load_container(&mut self, load_container: ContainerType, number: i32) {
            assert!(number > 0);
            for _ in 0..number {
                if self.container_1.isNoContainer() {
                    self.container_1 = load_container;
                } else if self.container_2.isNoContainer() {
                    self.container_2 = load_container;
                } else {
                    panic!("INVALID STATE");
                }
            }
        }

        pub fn get_request_visited(&self, request: usize) -> bool {
            //request binary is all 0 with 1 at the request index from the right
            let request_binary: u64 = 1 << request;
            let request_result = self.requests_visisted & request_binary;
            return request_result != 0;
        }

        fn set_request_visited(&mut self, request: usize) {
            //request binary is all 0 with 1 at the request index from the right
            let request_binary: u64 = 1 << request;
            self.requests_visisted = self.requests_visisted | request_binary;
        }

        pub fn route_to_node(&self, config: &Config, node: usize) -> Option<&SearchState> {
            let mut path_options = Vec::with_capacity(4);
            let fuel_needed_directly = config.fuel_needed_for_route(self.current_node, node);
            //first step, use previous ones to go directly to node
            for (index, option) in self.path_options.iter().enumerate() {
                let new_option = PathOption::next_path_option(config, option, index, node);
                SearchState::possibly_add_to_path_options(&mut path_options, new_option);
            }
            //second step, go to AFS in order to try reaching the node from there (possibly even chaining multiple AFS)
            for (index, option) in self.path_options.iter().enumerate() {
                for afs in config.get_first_afs()..config.get_first_afs() + config.get_afs() {
                    let new_option = PathOption::next_path_option(config, option, index, afs);
                    SearchState::possibly_add_to_path_options(&mut path_options, new_option);
                }
                //TODO: DEAL WITH REFUELING AT DEPOT
            }
            //third step, try using previous one to get to
            let mut improvement_found = true;
            while improvement_found {
                improvement_found = false;
                //to new node
                for (index, option) in self.path_options.iter().enumerate() {
                    //save for repeated usage:
                    let current_node = option.get_current_node();
                    //to new node
                    if option.get_current_node() != node {
                        let new_option = PathOption::next_path_option(config, option, index, node);
                        let made_change = SearchState::possibly_add_to_path_options(
                            &mut path_options,
                            new_option,
                        );
                        if made_change {
                            improvement_found = true;
                        }
                    }
                    //to afs
                    for afs in config.get_first_afs()..config.get_first_afs() + config.get_afs() {
                        if current_node != afs {
                            let new_option =
                                PathOption::next_path_option(config, option, index, node);
                            let made_change = SearchState::possibly_add_to_path_options(
                                &mut path_options,
                                new_option,
                            );
                            if made_change {
                                improvement_found = true;
                            }
                        }
                    }
                    //TODO: DEAL WITH REFUELING AT DEPOT
                }
            }
            //fourth step, remove anything that does not end at the depot
            path_options.retain(|x| x.get_current_node() == node);
            if path_options.len() == 0 {
                //no path found
                return None;
            } else {
                let result = self.create_next_one_changing_containers(config, path_options);
                return Some(result);
            }
        }

        ///Removes the elements of `path_options` that are completely inferior to `new_option`
        /// and adds `new_option` if it was partially superior to at least one of the previous elements
        /// Returns `true` if a change was made
        fn possibly_add_to_path_options(
            path_options: &mut Vec<PathOption>,
            new_option: Option<PathOption>,
        ) -> bool {
            let unpacked_option = match new_option {
                None => return false,
                Some(item) => item,
            };
            //to detect whether something was removed
            let original_length = path_options.len();
            //remove the entries that are completely inferior to the new one (CAN THIS EVEN HAPPEN?)
            path_options.retain(|x| !unpacked_option.completely_superior_to(&x));
            if (original_length != path_options.len()) {
                //something was removed
                path_options.push(unpacked_option);
                return true;
            } else {
                //check whether unpacked_option is at least partially superior to one of the existing ones
                for i in 0..path_options.len() {
                    if unpacked_option.partly_superior_to(&path_options[i]) {
                        path_options.push(unpacked_option);
                        return true;
                    }
                }
                return false;
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
