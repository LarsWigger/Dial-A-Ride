use crate::data::Config;
use crate::data::Solution;
use crate::data::Truck;

mod solver_data {
    use crate::data::*;
    use std::collections::HashMap;
    use std::rc::Rc;
    use std::thread::current;
    use std::time::Instant;

    ///Represents the route a single truck could take.
    pub struct Route {
        ///the nodes taken by the route, saved as u8 to save memory, special nodes specified above for (de-)loading at depot,
        /// fueling at depot is indicated by ROUTE_DEPOT_REFUEL in succession which is already indicated that way in the actual `PathOption`s
        path: Vec<u8>,
        ///summary metric for comparing different routes
        total_distance: u32,
    }

    impl Route {
        ///Creates a complete `Route` starting from `search_state`s path with index `path_index` and iterates over the previous_states until there is no `previous_state`
        pub fn new(
            search_state: &Rc<SearchState>,
            path_index: usize,
            total_distance: u32,
        ) -> Route {
            let path = Route::new_path_recursive(search_state, path_index, 0);
            return Route {
                path,
                total_distance,
            };
        }

        ///Recursive helper function that tracks back to the root state. On the way to the root state, it calculates the numer of elements needed in the final
        /// vector. The count from previous (upper) calls is passed as `current_size`.
        fn new_path_recursive(
            current_state: &Rc<SearchState>,
            current_state_path_index: usize,
            current_size: usize,
        ) -> Vec<u8> {
            match &current_state.previous_state {
                Option::Some(previous_state) => {
                    let mut path;
                    if current_state.was_depot_loaded() {
                        //calculate how many containers were exchanged
                        let diff_empty_20 = current_state.container_data.empty_20
                            - previous_state.container_data.empty_20;
                        let diff_empty_40 = current_state.container_data.empty_40
                            - previous_state.container_data.empty_40;
                        let size_at_this_step =
                            (diff_empty_20.abs() + diff_empty_40.abs()) as usize;
                        //recursion
                        path = Route::new_path_recursive(
                            previous_state,
                            current_state_path_index,
                            current_size + size_at_this_step,
                        );
                        //loading
                        let len_before = path.len(); //sanity check
                        if diff_empty_20 < 0 {
                            for _ in 0..-diff_empty_20 {
                                path.push(ROUTE_DEPOT_DELOAD_20);
                            }
                        } else if diff_empty_20 > 0 {
                            for _ in 0..diff_empty_20 {
                                path.push(ROUTE_DEPOT_LOAD_20);
                            }
                        }
                        if diff_empty_40 < 0 {
                            for _ in 0..-diff_empty_40 {
                                path.push(ROUTE_DEPOT_DELOAD_40);
                            }
                        } else if diff_empty_40 > 0 {
                            for _ in 0..diff_empty_40 {
                                path.push(ROUTE_DEPOT_LOAD_40);
                            }
                        }
                        assert_eq!(path.len(), len_before + size_at_this_step);
                    } else {
                        let current_path = &current_state.path_options[current_state_path_index];
                        let size_at_this_step = current_path.path.len();
                        path = Route::new_path_recursive(
                            previous_state,
                            current_path.previous_index,
                            current_size + size_at_this_step,
                        );
                        for node in &current_path.path {
                            path.push(*node as u8);
                        }
                    }
                    return path;
                }
                Option::None => {
                    //the original state will always have only 1 path option: 0
                    let mut path = Vec::with_capacity(current_size + 1);
                    path.push(0);
                    return path;
                }
            }
        }
    }
    ///Represents all the known routes for a single truck. Also has simple counters to evaluate how much complexity this saves with the given data.
    pub struct KnownRoutesForTruck {
        ///maps the `requests_visited` bits to the best corresponding route known (so far). Route is behind smart pointer to copying later on
        map: HashMap<u64, Rc<Route>>,
        ///not needed for calculation, counts how many valid routes were inserted
        valid_insertions: usize,
    }

    impl KnownRoutesForTruck {
        ///Creates a new `KnownRoutesForTruck`
        pub fn new() -> KnownRoutesForTruck {
            //TODO: use faster hasher
            let map = HashMap::new();
            return KnownRoutesForTruck {
                map,
                valid_insertions: 0,
            };
        }

        ///Adds the best route from the given `search_state` if:
        /// - there are no full containers currently loaded in `SearchState`
        /// - there is no other known route for `search_state.requests_visited` yet
        /// - the route in `search_state` for its `requests_visited` is better than the one already known
        pub fn possibly_add(&mut self, search_state: &Rc<SearchState>) {
            //check whether any full containers have been picked up but not delivered
            //this could not result in any proper result anyway, so we may as well prevent it here
            if (search_state.container_data.full_request_1_source != 0)
                && (search_state.container_data.full_request_2_source != 0)
            {
                return;
            }
            self.valid_insertions += 1;
            let requests_visited = search_state.requests_visisted;
            let (best_path_index, total_distance) =
                search_state.get_path_index_and_total_distance();
            let previous_entry = self.map.get(&requests_visited);
            let save_or_overwrite = match previous_entry {
                Option::None => true,
                Option::Some(previous_route) => (total_distance < previous_route.total_distance),
            };
            if save_or_overwrite {
                let new_route = Route::new(search_state, best_path_index, total_distance);
                self.map.insert(requests_visited, Rc::new(new_route));
            }
        }

        pub fn summarize_to_terminal(&self) {
            println!(
                "There were {} valid insertions out of which {} remain.",
                self.valid_insertions,
                self.map.len()
            );
            let percentage_discarded = ((self.valid_insertions - self.map.len()) as f64)
                / (self.valid_insertions as f64)
                * 100.;
            println!(
                "So {} routes / about {:.0}% were discarded.",
                self.valid_insertions - self.map.len(),
                percentage_discarded
            );
        }
    }

