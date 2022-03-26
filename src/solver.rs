use crate::data::Config;
use crate::data::Truck;

mod solver_data {
    use crate::data::Config;
    use crate::data::ContainerRequest;
    use crate::data::Truck;
    use std::collections::HashMap;
    use std::rc::Rc;

    ///Represents the type of container currently loaded. The full containers are additionally identified by the pickup request for them.
    /// For the empty containers, this makes no difference at all.
    #[derive(Eq, PartialEq, Copy, Clone)]
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

    pub struct Route {
        route: Vec<usize>,
        truck_index: usize,
    }
    pub struct KnownOptions {
        map: HashMap<u64, Route>,
        truck_index: usize,
    }

    impl KnownOptions {
        pub fn new(truck_index: usize) -> KnownOptions {
            let map = HashMap::new();
            return KnownOptions { map, truck_index };
        }

        pub fn get_map(self) -> HashMap<u64, Route> {
            return self.map;
        }
    }

    ///Represents a single PathOption. When navigating between two nodes, stops at fuel stations might be necessary or optional.
    /// These multiple possible paths differ in three summary values that are relevant for us:
    /// - `fuel_level`: The fuel level of the vehicle at the last node
    /// - `total_distance`: The total distance travelled by the vehicle at the last node. Should be as low as possible, but a different route may be longer but have more fuel remaining
    /// - `total_time`: The total time, might include waiting and fueling times
    ///
    /// The concrete path is not relevant for the comparison of different paths, these three summary values describe it completely.
    struct PathOption {
        ///the fuel level of the vehicle at the last node, in 0.01l to avoid floating point operations
        fuel_level: u32,
        ///the total distance travelled by the vehicle at the last node
        total_distance: u32,
        ///the total
        total_time: u32,
        ///the nodes traversed in this path
        path: Vec<usize>,
        ///the index of the previous `PathOption` this one uses as a base
        previous_index: usize,
    }

