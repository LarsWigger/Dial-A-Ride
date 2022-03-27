use crate::data::Config;
use crate::data::Solution;
use crate::data::Truck;

mod solver_data {
    use crate::data::Config;
    use crate::data::Truck;
    use std::collections::HashMap;
    use std::rc::Rc;

    const ROUTE_DEPOT_REFUEL: usize = std::u8::MAX as usize;
    const ROUTE_DEPOT_LOAD_20: u8 = std::u8::MAX - 1;
    const ROUTE_DEPOT_DELOAD_20: u8 = std::u8::MAX - 2;
    const ROUTE_DEPOT_LOAD_40: u8 = std::u8::MAX - 3;
    const ROUTE_DEPOT_DELOAD_40: u8 = std::u8::MAX - 4;

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
                    if previous_state.was_depot_loaded() {
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
        valid_insertions: u64,
        ///not needed for calculation, counts how many entries were overwritten
        overwrites: u64,
    }

    impl KnownRoutesForTruck {
        ///Creates a new `KnownRoutesForTruck`
        pub fn new() -> KnownRoutesForTruck {
            //TODO: use faster hasher
            let map = HashMap::new();
            return KnownRoutesForTruck {
                map,
                valid_insertions: 0,
                overwrites: 0,
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
            let mut save = false;
            let mut overwrite = false;
            match previous_entry {
                Option::None => {
                    save = true;
                }
                Option::Some(previous_route) => {
                    overwrite = total_distance < previous_route.total_distance
                }
            };
            if save || overwrite {
                let new_route = Route::new(search_state, best_path_index, total_distance);
                self.map.insert(requests_visited, Rc::new(new_route));
                if overwrite {
                    self.overwrites += 1;
                }
            }
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
    }

    //possible TODO: the vectors are initialized with the maximum size even though it is not needed.
    impl AllKnownOptions {
        pub fn new() -> AllKnownOptions {
            let option_map = HashMap::new();
            return AllKnownOptions {
                option_map,
                num_trucks_next: 1,
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
        pub fn additional_merge(&mut self, truck_options: &KnownRoutesForTruck) {
            //performing this operation in place could lead to problems
            let mut new_map: HashMap<u64, CombinationOption> = HashMap::new();
            for (own_key, own_value) in &self.option_map {
                for (other_key, other_value) in &truck_options.map {
                    //only if there is no clear conflict between these two routes
                    //meaning, no request visited by both
                    if own_key & other_key == 0 {
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
            self.option_map = new_map;
            self.num_trucks_next += 1;
        }
    }

    ///Represents a single PathOption. When navigating between two nodes, stops at fuel stations might be necessary or optional.
    /// These multiple possible paths differ in three summary values that are relevant for us:
    /// - `fuel_level`: The fuel level of the vehicle at the last node. There is no "better" value, due to longer refueling times a lower `fuel_level` for refueling might be better
    ///     in some scenarios. However, for a single route, there are not that many different values for `fuel_level`:
    ///     - the `fuel_level` after arriving right from the original node
    ///     - the respective `fuel_level` after arriving from particular AFS or the depot after refueling, one for each
    ///     - stopping at a different request first could lead to a different `fuel_level` upon arrival, but this is covered by routing to that request first and as such, not relevant here
    /// - `total_distance`: The total distance travelled by the vehicle at the last node. All else being equal, lower is always better.
    /// - `total_time`: The total time, might include waiting and fueling times. All else being equal, lower is always better
    ///
    /// Comparisons between paths ending at different nodes are obviously pointless.
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
        ///Creates a new path option that expands itself (`self`) to `to`. `config` and `previous_index`
        /// are needed for context, `previous_index` is the index of the original `path_option` in the last `SearchState`.
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
            depot_service: bool,
        ) -> Option<PathOption> {
            let from = self.get_current_node();
            let total_distance = config.get_distance_between(from, to);
            let fuel_needed = config.fuel_needed_for_route(from, to);
            if fuel_needed > self.fuel_level {
                return None;
            }
            let fuel_level = self.fuel_level - fuel_needed;
            //deal with handling and refueling times
            let mut total_time = self.total_time + config.get_time_between(from, to);
            //request handling times
            if to < config.get_first_afs() {
                if to != 0 {
                    if total_time > config.get_latest_visiting_time(to) {
                        //too late, impossible
                        return None;
                    } else if total_time < config.get_earliest_visiting_time(to) {
                        //too early, just wait
                        total_time = config.get_earliest_visiting_time(to)
                    }
                    total_time += config.get_service_time(to);
                }
            }
            if depot_service {
                total_time += config.get_depot_service_time();
            }
            //create new path
            let mut path: Vec<usize> = Vec::with_capacity(self.path.len() + 1);
            for node in &self.path {
                path.push(*node);
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

        ///If currently at the depot, this function returns the next `PathOption` where the truck has been completely refilled
        pub fn refuel_at_depot(&self, truck: &Truck, previous_index: usize) -> Option<PathOption> {
            //only call refuel_at_depot when already at a depot
            assert!(self.get_current_node() == 0);
            //create new path
            let mut path: Vec<usize> = Vec::with_capacity(self.path.len() + 1);
            for node in &self.path {
                path.push(*node);
            }
            path.push(ROUTE_DEPOT_REFUEL);
            //create new option before
            let mut new_option = PathOption {
                fuel_level: self.fuel_level,
                total_distance: self.total_distance,
                total_time: self.total_time,
                path,
                previous_index,
            };
            new_option.refuel(truck);
            return Option::Some(new_option);
        }

        ///Returns `true` if there is no scenario where `other` would be preferred over self `self`, `false` otherwise
        pub fn completely_superior_to(&self, other: &PathOption) -> bool {
            return self.partly_superior_to(other)
                //check whether not worse in either regard
                && (self.total_distance <= other.total_distance)
                && (self.total_time <= other.total_time);
        }

        ///Returns `true` if `self` might be preferable over `other` in a certain scenario. This does not indicate complete superiority, which is a harder criterium including this one
        pub fn partly_superior_to(&self, other: &PathOption) -> bool {
            let comparable = (self.get_current_node() == other.get_current_node())
                && (self.fuel_level == other.fuel_level);
            let at_least_one_better = (self.total_distance < other.total_distance)
                || (self.total_time < other.total_time);
            return comparable && at_least_one_better;
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

        pub fn refuel(&mut self, truck: &Truck) {
            self.total_time += truck.get_minutes_for_refueling(self.fuel_level);
            self.fuel_level = truck.get_fuel();
        }
    }

    ///Represents the current state of the search. Each SearchState is at either a request node or a depot, AFS are covered in the `PathOption`s.
    pub struct SearchState {
        ///current node in the distance/time matrix
        current_node: usize,
        ///all the paths leading from the previous `SearchState` to `current_node`, empty path possible, indicates loading at depot
        path_options: Vec<Rc<PathOption>>,
        ///represents the containers the truck is carrying in this state
        container_data: ContainerData,
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
            path_options.push(Rc::new(PathOption {
                fuel_level,
                total_distance: 0,
                total_time: 0,
                path,
                previous_index: 0,
            }));
            let search_state = SearchState {
                current_node: 0,
                path_options,
                container_data: ContainerData::new(),
                requests_visisted: 0,
                previous_state: Option::None,
            };
            return Rc::new(search_state);
        }

        ///Creates the next state after `current_state` using `path_options`. Can't be called on `self` because the `Rc` to `self` is needed
        fn create_next_state_after_current_state(
            config: &Config,
            current_state: &Rc<SearchState>,
            path_options: Vec<Rc<PathOption>>,
        ) -> Rc<SearchState> {
            //this function should only be called when at least one path exists
            let current_node = path_options[0].get_current_node();
            let new_state_reference = Rc::clone(&current_state);
            let mut new_state = SearchState {
                current_node,
                path_options,
                container_data: current_state.container_data.clone(),
                requests_visisted: current_state.requests_visisted,
                previous_state: Option::Some(new_state_reference),
            };
            //do containers need to be changed/is the new node a request?
            if 0 < current_node && current_node < config.get_first_afs() {
                new_state.handle_request_containers(config);
            }
            return Rc::new(new_state);
        }

        ///Adjusts the containers at the request node the state is currently at and marks this request as visited afterwards.
        /// Must only be called when the current node is a request (unchecked) and the request has not been visited before (checked)
        fn handle_request_containers(&mut self, config: &Config) {
            //request not already handled:
            assert!(!self.get_request_visited(self.current_node));
            let request = config.get_request_at_node(self.current_node);
            if self.current_node <= config.get_full_pickup() {
                //full pickup request
                self.container_data.num_20 += request.full_20;
                self.container_data.num_40 += request.full_40;
                if self.container_data.full_request_1_source == 0 {
                    self.container_data.full_request_1_source = self.current_node;
                } else if self.container_data.full_request_2_source == 0 {
                    self.container_data.full_request_2_source = self.current_node;
                } else {
                    panic!("THIS SHOULD NOT HAPPEN!");
                }
            } else if self.current_node < config.get_first_full_dropoff() {
                //empty pickup request
                self.container_data.empty_20 += request.empty_20;
                self.container_data.empty_40 += request.empty_40;
                self.container_data.num_20 += request.empty_20;
                self.container_data.num_40 += request.empty_40;
            } else if self.current_node < config.get_first_full_dropoff() + config.get_full_pickup()
            {
                //full delivery
                let source_node = config.get_pick_node_for_full_dropoff(self.current_node);
                //values are negative, so this is effectively a subtraction
                self.container_data.num_20 += request.full_20;
                self.container_data.num_40 += request.full_40;
                if self.container_data.full_request_1_source == source_node {
                    self.container_data.full_request_1_source = 0;
                } else if self.container_data.full_request_2_source == source_node {
                    self.container_data.full_request_2_source = 0;
                } else {
                    panic!("THIS SHOULD NOT HAPPEN!");
                }
            } else if self.current_node < config.get_first_afs() {
                //empty delivery
                //can also just add this because the values are negative
                self.container_data.empty_20 += request.empty_20;
                self.container_data.empty_40 += request.empty_40;
                self.container_data.num_20 += request.empty_20;
                self.container_data.num_40 += request.empty_40;
            }
            self.set_request_visited(self.current_node);
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
            let initial_state;
            let depot_service;
            //handle special case where depot was loaded in the previous state
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
            //first step, use paths from current_state to go directly to node or fuel stations or depot
            for (option_index, option) in initial_state.path_options.iter().enumerate() {
                //target node
                let mut new_option =
                    option.next_path_option(config, truck, option_index, node, depot_service);
                SearchState::possibly_add_to_path_options(&mut path_options, new_option);
                //fuel stations
                for afs in config.get_first_afs()..config.get_dummy_depot() {
                    new_option =
                        option.next_path_option(config, truck, option_index, afs, depot_service);
                    SearchState::possibly_add_to_path_options(&mut path_options, new_option);
                }
                //depot for refueling only
                if current_state.current_node != 0 {
                    //not already at depot, navigate to depot normally first. no reason to save this temporary one
                    let depot_option =
                        match option.next_path_option(config, truck, option_index, 0, false) {
                            Option::None => continue, //cannot be reached, nothing more to to in this iteration
                            Option::Some(tmp) => tmp,
                        };
                    new_option = depot_option.refuel_at_depot(truck, option_index);
                } else {
                    new_option = option.refuel_at_depot(truck, option_index);
                }
                //add option where refueled at depot, new_option was set correctly before
                SearchState::possibly_add_to_path_options(&mut path_options, new_option);
            }
            //second step, try using previously found path_options to find a better path to somewhere (not only the depot)
            let mut improvement_found = true;
            let mut iteration_clone = Vec::with_capacity(vec_capacity);
            while improvement_found {
                improvement_found = false;
                //iterate over efficient copy of path_options because compiler complains otherwise
                iteration_clone.retain(|_| false);
                for index in 0..path_options.len() {
                    let option = Rc::clone(&path_options[index]);
                    iteration_clone.push(option);
                }
                for (option_index, option) in iteration_clone.iter().enumerate() {
                    //save for repeated usage:
                    let current_node = option.get_current_node();
                    //to target node
                    if option.get_current_node() != node {
                        improvement_found |= SearchState::possibly_add_to_path_options(
                            &mut path_options,
                            option.next_path_option(config, truck, option_index, node, false),
                        );
                    } else {
                        //refueling, makes no sense when already at target node
                        //at AFS
                        for afs in config.get_first_afs()..config.get_dummy_depot() {
                            //routing from itself to itself is completely pointless
                            if current_node != afs {
                                improvement_found |= SearchState::possibly_add_to_path_options(
                                    &mut path_options,
                                    option.next_path_option(
                                        config,
                                        truck,
                                        option_index,
                                        afs,
                                        false,
                                    ),
                                );
                            }
                        }
                        //at depot
                        if current_state.current_node != 0 {
                            //not already at depot, navigate to depot normally first, then refuel, tmp_option discarded
                            let tmp_option = match option.next_path_option(
                                config,
                                truck,
                                option_index,
                                0,
                                false,
                            ) {
                                Option::None => continue, //cannot be reached, nothing to to in this iteration
                                Option::Some(tmp) => tmp,
                            };
                            improvement_found |= SearchState::possibly_add_to_path_options(
                                &mut path_options,
                                tmp_option.refuel_at_depot(truck, option_index),
                            );
                        } else {
                            //already at depot
                            improvement_found |= SearchState::possibly_add_to_path_options(
                                &mut path_options,
                                option.refuel_at_depot(truck, option_index),
                            );
                        }
                    }
                }
            }
            //third step, remove anything that does not end at the target node
            path_options.retain(|option| option.get_current_node() == node);
            if path_options.len() == 0 {
                //no path found
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
                //check whether unpacked_option is at least partially superior to one of the existing ones
                for i in 0..path_options.len() {
                    if unpacked_option.partly_superior_to(&path_options[i]) {
                        path_options.push(Rc::new(unpacked_option));
                        return true;
                    }
                }
                return false;
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
            assert!(self.container_data.num_20 <= truck.get_num_20_foot_containers());
            assert!(self.container_data.num_40 <= truck.get_num_40_foot_containers());
            //cannot visit the same request twice
            if self.get_request_visited(request_node) {
                return false;
            }
            let request = config.get_request_at_node(request_node);
            //only to need to check whether something can be picked up
            if request_node < config.get_first_full_dropoff() {
                //pickup request
                let newly_loaded_20 = request.empty_20 + request.full_20;
                if newly_loaded_20 > 0 {
                    if (newly_loaded_20 + self.container_data.num_20)
                        > truck.get_num_20_foot_containers()
                    {
                        return false;
                    }
                }
                let newly_loaded_40 = request.empty_40 + request.full_40;
                if newly_loaded_40 > 0 {
                    if newly_loaded_40 + self.container_data.num_40
                        > truck.get_num_40_foot_containers()
                    {
                        return false;
                    }
                }
                assert!(newly_loaded_20 + newly_loaded_40 > 0);
            } else if request_node >= config.get_first_full_dropoff() {
                //dropoff
                if request_node >= config.get_first_full_dropoff() + config.get_full_pickup() {
                    //full container dropoof
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
            } else {
                panic!("THIS CASE SHOULD NEVER BE REACHED!");
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
            change_20: i32,
            change_40: i32,
        ) -> bool {
            let new_20 = self.container_data.num_20 + change_20;
            let new_40 = self.container_data.num_40 + change_40;
            //easy check: can that many containers be loaded at all?
            if (new_20 < 0)
                || (new_20 > truck.get_num_20_foot_containers())
                || (new_40 < 0)
                || (new_40 > truck.get_num_40_foot_containers())
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
            {
                return false;
            }
            return false;
        }

        ///Creates the next state after `current_state` where nothing has been changed except hat `change_20` empty 20-foot containers (can be negative) have been loaded.
        /// Same for `change_40`. `path_options` is empty because a) there is nothing for it anyway and b) this indicates that something was loaded in this state
        pub fn load_at_depot(
            config: &Config,
            current_state: &Rc<SearchState>,
            change_20: i32,
            change_40: i32,
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
            return Rc::new(new_state);
        }
    }

    ///Combines all the data about the current state of loaded containers for a `SearchState`
    #[derive(Copy, Clone)]
    struct ContainerData {
        ///number of empty 20 foot containers loaded
        empty_20: i32,
        ///number of empty 40 foot containers loaded
        empty_40: i32,
        ///number of 20 foot containers (empty + full) loaded. Saving it this way saves computations
        num_20: i32,
        ///number of 40 foot containers (empty + full) loaded. Saving it this way saves computations
        num_40: i32,
        ///if a full container is loaded, its origin is saved here or in `full_request_2_source`. 0 indicates nothing saved at the moment
        full_request_1_source: usize,
        ///see `full_request_1_source`
        full_request_2_source: usize,
    }

    impl ContainerData {
        ///Creates a new `ContainerData` representing nothing currently carried
        pub fn new() -> ContainerData {
            return ContainerData {
                empty_20: 0,
                empty_40: 0,
                num_20: 0,
                num_40: 0,
                full_request_1_source: 0,
                full_request_2_source: 0,
            };
        }
    }

    pub struct EmptyContainersStillNeeded {
        ///Number of empty 20-foot containers that still need to be delivered
        empty_20_delivery: i32,
        ///Number of empty 40-foot containers that still need to be delivered
        empty_40_delivery: i32,
    }
}
use solver_data::*;
use std::rc::Rc;

pub fn solve(config: Config) -> Solution {
    let mut all_known_options = AllKnownOptions::new();
    let current_truck = config.get_truck(0);
    println!("Calculating options for truck 0 ...");
    let mut options_for_truck = solve_for_truck(&config, 0);
    all_known_options.inital_merge(&options_for_truck);
    for truck_index in 1..config.get_num_trucks() {
        //avoid unnecessary recalculation of options_for_truck
        if config.get_truck(truck_index) != current_truck {
            println!("Calculating options for truck {} ...", truck_index);
            options_for_truck = solve_for_truck(&config, truck_index);
        } else {
            println!(
                "Truck {} is the same as the one before, no calculation necessary.",
                truck_index
            );
        }
        all_known_options.additional_merge(&options_for_truck);
    }
    return Solution {};
}

///Calculates all the known options for truck at given index
fn solve_for_truck(config: &Config, truck_index: usize) -> KnownRoutesForTruck {
    let truck = config.get_truck(truck_index);
    let root_state = SearchState::start_state(truck.get_fuel());
    let mut known_options = KnownRoutesForTruck::new();
    solve_for_truck_recursive(config, &truck, &mut known_options, &root_state);
    return known_options;
}

///Represents all the possible loadings done at the depot
/// First number is addition of empty_20 containers, second of empty_40 containers
static POSSIBLE_DEPOT_LOADS: &'static [(i32, i32)] = &[
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

fn solve_for_truck_recursive(
    config: &Config,
    truck: &Truck,
    known_options: &mut KnownRoutesForTruck,
    current_state: &Rc<SearchState>,
) {
    //dummy depot is never routed to internally, there is no reason to differentiate between it and the original depot
    assert!(current_state.get_current_node() != config.get_dummy_depot());
    if current_state.get_current_node() == 0 {
        known_options.possibly_add(&current_state);
    }
    //depot, handled first because if it cannot be reached the rest is pointless anyway
    if current_state.get_current_node() == 0 {
        //loading at the depot is always done in a separate state after navigating to the depot. This prevents repeated identical routing and makes parsing the route easier
        //only do this when the depot has not been loaded in the current_state already, otherwise infinite branching would result
        if !current_state.was_depot_loaded() {
            //calculate only once
            let containers_needed = current_state.get_containers_still_needed(config);
            //some combinations are always nonsene, so these are not included in the predefined array
            for (change_20, change_40) in POSSIBLE_DEPOT_LOADS {
                if current_state.can_handle_depot_load(
                    truck,
                    &containers_needed,
                    *change_20,
                    *change_40,
                ) {
                    //create new state where the loading has been applied. Due to the above condition, no loading will happen immadiately afterwards
                    let next_state =
                        SearchState::load_at_depot(config, &current_state, *change_20, *change_40);
                    solve_for_truck_recursive(config, truck, known_options, &next_state)
                }
            }
        }
        //if already at the depot and loaded, has to move somewhere else and the only option for this are requests
    } else {
        //not already at depot, try routing to it. If it cannot be reached, the route can't be finished so this search may already be interrupted
        let possible_depot_state = SearchState::route_to_node(config, truck, &current_state, 0);
        match possible_depot_state {
            Option::None => return,
            Option::Some(state) => solve_for_truck_recursive(config, truck, known_options, &state),
        };
    };
    //try moving to the requests
    for request_node in 1..config.get_first_afs() {
        if current_state.can_handle_request(config, truck, request_node) {
            let next_state =
                SearchState::route_to_node(config, truck, &current_state, request_node);
            match next_state {
                Option::None => (),
                Option::Some(state) => {
                    solve_for_truck_recursive(config, truck, known_options, &state)
                }
            };
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
        let next_state = SearchState::route_to_node(&config, truck, &base_state, 1);
    }
}