    ///Combines up to `config.num_trucks` routes, each corresponding to the truck at the respective index,
    /// while also saving the summary metric of the `total_distance` of these routes
    struct CombinationOption {
        ///summary metric, sum of the distance of the individual routes
        total_distance: u32,
        ///the different routes, each route is for the truck at the same index, wrapped in reference counted pointer to avoid a lot of copying
        routes: Vec<Rc<Route>>,
    }

    ///Combines all the `KnownRoutesForTruck` in one summary structure
    pub struct AllKnownOptions {
        ///maps `requests_visited` to the corresponding `CombinationOption` that is the best one known so far
        option_map: HashMap<u64, CombinationOption>,
        ///the number of trucks covered after the next union, needed for knowing how many to elements a single vector needs
        num_trucks_next: usize,
        ///performance metric to accurately estimate how many unnecessary branches were avoided
        num_compatible_combinations: usize,
        ///whether summaries of merges should be printed to the terminal
        verbose: bool,
    }

    impl AllKnownOptions {
        pub fn new(verbose: bool) -> AllKnownOptions {
            let option_map = HashMap::new();
            return AllKnownOptions {
                option_map,
                num_trucks_next: 1,
                num_compatible_combinations: 0,
                verbose,
            };
        }

        ///Just copy references to all the routes into `option_map` while adjusting/initializing the slightly different format
        pub fn inital_merge(&mut self, truck_options: &KnownRoutesForTruck) {
            for (key, value) in &truck_options.map {
                //gets thrown away anyway
                let mut routes = Vec::with_capacity(1);
                routes.push(Rc::clone(value));
                let option = CombinationOption {
                    routes,
                    total_distance: value.total_distance,
                };
                self.option_map.insert(*key, option);
            }
            self.num_trucks_next = 2;
        }

        ///Merge the new `truck_options` with the previously known options.
        /// If two options are compatible (no request visited by both), they can be combined.
        /// This combination is inserted if there is either no previous one or if it has a lower `total_distance`.
        /// In order to avoid conflicts, the original map is replaced with a new one without being changed itself
        pub fn subsequent_merge(&mut self, truck_options: &KnownRoutesForTruck) {
            self.num_compatible_combinations = 0;
            //performing this operation in place could lead to problems
            let mut new_map: HashMap<u64, CombinationOption> = HashMap::new();
            for (own_key, own_value) in &self.option_map {
                for (other_key, other_value) in &truck_options.map {
                    //only if there is no clear conflict between these two routes
                    //meaning, no request visited by both
                    if own_key & other_key == 0 {
                        self.num_compatible_combinations += 1;
                        let combined_key = own_key | other_key;
                        let combined_distance =
                            own_value.total_distance + other_value.total_distance;
                        //has something better already been saved to new_map?
                        let alternative = new_map.get(&combined_key);
                        let insert_into_map = match alternative {
                            Option::Some(old_value) => old_value.total_distance > combined_distance,
                            Option::None => true,
                        };
                        if insert_into_map {
                            let mut combination_routes = Vec::with_capacity(self.num_trucks_next);
                            for route in &own_value.routes {
                                combination_routes.push(Rc::clone(route));
                            }
                            combination_routes.push(Rc::clone(other_value));
                            let new_option = CombinationOption {
                                routes: combination_routes,
                                total_distance: combined_distance,
                            };
                            new_map.insert(combined_key, new_option);
                        }
                    }
                }
            }
            if self.verbose {
                println!(
                "Performed merge. There were {} previously known options and {} for the current truck. Now there are {} options.",
                self.option_map.len(),
                truck_options.map.len(),
                new_map.len()
            );
                let options_expected = self.option_map.len() * truck_options.map.len();
                let percentage_valid =
                    (self.num_compatible_combinations as f64) / (options_expected as f64) * 100.;
                println!(
                    "{} option combinations out of {} were valid, about {:.0}%.",
                    self.num_compatible_combinations, options_expected, percentage_valid
                );
                let valid_options_discarded = self.num_compatible_combinations - new_map.len();
                let percentage_discarded = (valid_options_discarded as f64)
                    / (self.num_compatible_combinations as f64)
                    * 100.;
                println!(
                    "Out of these valid options, {} / {:.0}% were discarded.",
                    valid_options_discarded, percentage_discarded
                );
            }
            self.option_map = new_map;
            self.num_trucks_next += 1;
        }

        pub fn get_solution(&self, config: Config, start_time: Instant) -> Solution {
            let seconds_taken = start_time.elapsed().as_secs();
            //calculate key to complete solution
            let num_requests = config.get_first_afs() - 1;
            let solution_key = std::u64::MAX >> (64 - num_requests);
            let comb_option = match self.option_map.get(&solution_key) {
                Option::None => {
                    return Solution::new(config, Vec::with_capacity(0), 0, seconds_taken)
                }
                Option::Some(option) => option,
            };
            //solution exists, adjust format
            let mut route_vec = Vec::with_capacity(config.get_num_trucks());
            for route in &comb_option.routes {
                let mut path_copy = Vec::with_capacity(route.path.len());
                for node in &route.path {
                    path_copy.push(*node);
                }
                //internally, the dummy depot is not used, fix that here
                let path_len = path_copy.len();
                if path_len > 1 {
                    path_copy[path_len - 1] = config.get_dummy_depot() as u8;
                }
                route_vec.push(path_copy);
            }
            return Solution::new(config, route_vec, comb_option.total_distance, seconds_taken);
        }
    }

