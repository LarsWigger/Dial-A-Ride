use crate::data::Config;
use crate::data::Solution;
use crate::data::Truck;

mod solver_data {
    use crate::data::Config;
    use crate::data::Truck;
    use std::collections::HashMap;
    use std::rc::Rc;

    ///Represents the route a single truck could take.
    pub struct Route {
        ///the nodes taken by the route, saved as u8 to save memory, order backwards to avoid reverting every single route
        reverse_path: Vec<u8>,
        ///summary metric for comparing different routes
        total_distance: u32,
    }

    impl Route {
        ///Creates a complete `Route` starting from `search_state`s path with index `path_index` and iterates over the previous_states until there is no `previous_state`
        pub fn new(search_state: &Rc<SearchState>, path_index: usize) -> Route {
            //setup for iteration
            let mut current_state = search_state;
            let mut current_path_index;
            let mut current_path = search_state.get_path(path_index);
            //save total_distance for later
            let total_distance = current_path.total_distance;
            //calculate size of vector first to avoid either time for reallocation or waste of memory - there will be A LOT of routes at once!
            let mut vec_size = 0;
            loop {
                vec_size += current_path.path.len();
                current_path_index = current_path.previous_index;
                match &current_state.previous_state {
                    Option::None => break,
                    Option::Some(state) => {
                        current_state = &state;
                        current_path = current_state.get_path(current_path_index)
                    }
                }
            }
            let mut reverse_path = Vec::with_capacity(vec_size);
            //reset variables that were changed and will be read before being overwritten
            current_state = search_state;
            current_path = current_state.get_path(path_index);
            //fill up the reverse_path
            loop {
                current_path_index =
                    Route::add_path_to_reverse_path(&mut reverse_path, current_path);
                match &current_state.previous_state {
                    Option::None => break,
                    Option::Some(state) => {
                        current_state = &state;
                        current_path = current_state.get_path(current_path_index)
                    }
                }
            }
            assert!(vec_size == reverse_path.len());
            return Route {
                reverse_path,
                total_distance,
            };
        }

        ///Helper function that adds `path_option` to `reverse_path` and returns the index of the next `PathOption` to be processed
        fn add_path_to_reverse_path(reverse_path: &mut Vec<u8>, path_option: &PathOption) -> usize {
            for index in (0..path_option.path.len()).rev() {
                reverse_path.push(path_option.path[index] as u8);
            }
            return path_option.previous_index;
        }
    }
    ///Represents all the known routes for a single truck
    pub struct KnownRoutesForTruck {
        ///maps the `requests_visited` bits to the best corresponding route known (so far). Route is behind smart pointer to copying later on
        map: HashMap<u64, Rc<Route>>,
    }

    impl KnownRoutesForTruck {
        ///Creates a new `KnownRoutesForTruck`
        pub fn new() -> KnownRoutesForTruck {
            //TODO: use faster hasher
            let map = HashMap::new();
            return KnownRoutesForTruck { map };
        }

        ///Adds the best route from the given `search_state` if:
        /// - there are no full containers currently loaded in `SearchState`
        /// - there is no other known route for `search_state.requests_visited` yet
        /// - the route in `search_state` for its `requests_visited` is better than the one already known
        pub fn possibly_add(&mut self, search_state: &Rc<SearchState>) {
            //check whether any full containers have been picked up but not delivered
            //this could not result in any proper result anyway, so we may as well prevent it here
            if (search_state.container_data.num_20 - search_state.container_data.empty_20 > 0)
                || (search_state.container_data.num_40 - search_state.container_data.empty_40 > 0)
            {
                return;
            }

            let requests_visited = search_state.requests_visisted;
            let (best_path_index, total_distance) = search_state.get_total_distance();
            let previous_entry = self.map.get(&requests_visited);
            let save_or_overwrite;
            let new_route;
            match previous_entry {
                Option::None => {
                    save_or_overwrite = true;
                }
                Option::Some(previous_route) => {
                    save_or_overwrite = total_distance < previous_route.total_distance
                }
            };
            if save_or_overwrite {
                new_route = Route::new(search_state, best_path_index);
                self.map.insert(requests_visited, Rc::new(new_route));
            }
        }
    }

    ///Combines up to `config.num_trucks` routes, each corresponding to the truck at the respective index,
    /// while also saving the summary metric of the `total_distance` of these routes
    struct CombinationOption {
        ///summary metric, sum of the distance of the individual routes
        total_distance: u32,
        ///the different routes, each route is for the truck at the same index
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