    impl PathOption {
        ///Creates a new path option that exands itself (`self`) to `to`. `config` and `previous_option`
        /// are needed for context, `previous_option` is the index of the original `path_option` in the last `SearchState`.
        /// Takes request service and visiting times into account as well as refueling times.
        /// Returns `None` if there is no possible path, which may be if:
        /// - fuel is insufficient
        /// - arrival time is too late
        pub fn next_path_option(
            &self,
            config: &Config,
            truck: &Truck,
            previous_index: usize,
            to: usize,
        ) -> Option<PathOption> {
            let from = self.path[self.path.len() - 1];
            let fuel_needed = config.fuel_needed_for_route(from, to);
            if fuel_needed > self.fuel_level {
                return None;
            }
            let mut fuel_level = self.fuel_level - fuel_needed;
            //deal with handling and refueling times
            let mut total_time = self.total_time + config.get_time_between(from, to);
            //request handling times
            if to < config.get_first_afs() {
                //can request still be processed?
                if total_time > config.get_latest_visiting_time(to) {
                    return None;
                } else if total_time < config.get_earliest_visiting_time(to) {
                    //have to wait
                    total_time = config.get_earliest_visiting_time(to)
                }
                total_time += config.get_service_time(to);
            }
            //refueling
            if config.get_first_afs() <= to && to < config.get_dummy_depot() {
                total_time += truck.get_minutes_for_refueling(fuel_level);
                fuel_level = truck.get_fuel();
            }
            let total_distance = self.total_distance + config.get_distance_between(from, to);
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

        ///If `other` is at the same node and none of its `fuel_level`, `total_distance` or `total_time` is better than that of `self`, `true` is returned, otherwise `false`
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

    ///Represents the current state of the search. Each SearchState is at either a request node or a depot, AFS are covered in the `PathOption`s.
    pub struct SearchState {
        ///current node in the distance/time matrix
        current_node: usize,
        ///all the paths leading from the previous `SearchState` to `current_node`, sorted
        path_options: Vec<PathOption>,
        ///the currently loaded first container
        container_1: ContainerType,
        ///the currently loaded second container
        container_2: ContainerType,
        ///the requests that have been visited so far, binary encoding for efficiency
        requests_visisted: u64,
        ///the next state after this one, may or may not exist and may be overwritten later
        previous_state: Option<Rc<SearchState>>,
    }

    impl SearchState {
        ///Creates the initial `SearchState` at the depot with the given `fuel_capacity`, no actions taken so far
        pub fn start_state(fuel_level: u32) -> Rc<SearchState> {
            let mut path = Vec::with_capacity(1);
            path.push(0);
            let mut path_options = Vec::with_capacity(1);
            path_options.push(PathOption {
                fuel_level,
                total_distance: 0,
                total_time: 0,
                path,
                previous_index: 0,
            });
            let search_state = SearchState {
                current_node: 0,
                path_options,
                container_1: ContainerType::NoContainer,
                container_2: ContainerType::NoContainer,
                requests_visisted: 0,
                previous_state: Option::None,
            };
            return Rc::new(search_state);
        }

        ///Creates the next state after `current_state` using `path_options`
        fn create_next_state_after_current_state(
            config: &Config,
            current_state: Rc<SearchState>,
            path_options: Vec<PathOption>,
        ) -> Rc<SearchState> {
            //this function should only be called when at least one path exists
            let current_node = path_options[0].get_current_node();
            let mut new_state = SearchState {
                current_node,
                path_options,
                container_1: current_state.container_1,
                container_2: current_state.container_2,
                requests_visisted: current_state.requests_visisted,
                previous_state: Option::Some(current_state),
            };
            //do containers need to be changed/is the new node a request?
            if current_node < config.get_first_afs() {
                new_state.handle_request_containers(config);
            }
            return Rc::new(new_state);
        }

        ///Adjusts the containers at the request node the state is currently at and marks this request as visited afterwards.
        /// Must only be called when the current node is a request (unchecked) and the request has not been visited before (checked)
        fn handle_request_containers(&mut self, config: &Config) {
            //request no already handled:
            assert!(!self.get_request_visited(self.current_node));
            let request = config.get_request_at_node(self.current_node);
            //handle deload first in case load and deload happen at the same node (should not happen)
            if request.full_20 < 0 {
                let unloaded_container =
                    ContainerType::Full20(config.get_pick_node_for_full_dropoff(self.current_node));
                self.unload_container(unloaded_container, request.full_20);
            }
            if request.full_40 < 0 {
                let unloaded_container =
                    ContainerType::Full40(config.get_pick_node_for_full_dropoff(self.current_node));
                self.unload_container(unloaded_container, request.full_40);
            }
            if request.empty_20 < 0 {
                let unloaded_container = ContainerType::Empty20;
                self.unload_container(unloaded_container, request.empty_20);
            }
            if request.empty_40 < 0 {
                let unloaded_container = ContainerType::Empty40;
                self.unload_container(unloaded_container, request.empty_20);
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
                if self.container_1 == ContainerType::NoContainer {
                    self.container_1 = load_container;
                } else if self.container_2 == ContainerType::NoContainer {
                    self.container_2 = load_container;
                } else {
                    panic!("INVALID STATE");
                }
            }
        }

        ///Returns whether the given `request` has been visited, makes the binary encoding more accessible
        pub fn get_request_visited(&self, request: usize) -> bool {
            //request binary is all 0 with 1 at the request index from the right
            let request_binary: u64 = 1 << request;
            let request_result = self.requests_visisted & request_binary;
            return request_result != 0;
        }

        ///Sets the given `request` as visited, makes the binary encoding more accessible
        fn set_request_visited(&mut self, request: usize) {
            //request binary is all 0 with 1 at the request index from the right
            let request_binary: u64 = 1 << request;
            self.requests_visisted = self.requests_visisted | request_binary;
        }

        /// Calculates all the relevant routes from the current state to the target node and returns the next `SearchState`
        /// This may be `Option::None` if no route is found
        pub fn route_to_node(
            config: &Config,
            truck: &Truck,
            current_state: Rc<SearchState>,
            node: usize,
        ) -> Option<Rc<SearchState>> {
            let mut path_options = Vec::with_capacity(4);
            //first step, use previous ones to go directly to node
            for (index, option) in current_state.path_options.iter().enumerate() {
                let new_option = option.next_path_option(config, truck, index, node);
                SearchState::possibly_add_to_path_options(&mut path_options, new_option);
            }
            //second step, go to AFS in order to try reaching the node from there (possibly even chaining multiple AFS)
            for (index, option) in current_state.path_options.iter().enumerate() {
                for afs in config.get_first_afs()..config.get_first_afs() + config.get_afs() {
                    let new_option = option.next_path_option(config, truck, index, afs);
                    SearchState::possibly_add_to_path_options(&mut path_options, new_option);
                }
                //TODO: DEAL WITH REFUELING AT DEPOT
            }
            //third step, try using previously found path_options one to find a better path to somewhere (not only the depot)
            let mut improvement_found = true;
            while improvement_found {
                improvement_found = false;
                for (index, option) in current_state.path_options.iter().enumerate() {
                    //save for repeated usage:
                    let current_node = option.get_current_node();
                    //to new node
                    if option.get_current_node() != node {
                        let new_option = option.next_path_option(config, truck, index, node);
                        let made_change = SearchState::possibly_add_to_path_options(
                            &mut path_options,
                            new_option,
                        );
                        if made_change {
                            improvement_found = true;
                        }
                    }
                    //refueling, makes no sense starting from the target node
                    if current_node != node {
                        //to afs
                        for afs in config.get_first_afs()..config.get_first_afs() + config.get_afs()
                        {
                            //routing from itself to itself is completely pointless
                            if current_node != afs {
                                let new_option =
                                    option.next_path_option(config, truck, index, node);
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
            }
            //fourth step, remove anything that does not end at the depot
            path_options.retain(|x| x.get_current_node() == node);
            if path_options.len() == 0 {
                //no path found, overwrite just in case
                return Option::None;
            } else {
                return Option::Some(SearchState::create_next_state_after_current_state(
                    config,
                    current_state,
                    path_options,
                ));
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
            if original_length != path_options.len() {
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

        pub fn get_current_node(&self) -> usize {
            return self.current_node;
        }

        pub fn can_handle_request(
            config: &Config,
            current_state: Rc<SearchState>,
            request_index: usize,
        ) -> bool {
            panic!("NOT IMPLEMENTED!");
        }
    }

    pub struct SearchStateWrapper {}
}
use solver_data::*;
use std::collections::HashMap;
use std::rc::Rc;

pub fn solve(config: &Config) {}

///Calculates all the known options for truck at given index
fn solve_for_truck(config: &Config, truck_index: usize) -> HashMap<u64, Route> {
    let truck = config.get_truck(truck_index);
    let root_state = SearchState::start_state(truck.get_fuel());
    let mut known_options = KnownOptions::new(truck_index);
    solve_for_truck_recursive(config, &truck, &mut known_options, root_state);
    return known_options.get_map();
}

fn solve_for_truck_recursive(
    config: &Config,
    truck: &Truck,
    known_options: &mut KnownOptions,
    current_state: Rc<SearchState>,
) {
    if current_state.get_current_node() == 0 {
        //blabla
    } else if current_state.get_current_node() == config.get_dummy_depot() {
        //blabla
        return; //should never be left again
    }
    //try moving to the requests
    for request_index in 1..config.get_first_afs() {
        if SearchState::can_handle_request(config, Rc::clone(&current_state), request_index) {
            let next_state =
                SearchState::route_to_node(config, truck, Rc::clone(&current_state), request_index);
        };
    }
}

#[cfg(test)]
mod routing_tests {
    use crate::parser;
    use crate::solver::*;
    #[test]
    fn route_0_to_1() {
        let config = parser::parse(2, 2, 2, 2, 1, 2);
        let truck = config.get_truck(0);
        let base_state = SearchState::start_state(truck.get_fuel());
        let next_state = SearchState::route_to_node(&config, truck, base_state, 1);
    }
}