    ///Represents a single PathOption. When navigating between two nodes, stops at fuel stations might be necessary or optional.
    /// These multiple possible paths differ in three summary values that are relevant for us:
    /// - `fuel_level`: The fuel level of the vehicle at the last node. All else being equal, higher is better as that meaens:
    ///     - less additional stops needed (or at least no difference/disadvantage)
    ///     - less time taken when refueling later and less time is always an advantage
    /// - `total_distance`: The total distance travelled by the vehicle at the last node. All else being equal, lower is always better.
    /// - `total_time`: The total time, might include waiting and fueling times. All else being equal, lower is always better. If too early, waiting is still possible
    ///
    /// Comparisons between paths ending at different nodes are obviously pointless.
    /// The concrete path is not relevant for the comparison of different paths, these three summary values describe it completely.
    /// There can be at most 9 different `PathOption`s connecting two nodes.
    /// If there were one more, one of the 10 options would be completely inferior to another, reducing the number back to 9 (mathematical proof not shown here).
    /// There can be less than 9 `PathOption`s, though.
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
        ///Creates a new path option that expands itself (`self`) to `to`. `config` and `previous_index`
        /// are needed for context, `previous_index` is the index of the original `path_option` in the last `SearchState`.
        /// Takes request service and visiting times into account as well as refueling times.
        /// Returns `None` if there is no possible path, which may be if:
        /// - fuel is insufficient
        /// - arrival time is too late
        /// If `new_path == true`, the depot service time is added to the total time.
        /// If `new_path == true`, `self.path_options` is not copied over. This is needed when `self` is from the previous state.
        pub fn next_path_option(
            &self,
            config: &Config,
            truck: &Truck,
            previous_index: usize,
            to: usize,
            depot_service: bool,
            new_path: bool,
        ) -> Option<PathOption> {
            let from = self.get_current_node();
            assert_ne!(from, to);
            let min_time = ((self.total_distance as f64) * 1.2) as u32;
            assert!(
                min_time <= self.total_time,
                "min_time: {} total_time: {} node: {}",
                min_time,
                self.total_time,
                from
            );
            //calculate new total distance
            let additional_distance = config.get_distance_between(from, to);
            let total_distance = self.total_distance + additional_distance;
            //calculate fuel level on arrival
            let fuel_needed = config.get_fuel_needed_for_distance(additional_distance);
            if fuel_needed > self.fuel_level {
                return None;
            }
            let fuel_level = self.fuel_level - fuel_needed;
            //calculate new total_time, dealing with handling and refueling times
            let mut total_time = self.total_time + config.get_time_between(from, to);
            //request handling times
            if to < config.get_first_afs() {
                if to != 0 {
                    if total_time > config.get_latest_visiting_time_at_request_node(to) {
                        //too late, impossible
                        return None;
                    } else if total_time < config.get_earliest_visiting_time_at_request_node(to) {
                        //too early, just wait
                        total_time = config.get_earliest_visiting_time_at_request_node(to)
                    }
                    total_time += config.get_service_time_at_request_node(to);
                } else if depot_service {
                    total_time += config.get_depot_service_time();
                }
            }
            //t_max applies to every type of node, including AFS and the depot
            if total_time > config.get_t_max() {
                return None;
            }
            //create new path
            let mut path;
            if new_path {
                path = Vec::with_capacity(1);
            } else {
                path = Vec::with_capacity(self.path.len() + 1);
                for node in &self.path {
                    path.push(*node);
                }
            }
            path.push(to);
            //create new option before possibly refueling
            let mut new_option = PathOption {
                fuel_level,
                total_distance,
                total_time,
                path,
                previous_index,
            };
            //refuel if at AFS, dummy depot should never be reached
            assert_ne!(to, config.get_dummy_depot());
            if config.get_first_afs() <= to {
                new_option.refuel(truck);
            }
            return Some(new_option);
        }

        ///If currently at the depot, this function returns the next `PathOption` where the truck has been completely refilled.
        /// If `new_path == true`, the depot service time is added to the total time.
        /// If `new_path == true`, `self.path_options` is not copied over. This is needed when `self` is from the previous state.
        fn refuel_at_depot(
            &self,
            config: &Config,
            truck: &Truck,
            previous_index: usize,
            depot_service: bool,
            new_path: bool,
        ) -> Option<PathOption> {
            //only call refuel_at_depot when already at a depot
            assert_eq!(self.get_current_node(), 0);
            //create new path
            let mut path;
            if new_path {
                path = Vec::with_capacity(1);
            } else {
                path = Vec::with_capacity(self.path.len() + 1);
                for node in &self.path {
                    path.push(*node);
                }
            }
            path.push(ROUTE_DEPOT_REFUEL);
            //create new option before refueling
            let mut new_option = PathOption {
                fuel_level: self.fuel_level,
                total_distance: self.total_distance,
                total_time: self.total_time,
                path,
                previous_index,
            };
            new_option.refuel(truck);
            if depot_service {
                new_option.total_time += config.get_depot_service_time();
            }
            return Option::Some(new_option);
        }

        ///Returns `true` if `self` would be preferred over `other` in every scenario.
        /// Also checks whether the two paths are comparable in the first place.
        /// This criterium includes `partly_superior_to`, so equivalence does not count as complete superiority.
        pub fn completely_superior_to(&self, other: &PathOption) -> bool {
            return self.comparable_to(other)
                //at least one value must be clearly better
                && self.partly_superior_to(other)
                //check whether not worse in any regard
                && (self.total_distance <= other.total_distance)
                && (self.total_time <= other.total_time)
                && (self.fuel_level >= other.fuel_level);
        }