        ///just copy references to all the routes into `option_map` while adjusting/initializing the slightly different format
        pub fn inital_merge(&mut self, truck_options: &KnownRoutesForTruck) {
            for (key, value) in &truck_options.map {
                //it gets thrown away anyway, not need for more difficult allocation. Essentially just for compatibility
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
            //since this is created completely anew and not jsut copied, all values will be for the same number of vehicles
            let mut new_map = HashMap::new();
            for (own_key, own_value) in &self.option_map {
                for (other_key, other_value) in &truck_options.map {
                    //only if there is no clear conflict between these two routes
                    //meaning, no request visited by both
                    if own_key & other_key == 0 {
                        let combined_key = own_key | other_key;
                        let combined_distance =
                            own_value.total_distance + other_value.total_distance;
                        let alternative = self.option_map.get(&combined_key);
                        let insert_into_map;
                        match alternative {
                            Option::Some(old_value) => {
                                insert_into_map = old_value.total_distance > combined_distance
                            }
                            Option::None => insert_into_map = true,
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
            depot_refuel: bool,
        ) -> Option<PathOption> {
            let from = self.path[self.path.len() - 1];
            //only call depot_refuel when already at a depot
            assert!(!depot_refuel || from == 0);
            let total_distance = config.get_distance_between(from, to);
            let fuel_needed = config.fuel_needed_for_route(from, to);
            if fuel_needed > self.fuel_level {
                return None;
            }
            let mut fuel_level = self.fuel_level - fuel_needed;
            //deal with handling and refueling times
            let mut total_time = self.total_time + config.get_time_between(from, to);
            //request handling times
            if to < config.get_first_afs() {
                if to != 0 {
                    //can request still be processed?
                    if total_time > config.get_latest_visiting_time(to) {
                        return None;
                    } else if total_time < config.get_earliest_visiting_time(to) {
                        //have to wait
                        total_time = config.get_earliest_visiting_time(to)
                    }
                    total_time += config.get_service_time(to);
                } else {
                    //depot handling times
                    //TODO!
                }
            }
            //create new path
            let new_path_len = self.path.len() + 1;
            let mut path: Vec<usize> = Vec::with_capacity(new_path_len);
            for i in 0..self.path.len() {
                path.push(self.path[i]);
            }
            path.push(to);
            //create new option before
            let mut new_option = PathOption {
                fuel_level,
                total_distance,
                total_time,
                path,
                previous_index,
            };
            //refuel if at AFS or at the depot if this is intended
            if config.get_first_afs() <= to && to < config.get_dummy_depot()
                || (to == 0 && depot_refuel)
            {
                new_option.refuel(truck);
            }
            return Some(new_option);
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
            let comparable = (self.path[self.path.len() - 1] == other.path[other.path.len() - 1])
                && (self.fuel_level == other.fuel_level);
            let at_least_one_better = (self.total_distance < other.total_distance)
                || (self.total_time < other.total_time);
            return comparable && at_least_one_better;
        }

        ///Returns the node this `PathOption` is currently at
        pub fn get_current_node(&self) -> usize {
            return self.path[self.path.len() - 1];
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
        ///all the paths leading from the previous `SearchState` to `current_node`, sorted
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

        /// Calculates all the relevant routes from `current_state` to the target node and returns the next `SearchState` already wrapped in `Rc`
        /// This may be `Option::None` if no route is found
        pub fn route_to_node(
            config: &Config,
            truck: &Truck,
            current_state: &Rc<SearchState>,
            node: usize,
        ) -> Option<Rc<SearchState>> {
            assert!(current_state.current_node != node);
            //it is not predictable in advance how big this will get, so I just initialize it with 20 (probably bad)
            //TODO: calculate a more realistic size, there is a maximum number of elements that can be calculated
            let mut path_options = Vec::with_capacity(20);
            //first step, use previous ones to go directly to node or fuel stations or depot
            for (index, option) in current_state.path_options.iter().enumerate() {
                //target node
                let mut new_option = option.next_path_option(config, truck, index, node, false);
                SearchState::possibly_add_to_path_options(&mut path_options, new_option);
                //fuel stations
                for afs in config.get_first_afs()..config.get_first_afs() + config.get_afs() {
                    new_option = option.next_path_option(config, truck, index, afs, false);
                    SearchState::possibly_add_to_path_options(&mut path_options, new_option);
                }
                //depot (only for refueling, needs to appear at least twice in a row in the path)
                if current_state.current_node != 0 {
                    //navigate to depot normally first. Two zeros following each other represent a refuel, needed for clarity
                    //if we would go straight to the depot and refuel there would be only one
                    let tmp_option =
                        match option.next_path_option(config, truck, index, node, false) {
                            Option::None => continue, //cannot be reached, nothing to to in this iteration
                            Option::Some(opt) => opt,
                        };
                    //now the one with refueling
                    new_option = tmp_option.next_path_option(config, truck, index, node, true);
                    SearchState::possibly_add_to_path_options(&mut path_options, new_option);
                } else {
                    //already at depot
                    new_option = option.next_path_option(config, truck, index, node, true);
                    SearchState::possibly_add_to_path_options(&mut path_options, new_option);
                }
            }
            //second step, try using previously found path_options one to find a better path to somewhere (not only the depot)
            let mut improvement_found = true;
            let mut iteration_clone = Vec::with_capacity(20);
            while improvement_found {
                improvement_found = false;
                //compiler complains and this is probably cheaper than copying all the PathOptions
                iteration_clone.retain(|_| false);
                for index in 0..path_options.len() {
                    let option = Rc::clone(&path_options[index]);
                    iteration_clone.push(option);
                }
                for (index, option) in iteration_clone.iter().enumerate() {
                    //save for repeated usage:
                    let current_node = option.get_current_node();
                    //to target node
                    if option.get_current_node() != node {
                        let new_option = option.next_path_option(config, truck, index, node, false);
                        improvement_found |= SearchState::possibly_add_to_path_options(
                            &mut path_options,
                            new_option,
                        );
                    }
                    //refueling, makes no sense starting from the target node
                    if current_node != node {
                        //to afs
                        for afs in
                            config.get_first_afs()..(config.get_first_afs() + config.get_afs())
                        {
                            //routing from itself to itself is completely pointless
                            if current_node != afs {
                                let new_option =
                                    option.next_path_option(config, truck, index, node, false);
                                improvement_found |= SearchState::possibly_add_to_path_options(
                                    &mut path_options,
                                    new_option,
                                );
                            }
                        }
                        //to depot
                        //this may be called when already at the depot. In that case, depot will appear twice in the path, but the result will be refueled
                        let new_option = option.next_path_option(config, truck, index, node, true);
                        improvement_found |= SearchState::possibly_add_to_path_options(
                            &mut path_options,
                            new_option,
                        );
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
        /// and adds `new_option` if it was partially superior to at least one of the previous elements
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
        pub fn get_total_distance(&self) -> (usize, u32) {
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

        ///Returns `true` if the state can handle the request next (without visiting a different request first)
        pub fn can_handle_request(
            &self,
            config: &Config,
            truck: &Truck,
            request_node: usize,
        ) -> bool {
            assert!(self.container_data.num_20 <= truck.get_num_20_foot_containers());
            assert!(self.container_data.num_40 <= truck.get_num_40_foot_containers());
            //TODO: change container system to only allow one change per pickup, validating it in the data
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

        ///Get the `PathOption` at the given `index`
        fn get_path(&self, index: usize) -> &PathOption {
            return &self.path_options[index];
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

fn solve_for_truck_recursive(
    config: &Config,
    truck: &Truck,
    known_options: &mut KnownRoutesForTruck,
    current_state: &Rc<SearchState>,
) {
    //routes can end only at the dummy depot
    if current_state.get_current_node() == config.get_dummy_depot() {
        known_options.possibly_add(&current_state);
        return;
    }
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
    //try changing containers at depot
    //optimization potential: do this at the beginning, if the depot cannot be reached, this state is a dead end
    //if necessary, navigate to the depot
    let depot_state;
    if current_state.get_current_node() == 0 {
        depot_state = current_state;
    } else {
        let possible_depot_state = SearchState::route_to_node(config, truck, &current_state, 0);
        match possible_depot_state {
            Option::None => return,
            Option::Some(state) => depot_state = &state,
        };
    };
    //TODO: navigate to dummy depot only when no full containers are loaded, remove unnecessary check (no, it is not unnecessary, might be the start depot)
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