        ///Returns `true` if `self` might be preferable over `other` in a certain scenario.
        /// Does not check whether the two paths are comparable in the first place.
        /// Weaker criterium than `completely_superior_to`, included the the latter.
        pub fn partly_superior_to(&self, other: &PathOption) -> bool {
            return (self.total_distance < other.total_distance)
                || (self.total_time < other.total_time)
                || (self.fuel_level > other.fuel_level);
        }

        ///Returns `true` if the two `PathOption`s are comparable, meaning that they are at the same node and have the same fuel level.
        pub fn comparable_to(&self, other: &PathOption) -> bool {
            return self.get_current_node() == other.get_current_node();
        }

        ///Returns whether `self` has the same summary attributes as `other` and whether the two end at the same node.
        pub fn equivalent_to(&self, other: &PathOption) -> bool {
            return self.comparable_to(other)
                && (self.fuel_level == other.fuel_level)
                && (self.total_distance == other.total_distance)
                && (self.total_time == other.total_time);
        }

        ///Returns the node this `PathOption` is currently at.
        /// If this was a refuel at the depot, return `0`, not the special value
        pub fn get_current_node(&self) -> usize {
            let last_el = self.path[self.path.len() - 1];
            if last_el == ROUTE_DEPOT_REFUEL {
                return 0;
            }
            return last_el;
        }

        fn refuel(&mut self, truck: &Truck) {
            self.total_time += truck.get_minutes_for_refueling(self.fuel_level);
            self.fuel_level = truck.get_fuel();
        }

        ///Tries to create a `PathOption` after this one where the truck is refueled at the depot.
        /// If `depot_service == true`, the service time of the depot is added to the `total_time`
        /// If `new_path == true`, `self.path_options` is not copied over. This is needed when `self` is from the previous state.
        /// Returns `true` if the insertion into `path_options` was successful.
        pub fn try_refueling_at_depot(
            &self,
            config: &Config,
            truck: &Truck,
            option_index: usize,
            path_options: &mut Vec<Rc<PathOption>>,
            depot_service: bool,
            new_path: bool,
        ) -> bool {
            let new_option;
            if self.get_current_node() != 0 {
                //not already at depot, navigate to depot normally first. no reason to save this temporary one in path_options
                let depot_option = match self.next_path_option(
                    config,
                    truck,
                    option_index,
                    0,
                    depot_service,
                    new_path,
                ) {
                    Option::None => return false, //cannot be reached, nothing more to do here
                    Option::Some(tmp) => tmp,
                };
                new_option =
                    depot_option.refuel_at_depot(config, truck, option_index, false, false);
            } else {
                new_option =
                    self.refuel_at_depot(config, truck, option_index, depot_service, new_path);
            }
            //add option where refueled at depot, new_option was set correctly before
            return SearchState::possibly_add_to_path_options(path_options, new_option);
        }
    }

    ///Represents the current state of the search. Each SearchState is at either a request node or a depot, AFS are covered in the `PathOption`s.
    pub struct SearchState {
        ///current node in the distance/time matrix
        current_node: usize,
        ///all the paths leading from the previous `SearchState` to `current_node`, empty path possible, indicates loading at depot
        path_options: Vec<Rc<PathOption>>,
        ///represents the containers the truck is carrying in this state,
        options: Vec<ContainerNumber>,
        full_request_1_source: usize,
        full_request_2_source: usize,
        ///the requests that have been visited so far, binary encoding for efficiency
        requests_visisted: u64,
        ///the next state after this one, may or may not exist and may be overwritten later
        previous_state: Option<Rc<SearchState>>,
    }

    impl SearchState {
        ///Creates the initial `SearchState` at the depot with the given `fuel_capacity`, no actions taken so far
        pub fn start_state(truck: &Truck) -> Rc<SearchState> {
            let mut path = Vec::with_capacity(1);
            path.push(0);
            let mut path_options = Vec::with_capacity(1);
            path_options.push(Rc::new(PathOption {
                fuel_level: truck.get_fuel(),
                total_distance: 0,
                total_time: 0,
                path,
                previous_index: 0,
            }));
            //container loading options
            let container_vec_capacity = (2 ^ (truck.get_num_20() + truck.get_num_40())) as usize;
            let mut options = Vec::with_capacity(container_vec_capacity);
            for empty_20 in 0..(truck.get_num_20() + 1) {
                for empty_40 in 0..(truck.get_num_40() + 1) {
                    options.push(ContainerNumber {
                        empty_20,
                        empty_40,
                        num_20: empty_20,
                        num_40: empty_40,
                        previous_index: 0,
                    });
                }
            }
            let search_state = SearchState {
                current_node: 0,
                path_options,
                options,
                full_request_1_source: 0,
                full_request_2_source: 0,
                requests_visisted: 0,
                previous_state: Option::None,
            };
            return Rc::new(search_state);
        }

        ///Creates the next state after `current_state` using `path_options`. Can't be called on `self` because the `Rc` to `self` is needed
        fn create_next_state_after_current_state(
            config: &Config,
            truck: &Truck,
            current_state: &Rc<SearchState>,
            path_options: Vec<Rc<PathOption>>,
        ) -> Rc<SearchState> {
            //this function should only be called when at least one path exists
            let current_node = path_options[0].get_current_node();
            let new_state_reference = Rc::clone(&current_state);
            let (options, full_request_1_source, full_request_2_source) =
                current_state.next_container_data(config, truck, current_node);
            let mut new_state = SearchState {
                current_node,
                path_options,
                options,
                full_request_1_source,
                full_request_2_source,
                requests_visisted: current_state.requests_visisted,
                previous_state: Option::Some(new_state_reference),
            };
            assert!(!new_state.get_request_visited(current_node));
            new_state.set_request_visited(current_node);
            return Rc::new(new_state);
        }

        ///Returns a tuple of three elements, in this order:
        /// 1. `options`
        /// 2. `full_request_1_source`
        /// 3. `full_request_2_source`
        fn next_container_data(
            &self,
            config: &Config,
            truck: &Truck,
            request_node: usize,
        ) -> (Vec<ContainerNumber>, usize, usize) {
            if request_node == 0 {
                return self.handle_depot_visit(config, truck, request_node);
            } else {
                assert!(request_node < config.get_first_afs());
                return self.handle_request_loading(config, truck, request_node);
            }
        }

        fn handle_request_loading(
            &self,
            config: &Config,
            truck: &Truck,
            request_node: usize,
        ) -> (Vec<ContainerNumber>, usize, usize) {
            //number of options can decrease, but never increase
            let mut options = Vec::with_capacity(self.options.len());
            let full_request_1_source;
            let full_request_2_source;
            let request = config.get_request_at_node(request_node);
            if request_node <= config.get_full_pickup() {
                //full pickup request
                if self.full_request_1_source == 0 {
                    full_request_1_source = request_node;
                    full_request_2_source = self.full_request_2_source;
                } else if self.full_request_2_source == 0 {
                    full_request_1_source = self.full_request_1_source;
                    full_request_2_source = request_node;
                } else {
                    panic!("THIS SHOULD NOT HAPPEN!");
                }
                for previous_index in 0..self.options.len() {
                    let container_number = self.options[previous_index];
                    let num_20 = container_number.num_20 + request.full_20;
                    let num_40 = container_number.num_40 + request.full_40;
                    if num_20 > truck.get_num_20() || num_40 > truck.get_num_40() {
                        //cannot load the new containers with the previous load
                        continue;
                    }
                    options.push(ContainerNumber {
                        empty_20: container_number.empty_20,
                        empty_40: container_number.empty_40,
                        num_20,
                        num_40,
                        previous_index,
                    });
                }
            } else if request_node < config.get_first_full_dropoff() {
                //empty pickup request
                full_request_1_source = self.full_request_1_source;
                full_request_2_source = self.full_request_2_source;
                for previous_index in 0..self.options.len() {
                    let container_number = self.options[previous_index];
                    let num_20 = container_number.num_20 + request.empty_20;
                    let num_40 = container_number.num_40 + request.empty_40;
                    if num_20 > truck.get_num_20() || num_40 > truck.get_num_40() {
                        //cannot load the new containers with the previous load
                        continue;
                    }
                    let empty_20 = container_number.empty_20 + request.empty_20;
                    let empty_40 = container_number.empty_40 + request.empty_40;
                    options.push(ContainerNumber {
                        empty_20,
                        empty_40,
                        num_20,
                        num_40,
                        previous_index,
                    });
                }
            } else if request_node < config.get_first_full_dropoff() + config.get_full_pickup() {
                //full delivery
                let source_node = config.get_pick_node_for_full_dropoff(request_node);
                if self.full_request_1_source == source_node {
                    full_request_1_source = 0;
                    full_request_2_source = self.full_request_2_source;
                } else if self.full_request_2_source == source_node {
                    full_request_1_source = self.full_request_1_source;
                    full_request_2_source = 0;
                } else {
                    panic!("THIS SHOULD NOT HAPPEN!");
                }
                for previous_index in 0..self.options.len() {
                    let container_number = self.options[previous_index];
                    //values are negative, so this is effectively a subtraction
                    let num_20 = container_number.num_20 + request.full_20;
                    let num_40 = container_number.num_40 + request.full_40;
                    //cannot lead to invalid options, the necessary containers have to be loaded
                    options.push(ContainerNumber {
                        empty_20: container_number.empty_20,
                        empty_40: container_number.empty_40,
                        num_20,
                        num_40,
                        previous_index,
                    });
                }
            } else {
                //empty delivery
                full_request_1_source = self.full_request_1_source;
                full_request_2_source = self.full_request_2_source;
                for previous_index in 0..self.options.len() {
                    let container_number = self.options[previous_index];
                    //can also just add this because the values are negative
                    let empty_20 = container_number.empty_20 + request.empty_20;
                    let empty_40 = container_number.empty_40 + request.empty_40;
                    if empty_20 < 0 || empty_40 < 0 {
                        //cannot load the new containers with the previous load
                        continue;
                    }
                    let num_20 = container_number.num_20 + request.empty_20;
                    let num_40 = container_number.num_40 + request.empty_40;
                    options.push(ContainerNumber {
                        empty_20,
                        empty_40,
                        num_20,
                        num_40,
                        previous_index,
                    });
                }
            }
            return (options, full_request_1_source, full_request_2_source);
        }

        fn handle_depot_visit(
            &self,
            config: &Config,
            truck: &Truck,
            request_node: usize,
        ) -> (Vec<ContainerNumber>, usize, usize) {
        }

        ///Returns whether the given `request` has been visited, makes the binary encoding more accessible.
        pub fn get_request_visited(&self, request_node: usize) -> bool {
            assert!(request_node != 0 && request_node < 64);
            //request binary is all 0 with 1 at the request index from the right
            let request_binary: u64 = 1 << (request_node - 1);
            let request_result = self.requests_visisted & request_binary;
            return request_result != 0;
        }

        ///Sets the given `request` as visited, makes the binary encoding more accessible
        fn set_request_visited(&mut self, request_node: usize) {
            assert!(request_node != 0 && request_node < 64);
            //request binary is all 0 with 1 at the request index from the right
            let request_binary: u64 = 1 << (request_node - 1);
            self.requests_visisted = self.requests_visisted | request_binary;
        }

        /// Calculates all the relevant routes from `current_state` to the target node and returns the next `SearchState` already wrapped in `Rc`.
        /// This may be `Option::None` if no route is found
        pub fn route_to_node(
            config: &Config,
            truck: &Truck,
            current_state: &Rc<SearchState>,
            node: usize,
        ) -> Option<Rc<SearchState>> {
            assert_ne!(current_state.current_node, node);
            //TODO: calculate a more realistic size, there is a maximum number of elements that can be calculated
            let vec_capacity = 20;
            let mut path_options = Vec::with_capacity(vec_capacity);
            //first step, initial filling using the PathOptions from this state
            SearchState::fill_with_path_options_from_previous_state(
                config,
                truck,
                current_state,
                node,
                &mut path_options,
            );
            //second step, try using previously found path_options to find a better path to somewhere (not only the depot)
            SearchState::fill_with_following_paths(config, truck, node, &mut path_options);
            //third step, remove anything that does not end at the target node
            path_options.retain(|option| option.get_current_node() == node);
            if path_options.len() == 0 {
                //no path found
                return Option::None;
            } else {
                return Option::Some(SearchState::create_next_state_after_current_state(
                    config,
                    truck,
                    current_state,
                    path_options,
                ));
            }
        }

        ///Helper function to separate code. Fills `path_options` with the ones that can be reached from the ones in `current_state`.
        /// The paths from `current_state` are not copied as they
        fn fill_with_path_options_from_previous_state(
            config: &Config,
            truck: &Truck,
            current_state: &Rc<SearchState>,
            node: usize,
            path_options: &mut Vec<Rc<PathOption>>,
        ) {
            let initial_state;
            let depot_service;
            if current_state.was_depot_loaded() {
                initial_state = match &current_state.previous_state {
                    Option::None => panic!("SHOULD NEVER HAPPEN!"),
                    Option::Some(state) => state,
                };
                depot_service = true;
            } else {
                initial_state = current_state;
                depot_service = false;
            }
            //first step, use paths from initial_state to go directly to node or fuel stations or depot
            for (previous_index, option) in initial_state.path_options.iter().enumerate() {
                //target node
                let mut new_option = option.next_path_option(
                    config,
                    truck,
                    previous_index,
                    node,
                    depot_service,
                    true,
                );
                SearchState::possibly_add_to_path_options(path_options, new_option);
                //fuel stations
                for afs in config.get_first_afs()..config.get_dummy_depot() {
                    new_option = option.next_path_option(
                        config,
                        truck,
                        previous_index,
                        afs,
                        depot_service,
                        true,
                    );
                    SearchState::possibly_add_to_path_options(path_options, new_option);
                }
                option.try_refueling_at_depot(
                    config,
                    truck,
                    previous_index,
                    path_options,
                    depot_service,
                    true,
                );
            }
        }

        ///Extends the paths already in `path_options` trying to route to `node`
        fn fill_with_following_paths(
            config: &Config,
            truck: &Truck,
            node: usize,
            path_options: &mut Vec<Rc<PathOption>>,
        ) {
            let mut improvement_found = true;
            let mut iteration_clone = Vec::with_capacity(path_options.capacity());
            while improvement_found {
                improvement_found = false;
                //iterate over efficient copy of path_options because compiler complains otherwise
                iteration_clone.retain(|_| false);
                for index in 0..path_options.len() {
                    let option = Rc::clone(&path_options[index]);
                    iteration_clone.push(option);
                }
                //iterate over all the options known so far and see whether they lead to a new interesting option
                for option in iteration_clone.iter() {
                    //save for repeated usage:
                    let current_node = option.get_current_node();
                    //going away from the target node is both forbidden and pointless
                    if current_node != node {
                        //straight to target node
                        improvement_found |= SearchState::possibly_add_to_path_options(
                            path_options,
                            option.next_path_option(
                                config,
                                truck,
                                option.previous_index,
                                node,
                                false,
                                false,
                            ),
                        );
                        //refuel at a particular AFS
                        for afs in config.get_first_afs()..config.get_dummy_depot() {
                            //routing from itself to itself is completely pointless
                            if current_node != afs {
                                improvement_found |= SearchState::possibly_add_to_path_options(
                                    path_options,
                                    option.next_path_option(
                                        config,
                                        truck,
                                        option.previous_index,
                                        afs,
                                        false,
                                        false,
                                    ),
                                );
                            }
                        }
                        //refuel at depot
                        improvement_found |= option.try_refueling_at_depot(
                            config,
                            truck,
                            option.previous_index,
                            path_options,
                            false,
                            false,
                        );
                    }
                }
            }
        }

        ///Removes the elements of `path_options` that are completely inferior to `new_option`
        /// and adds `new_option` if it was partially superior to at least one of the previous elements.
        /// Returns `true` if a change was made, `false` otherwise
        fn possibly_add_to_path_options(
            path_options: &mut Vec<Rc<PathOption>>,
            new_option: Option<PathOption>,
        ) -> bool {
            let unpacked_option = match new_option {
                None => return false,
                Some(item) => item,
            };
            //to detect whether something was removed
            let original_length = path_options.len();
            //remove the entries that are completely inferior to the new one (CAN THIS EVEN HAPPEN?)
            path_options.retain(|option| !unpacked_option.completely_superior_to(&option));
            if original_length != path_options.len() {
                //something was removed => completely superior to something => insert, done
                path_options.push(Rc::new(unpacked_option));
                return true;
            } else {
                //check whether there is a reason against adding the new_option
                for i in 0..path_options.len() {
                    let comp_option = &path_options[i];
                    if comp_option.completely_superior_to(&unpacked_option)
                        || comp_option.equivalent_to(&unpacked_option)
                    {
                        return false;
                    }
                }
                //no reason against insertion found
                path_options.push(Rc::new(unpacked_option));
                return true;
            }
        }

        //Getter for `current_node`
        pub fn get_current_node(&self) -> usize {
            return self.current_node;
        }

        ///Returns the index of the `PathOption` with the lowest `total_distance` as well as the `total_distance` itself
        pub fn get_path_index_and_total_distance(&self) -> (usize, u32) {
            let mut best_index = 0;
            let mut lowest_distance = std::u32::MAX;
            for (index, option) in self.path_options.iter().enumerate() {
                if option.total_distance < lowest_distance {
                    lowest_distance = option.total_distance;
                    best_index = index;
                }
            }
            return (best_index, lowest_distance);
        }

        ///Returns `true` if the state can handle the request next (without visiting a different request first).
        /// Time is not taken into account, in that case the routing would just return None
        pub fn can_handle_request(
            &self,
            config: &Config,
            truck: &Truck,
            request_node: usize,
        ) -> bool {
            assert!(self.container_data.num_20 <= truck.get_num_20());
            assert!(self.container_data.num_40 <= truck.get_num_40());
            assert!(self.container_data.empty_20 <= truck.get_num_20());
            assert!(self.container_data.empty_40 <= truck.get_num_40());
            assert!(self.container_data.empty_20 >= 0);
            assert!(self.container_data.empty_40 >= 0);
            //cannot visit the same request twice
            if self.get_request_visited(request_node) {
                return false;
            }
            let request = config.get_request_at_node(request_node);
            //only to need to check whether something can be picked up
            if request_node < config.get_first_full_dropoff() {
                //pickup request
                let newly_loaded_20 = request.empty_20 + request.full_20;
                let num_20_after_loading =
                    request.empty_20 + request.full_20 + self.container_data.num_20;
                if num_20_after_loading > truck.get_num_20() {
                    return false;
                }
                let newly_loaded_40 = request.empty_40 + request.full_40;
                let num_40_after_loading = newly_loaded_40 + self.container_data.num_40;
                if num_40_after_loading > truck.get_num_40() {
                    return false;
                }
                assert!(newly_loaded_20 + newly_loaded_40 > 0);
            } else {
                //dropoff
                if request_node < config.get_first_empty_dropoff() {
                    //full container dropoff
                    //just check whether the corresponging pickup request was visited
                    return self
                        .get_request_visited(config.get_pick_node_for_full_dropoff(request_node));
                } else {
                    //empty container dropoff
                    if -request.empty_20 > self.container_data.empty_20 {
                        return false;
                    }
                    if -request.empty_40 > self.container_data.empty_40 {
                        return false;
                    }
                }
            }
            return true;
        }

        ///Returns whether containers were loaded/unloaded at the depot in this state
        pub fn was_depot_loaded(&self) -> bool {
            return self.path_options.len() == 0;
        }

        ///Calculates and returns the number empty 20- and 40-foot containers that still need to be delivered in this state
        pub fn get_containers_still_needed(&self, config: &Config) -> EmptyContainersStillNeeded {
            let mut empty_20_delivery = 0;
            let mut empty_40_delivery = 0;
            for empty_delivery_node in config.get_first_empty_dropoff()..config.get_first_afs() {
                let request = config.get_request_at_node(empty_delivery_node);
                empty_20_delivery -= request.empty_20;
                empty_40_delivery -= request.empty_40;
            }
            return EmptyContainersStillNeeded {
                empty_20_delivery,
                empty_40_delivery,
            };
        }

        ///Checks whether that many empty 20- and 40-foot containers could be (un-)loaded at the depot.
        /// Also tries to avoid unecessary branching - picking up additional containers is pointless if none need to be delivered.
        pub fn can_handle_depot_load(
            &self,
            truck: &Truck,
            containers_needed: &EmptyContainersStillNeeded,
            change_20: i8,
            change_40: i8,
        ) -> bool {
            let new_20 = self.container_data.num_20 + change_20;
            let new_40 = self.container_data.num_40 + change_40;
            //easy check: can that many containers be loaded at all?
            if (new_20 < 0)
                || (new_20 > truck.get_num_20())
                || (new_40 < 0)
                || (new_40 > truck.get_num_40())
            {
                return false;
            }
            //avoid unecessary branching
            //no reason at all to pickup containers that cannot be delivered anyway
            //this also neatly covers deloading. If it would be better to deload 2 rather than just 1, the option for 1 will be rejected
            let new_empty_20 = self.container_data.empty_20 + change_20;
            let new_empty_40 = self.container_data.empty_40 + change_40;
            if (new_empty_20 > containers_needed.empty_20_delivery)
                || (new_empty_40 > containers_needed.empty_40_delivery)
                || (new_empty_20 < 0)
                || (new_empty_40 < 0)
            {
                return false;
            }
            return true;
        }

        ///Creates the next state after `current_state` where nothing has been changed except hat `change_20` empty 20-foot containers (can be negative) have been loaded.
        /// Same for `change_40`. `path_options` is empty because a) there is nothing for it anyway and b) this indicates that something was loaded in this state
        pub fn load_at_depot(
            current_state: &Rc<SearchState>,
            change_20: i8,
            change_40: i8,
        ) -> Rc<SearchState> {
            //this function should only be called when at least one path exists
            let current_node = current_state.current_node;
            let new_state_reference = Rc::clone(&current_state);
            let path_options = Vec::with_capacity(0);
            let mut new_state = SearchState {
                current_node,
                path_options,
                container_data: current_state.container_data.clone(),
                requests_visisted: current_state.requests_visisted,
                previous_state: Option::Some(new_state_reference),
            };
            new_state.container_data.empty_20 += change_20;
            new_state.container_data.empty_40 += change_40;
            new_state.container_data.num_20 += change_20;
            new_state.container_data.num_40 += change_40;
            assert!(new_state.container_data.empty_20 >= 0);
            assert!(new_state.container_data.empty_40 >= 0);
            return Rc::new(new_state);
        }
    }

    ///Combines all the data about the current state of loaded containers for a `SearchState`
    struct ContainerNumber {
        ///number of empty 20 foot containers loaded
        empty_20: i8,
        ///number of empty 40 foot containers loaded
        empty_40: i8,
        ///number of 20 foot containers (empty + full) loaded. Saving it this way saves computations
        num_20: i8,
        ///number of 40 foot containers (empty + full) loaded. Saving it this way saves computations
        num_40: i8,
        ///the index of the previous `ContainerNumber` option
        previous_index: usize,
    }

    pub struct EmptyContainersStillNeeded {
        ///Number of empty 20-foot containers that still need to be delivered
        pub empty_20_delivery: i8,
        ///Number of empty 40-foot containers that still need to be delivered
        pub empty_40_delivery: i8,
    }

    ///Represents all the possible loadings done at the depot
    /// First number is addition of empty_20 containers, second of empty_40 containers
    pub static POSSIBLE_DEPOT_LOADS: &'static [(i8, i8)] = &[
        //pure loading
        (1, 0),
        (2, 0),
        (1, 1),
        (0, 1),
        //pure unloading
        (-1, 0),
        (-2, 0),
        (-1, -1),
        (0, -1),
        //mixed
        (1, -1),
        (-1, 1),
    ];
}
use solver_data::*;
use std::rc::Rc;
use std::time::Instant;

pub fn solve(config: Config, verbose: bool) -> Solution {
    let start_time = Instant::now();
    let mut all_known_options = AllKnownOptions::new(verbose);
    let mut options_for_truck = solve_for_truck(&config, 0, verbose);
    all_known_options.inital_merge(&options_for_truck);
    for truck_index in 1..config.get_num_trucks() {
        let current_truck = config.get_truck(truck_index);
        //avoid unnecessary recalculation of options_for_truck
        if config.get_truck(truck_index - 1) != current_truck {
            options_for_truck = solve_for_truck(&config, truck_index, verbose);
        } else if verbose {
            println!(
                "\nTruck {} is identical to the one before, no calculation necessary.",
                truck_index
            );
        }
        all_known_options.subsequent_merge(&options_for_truck);
    }
    return all_known_options.get_solution(config, start_time);
}

///Calculates all the known options for truck at given index
fn solve_for_truck(config: &Config, truck_index: usize, verbose: bool) -> KnownRoutesForTruck {
    let truck = config.get_truck(truck_index);
    if verbose {
        println!("\nCalculating options for truck {} ...", truck_index);
        println!(
            "Truck can load {} 20- and {} 40-foot containers, having a fuel capacity of {}.",
            truck.get_num_20(),
            truck.get_num_40(),
            truck.get_fuel() / 100
        );
    }

    let root_state = SearchState::start_state(truck);
    let mut known_options = KnownRoutesForTruck::new();
    solve_for_truck_recursive(config, &truck, &mut known_options, &root_state);
    if verbose {
        known_options.summarize_to_terminal();
    }
    return known_options;
}

fn solve_for_truck_recursive(
    config: &Config,
    truck: &Truck,
    known_options: &mut KnownRoutesForTruck,
    current_state: &Rc<SearchState>,
) {
    //dummy depot is never routed to internally, there is no reason to differentiate between it and the original depot
    assert!(current_state.get_current_node() != config.get_dummy_depot());
    //depot, handled first because if it cannot be reached the rest is pointless anyway
    if current_state.get_current_node() == 0 {
        //should be reached only at the start (where depot loading is applied) and after depot loading (where it is only treated as possible complete route)
        apply_depot_actions(config, truck, known_options, current_state);
    } else {
        //not already at depot, try routing to it before applying the depot actions
        match SearchState::route_to_node(config, truck, &current_state, 0) {
            Option::None => return, //if the depot cannot be reached, the route cannot be ended anyway, so stop here
            Option::Some(depot_state) => {
                apply_depot_actions(config, truck, known_options, &depot_state);
            }
        };
    };
    //try moving to the requests
    for request_node in 1..config.get_first_afs() {
        if current_state.can_handle_request(config, truck, request_node) {
            match SearchState::route_to_node(config, truck, &current_state, request_node) {
                Option::None => (), //do
                Option::Some(state) => {
                    solve_for_truck_recursive(config, truck, known_options, &state);
                }
            };
        };
    }
}

fn apply_depot_actions(
    config: &Config,
    truck: &Truck,
    known_options: &mut KnownRoutesForTruck,
    current_state: &Rc<SearchState>,
) {
    assert_eq!(current_state.get_current_node(), 0);
    known_options.possibly_add(&current_state);
    //loading at the depot is always done in a separate state after navigating to the depot. This prevents repeated identical routing and makes parsing the route easier
    //only do this when the depot has not been loaded in the current_state already, otherwise infinite branching would result
    if !current_state.was_depot_loaded() {
        //calculate only once
        let containers_needed = current_state.get_containers_still_needed(config);
        //some combinations are always nonsense, so these are not included in the predefined array
        for (change_20, change_40) in POSSIBLE_DEPOT_LOADS {
            if current_state.can_handle_depot_load(
                truck,
                &containers_needed,
                *change_20,
                *change_40,
            ) {
                //create new state where the loading has been applied. Due to the above condition, no loading will happen immadiately afterwards
                let loaded_state =
                    SearchState::load_at_depot(&current_state, *change_20, *change_40);
                solve_for_truck_recursive(config, truck, known_options, &loaded_state);
            }
        }
    }
}
