use crate::data::Config;
use crate::data::Solution;
use crate::data::Truck;

mod solver_data {
    use crate::data::*;
    use std::collections::HashMap;
    use std::rc::Rc;
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
            let path = Route::new_path_recursive(search_state, path_index, std::usize::MAX, 0);
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
            current_container_index: usize,
            current_size: usize,
        ) -> Vec<u8> {
            match &current_state.previous_state {
                Option::Some(previous_state) => {
                    if (current_container_index != std::usize::MAX
                        && current_state.get_current_node() == 0)
                        && current_state.container_options.len() != 0
                    {
                        //calculate how many containers were exchanged
                        let current_container_option =
                            &current_state.container_options[current_container_index];
                        let previous_container_option = &previous_state.container_options
                            [current_container_option.previous_index];
                        let diff_empty_20 =
                            current_container_option.empty_20 - previous_container_option.empty_20;
                        let diff_empty_40 =
                            current_container_option.empty_40 - previous_container_option.empty_40;
                        //path_option
                        let current_path_option =
                            &current_state.path_options[current_state_path_index];
                        let size_at_this_step = ((diff_empty_20.abs() + diff_empty_40.abs())
                            as usize)
                            + current_path_option.path.len();
                        //recursion
                        let mut path = Route::new_path_recursive(
                            previous_state,
                            current_path_option.previous_index,
                            current_container_option.previous_index,
                            current_size + size_at_this_step,
                        );
                        let len_before = path.len(); //sanity check
                                                     //path
                        for node in &current_path_option.path {
                            path.push(*node as u8);
                        }
                        //loading
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
                        if path.len() != len_before + size_at_this_step {
                            println!("FOUND");
                        }
                        assert_eq!(path.len(), len_before + size_at_this_step);
                        return path;
                    } else {
                        //not at depot, only need to consider the PathOption
                        let current_path_option =
                            &current_state.path_options[current_state_path_index];
                        let size_at_this_step = current_path_option.path.len();
                        let next_container_index;
                        if current_container_index == std::usize::MAX {
                            next_container_index = 0;
                        } else {
                            next_container_index = current_state.container_options
                                [current_container_index]
                                .previous_index;
                        }
                        let mut path = Route::new_path_recursive(
                            previous_state,
                            current_path_option.previous_index,
                            next_container_index,
                            current_size + size_at_this_step,
                        );
                        let len_before = path.len(); //sanity check
                        for node in &current_path_option.path {
                            path.push(*node as u8);
                        }
                        assert_eq!(path.len(), len_before + size_at_this_step);
                        return path;
                    }
                }
                Option::None => {
                    if current_container_index != std::usize::MAX {
                        let starting_container_option =
                            &current_state.container_options[current_container_index];
                        let num_containers_loaded =
                            starting_container_option.empty_20 + starting_container_option.empty_40;
                        //the original state will always have only 1 path option: 0
                        let mut path =
                            Vec::with_capacity(current_size + 1 + (num_containers_loaded as usize));
                        path.push(0);
                        for _ in 0..starting_container_option.empty_20 {
                            path.push(ROUTE_DEPOT_LOAD_20);
                        }
                        for _ in 0..starting_container_option.empty_40 {
                            path.push(ROUTE_DEPOT_LOAD_40);
                        }
                        return path;
                    } else {
                        //special case to handle the initial SearchState where current_container_index is undefined upon reaching this case
                        let mut path = Vec::with_capacity(current_size + 1);
                        path.push(0);
                        return path;
                    }
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
            self.valid_insertions += 1;
            let requests_visited = search_state.requests_visited;
            let (best_path_index, total_distance) =
                search_state.get_best_path_index_and_total_distance();
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

    ///Summarizes a `PathOption` for comparisons using these three values that are relevant:
    /// - `fuel_level`: The fuel level of the vehicle at the last node. All else being equal, higher is better as that meaens:
    ///     - less additional stops needed (or at least no difference/disadvantage)
    ///     - less time taken when refueling later and less time is always an advantage
    /// - `total_distance`: The total distance travelled by the vehicle at the last node. All else being equal, lower is always better.
    /// - `total_time`: The total time, might include waiting and fueling times. All else being equal, lower is always better. If too early, waiting is still possible
    pub struct PathOptionSummary {
        ///the fuel level of the vehicle at the last node, in 0.01l to avoid floating point operations
        fuel_level: u32,
        ///the total distance travelled by the vehicle at the last node
        total_distance: u32,
        ///the total
        total_time: u32,
        ///the `n`th bit from the right represents whether the `ContainerOption` at index `n` is compatible with this `PathOption`
        compatible_container_options: u32,
    }

    impl PathOptionSummary {
        ///Returns `true? if this summary is superior to the other in at least one regard.
        /// This method may be called on another summary is not even comparable in context.
        pub fn partly_superior_to(&self, other: &PathOptionSummary) -> bool {
            return (self.total_distance < other.total_distance)
                || (self.total_time < other.total_time)
                || (self.fuel_level > other.fuel_level)
                //there is at least one option covered by self that is not covered by other
                || !other.covers_completely(self);
        }
        ///Returns `true` if this summary is not worse than the other in any regard
        pub fn at_least_as_good_as(&self, other: &PathOptionSummary) -> bool {
            return (self.total_distance <= other.total_distance)
                && (self.total_time <= other.total_time)
                && (self.fuel_level >= other.fuel_level)
                && self.covers_completely(other);
        }
        ///Returns true if `self` is compatible with every `ContainerOption` that `other` is compatible with
        fn covers_completely(&self, other: &PathOptionSummary) -> bool {
            let difference = self.compatible_container_options ^ other.compatible_container_options;
            return (difference & other.compatible_container_options) == 0;
        }

        //Creates a new `PathOptionSummary` that expands itself (`self`) to `to`.
        /// Takes request service and visiting times into account as well as refueling times.
        /// If `depot_refuel==true`, the truck is refueled when arriving at the depot.
        /// This method may be called when the truck is already at the depot, there is no disadvantage to doing so
        /// - since there is not travel, nothing changes except the refuel.
        /// Returns `None` if there is no possible path, which may be if:
        /// - fuel is insufficient
        /// - arrival time is too late (regarding either request visiting time or t_max)
        pub fn next_summary(
            &self,
            config: &Config,
            truck: &Truck,
            from: usize,
            to: usize,
            depot_refuel: bool,
            compatible_container_options: u32,
            loading_distance: u32,
        ) -> Option<PathOptionSummary> {
            let additional_distance = config.get_distance_between(from, to);
            //calculate fuel_level on arrival
            let fuel_needed = config.get_fuel_needed_for_distance(additional_distance);
            if fuel_needed > self.fuel_level {
                //cannot reach destination, no further calculation needed
                return Option::None;
            }
            let mut fuel_level = self.fuel_level - fuel_needed;
            //calculate new total_distance
            let total_distance = self.total_distance + additional_distance;
            //calculate new total_time, dealing with handling and refueling times
            let mut total_time = self.total_time + config.get_time_between(from, to);
            //service time, always applied first, loading_distance only != 0 for fill_with_previous_path_options
            total_time += config.get_subsequent_depot_service_time() * loading_distance;
            //request handling times
            if to < config.get_first_afs() {
                //not an AFS, either depot or
                if to != 0 {
                    if total_time > config.get_latest_visiting_time_at_request_node(to) {
                        //too late, impossible
                        return Option::None;
                    } else if total_time < config.get_earliest_visiting_time_at_request_node(to) {
                        //too early, just wait
                        total_time = config.get_earliest_visiting_time_at_request_node(to)
                    }
                    total_time += config.get_service_time_at_request_node(to);
                } else if depot_refuel {
                    total_time += truck.get_minutes_for_refueling(self.fuel_level);
                    fuel_level = truck.get_fuel();
                }
            } else {
                //AFS
                total_time += truck.get_minutes_for_refueling(self.fuel_level);
                fuel_level = truck.get_fuel();
            }
            //t_max applies to every type of node, including AFS and the depot
            if total_time > config.get_t_max() {
                return Option::None;
            }
            return Option::Some(PathOptionSummary {
                total_distance,
                total_time,
                fuel_level,
                compatible_container_options,
            });
        }
    }

    ///Represents a single PathOption. When navigating between two nodes, stops at fuel stations might be necessary or optional.
    /// The different paths can be compared solely on the values contained in `PathOptionSummary`.
    /// Comparisons between paths ending at different nodes are obviously pointless.
    /// The concrete path is not relevant for the comparison of different paths, these three summary values describe it completely.
    struct PathOption {
        ///the summary of the values, saved separately to avoid unnecessary path allocations
        summary: PathOptionSummary,
        ///the nodes traversed in this path
        path: Vec<usize>,
        ///the index of the previous `PathOption` this one uses as a base
        previous_index: usize,
    }

    impl PathOption {
        ///Returns `true` if `self` is inferior to the other `PathOptionSummary` at `node`.
        fn completely_inferior_to(&self, other: &PathOptionSummary, node: usize) -> bool {
            return (self.get_current_node() == node)
                && (other.partly_superior_to(&self.summary))
                && (other.at_least_as_good_as(&self.summary));
        }

        ///Create the complete `PathOption` after `self` using the given parameters.
        fn create_next_option(
            &self,
            own_index: usize,
            new_summary: PathOptionSummary,
            node: usize,
            depot_refuel: bool,
            new_path: bool,
        ) -> Rc<PathOption> {
            //create the new path
            let mut path = Vec::with_capacity(self.path.len() + 1);
            let last_node;
            if new_path {
                //there are no elements already in the path, so take the one from self
                last_node = self.get_current_node();
            } else {
                //copy the previous path and save for later
                for i in 0..self.path.len() {
                    path.push(self.path[i]);
                }
                last_node = path[path.len() - 1]
            };
            if depot_refuel {
                if last_node != 0 {
                    //not already at depot, so push the depot node first
                    path.push(0);
                }
                path.push(ROUTE_DEPOT_REFUEL);
            } else {
                path.push(node);
            }
            return Rc::new(PathOption {
                summary: new_summary,
                path,
                previous_index: own_index,
            });
        }

        ///Removes the elements of `path_options` that are completely inferior to `new_option`
        /// and adds `new_option` if it was partially superior to at least one of the previous elements.
        fn possibly_add_to_path_options(
            &self,
            own_index: usize,
            path_options: &mut Vec<Rc<PathOption>>,
            new_summary: Option<PathOptionSummary>,
            node: usize,
            depot_refuel: bool,
            new_path: bool,
        ) {
            let new_summary = match new_summary {
                Option::None => return,
                Option::Some(x) => x,
            };
            //to detect whether something was removed
            let original_length = path_options.len();
            //remove the entries that are completely inferior to the new one (CAN THIS EVEN HAPPEN?)
            path_options.retain(|option| !option.completely_inferior_to(&new_summary, node));
            if original_length != path_options.len() {
                //something was removed => completely superior to something => insert, done
                path_options.push(self.create_next_option(
                    own_index,
                    new_summary,
                    node,
                    depot_refuel,
                    new_path,
                ));
            } else {
                //check whether there is a reason against adding the new_option
                for i in 0..path_options.len() {
                    let comp_option = &path_options[i];
                    if (comp_option.get_current_node() == node)
                        && (comp_option.summary.at_least_as_good_as(&new_summary))
                    {
                        return;
                    }
                }
                //no reason against insertion found
                path_options.push(self.create_next_option(
                    own_index,
                    new_summary,
                    node,
                    depot_refuel,
                    new_path,
                ));
            }
        }

        ///Decodes the binary encoded mask, returning `true` if this `index` is set to 1/true
        fn decode_loading_mask(mask: u32, index: usize) -> bool {
            let index_mask = 1 << index;
            return (mask & index_mask) != 0;
        }

        //Sets the `index` in `mask` to false/0
        fn remove_index_from_mask(mask: u32, index: usize) -> u32 {
            let index_mask = 1 << index;
            return (!index_mask) & mask;
        }

        ///Helper function to separate code. Fills `path_options` with the ones that can be reached from the ones in `current_state`.
        /// The paths from `current_state` are not copied as they
        fn fill_with_path_options_from_previous_state(
            config: &Config,
            truck: &Truck,
            current_state: &Rc<SearchState>,
            node: usize,
            path_options: &mut Vec<Rc<PathOption>>,
            container_options_at_node: &Vec<ContainerOption>,
        ) {
            let loading_array = SearchState::get_container_masks(container_options_at_node);
            for loading_distance in (0_u32..3).rev() {
                if loading_array[loading_distance as usize] == 0 {
                    //nothing requires this loading distance anyway, so no need to even consider this case
                    continue;
                }
                //represents all the container_options that would be compatible (loading_distance and lower)
                let mut base_key = 0;
                for i in 0..(loading_distance + 1) {
                    base_key |= loading_array[i as usize];
                }
                for (option_index, option) in current_state.path_options.iter().enumerate() {
                    let from = option.get_current_node();
                    //remove the ones not compatible with this specific PathOption
                    let mut compatible_container_options = base_key; //done to avoid recalculating base_key
                    for container_index in 0..container_options_at_node.len() {
                        if PathOption::decode_loading_mask(
                            compatible_container_options,
                            container_index,
                        ) && !PathOption::decode_loading_mask(
                            option.summary.compatible_container_options,
                            container_options_at_node[container_index].previous_index,
                        ) {
                            //option would be compatible with this loading_distance,
                            //but the PathOption used as a starting point is not compatible with the previous ContainerOption
                            //so set the value for this ContainerOption to false
                            compatible_container_options = PathOption::remove_index_from_mask(
                                compatible_container_options,
                                container_index,
                            );
                        }
                    }
                    //routing options
                    //to target node
                    option.possibly_add_to_path_options(
                        option_index,
                        path_options,
                        option.summary.next_summary(
                            config,
                            truck,
                            from,
                            node,
                            false,
                            compatible_container_options,
                            loading_distance,
                        ),
                        node,
                        false,
                        true,
                    );
                    //fuel stations
                    for afs in config.get_first_afs()..config.get_dummy_depot() {
                        option.possibly_add_to_path_options(
                            option_index,
                            path_options,
                            option.summary.next_summary(
                                config,
                                truck,
                                from,
                                afs,
                                false,
                                compatible_container_options,
                                loading_distance,
                            ),
                            afs,
                            false,
                            true,
                        );
                    }
                    //depot refueling
                    option.possibly_add_to_path_options(
                        option_index,
                        path_options,
                        option.summary.next_summary(
                            config,
                            truck,
                            from,
                            0,
                            true,
                            compatible_container_options,
                            loading_distance,
                        ),
                        0,
                        true,
                        true,
                    );
                }
                if config.get_initial_depot_service_time() == 0 {
                    //speedup in case the other loops would be effectively identical/yield only inferior results
                    return;
                }
            }
        }

        ///Extends the paths already in `path_options` trying to route to `node`
        fn fill_with_following_paths(
            config: &Config,
            truck: &Truck,
            node: usize,
            path_options: &mut Vec<Rc<PathOption>>,
        ) {
            let mut iteration_queue = Vec::with_capacity(path_options.capacity());
            for index in 0..path_options.len() {
                let option = Rc::clone(&path_options[index]);
                iteration_queue.push(option);
            }
            while iteration_queue.len() != 0 {
                let option = iteration_queue.swap_remove(0);
                if Rc::strong_count(&option) == 1 {
                    //this is the only remaining pointer, meaning that it was removed from path_options. No reason to continue with this element
                    continue;
                }
                //save for repeated usage:
                let current_node = option.get_current_node();
                //going away from the target node is both forbidden and pointless
                if current_node != node {
                    //straight to target node
                    option.possibly_add_to_path_options(
                        option.previous_index,
                        path_options,
                        option.summary.next_summary(
                            config,
                            truck,
                            current_node,
                            node,
                            false,
                            option.summary.compatible_container_options,
                            0,
                        ),
                        node,
                        false,
                        false,
                    );
                    //refuel at a particular AFS
                    for afs in config.get_first_afs()..config.get_dummy_depot() {
                        //routing from itself to itself is completely pointless
                        if current_node != afs {
                            option.possibly_add_to_path_options(
                                option.previous_index,
                                path_options,
                                option.summary.next_summary(
                                    config,
                                    truck,
                                    current_node,
                                    afs,
                                    false,
                                    option.summary.compatible_container_options,
                                    0,
                                ),
                                afs,
                                false,
                                false,
                            );
                        }
                    }
                    //refuel at depot
                    option.possibly_add_to_path_options(
                        option.previous_index,
                        path_options,
                        option.summary.next_summary(
                            config,
                            truck,
                            current_node,
                            0,
                            true,
                            option.summary.compatible_container_options,
                            0,
                        ),
                        0,
                        true,
                        false,
                    );
                }
            }
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
    }

    ///Represents the current state of the search. Each SearchState is at either a request node or a depot, AFS are covered in the `PathOption`s.
    pub struct SearchState {
        ///current node in the distance/time matrix
        current_node: usize,
        ///all the paths leading from the previous `SearchState` to `current_node`, empty path possible, indicates loading at depot
        path_options: Vec<Rc<PathOption>>,
        ///represents the containers the truck is carrying in this state,
        container_options: Vec<ContainerOption>,
        full_request_1_source: usize,
        full_request_2_source: usize,
        ///the requests that have been visited so far, binary encoding for efficiency
        requests_visited: u64,
        ///the next state after this one, may or may not exist and may be overwritten later
        previous_state: Option<Rc<SearchState>>,
    }

    impl SearchState {
        pub fn is_carrying_full_container(&self) -> bool {
            return self.full_request_1_source != 0 || self.full_request_2_source != 0;
        }

        ///Creates the initial `SearchState` at the depot with the given `fuel_capacity`, no actions taken so far
        pub fn start_state(config: &Config, truck: &Truck) -> Rc<SearchState> {
            //container loading options
            let container_vec_capacity = (2 ^ (truck.get_num_20() + truck.get_num_40())) as usize;
            let mut container_options = Vec::with_capacity(container_vec_capacity);
            for empty_20 in 0..(truck.get_num_20() + 1) {
                for empty_40 in 0..(truck.get_num_40() + 1) {
                    container_options.push(ContainerOption {
                        empty_20,
                        empty_40,
                        num_20: empty_20,
                        num_40: empty_40,
                        previous_index: 0,
                        last_loading_distance: empty_20 + empty_40,
                        compatible_path_options: 0, //overwritten later
                    });
                }
            }
            //path_options
            let mut path_options: Vec<Rc<PathOption>> = Vec::with_capacity(3);
            let masks = SearchState::get_container_masks(&container_options);
            //all the possible loading distances, covered in descending order
            for loading_distance in (0..3).rev() {
                if masks[loading_distance] == 0 {
                    //no ContainerOption has this loading distance (should happen only for 2 loadings, at least one is always possible)
                    continue;
                }
                //calculate compatible_container_options
                let mut compatible_container_options = 0;
                for i in 0..(loading_distance + 1) {
                    compatible_container_options |= masks[i];
                }
                let new_summary = PathOptionSummary {
                    fuel_level: truck.get_fuel(),
                    total_distance: 0,
                    total_time: loading_distance as u32 * config.get_initial_depot_service_time(),
                    compatible_container_options,
                };
                //determine whether new_summary should be added. Since the higher ones are added first, this comparison is sufficient
                let mut add = true;
                for option in &path_options {
                    if option.summary.at_least_as_good_as(&new_summary) {
                        add = false;
                        break;
                    }
                }
                if add {
                    //has to be created again every single time
                    let mut path = Vec::with_capacity(1);
                    path.push(0);
                    let new_option = Rc::new(PathOption {
                        summary: new_summary,
                        path,
                        previous_index: 0,
                    });
                    path_options.push(new_option);
                }
            }
            let container_options =
                SearchState::update_container_options_with_paths(container_options, &path_options);
            let search_state = SearchState {
                current_node: 0,
                path_options,
                container_options,
                full_request_1_source: 0,
                full_request_2_source: 0,
                requests_visited: 0,
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
            container_options_at_node: Vec<ContainerOption>,
        ) -> Rc<SearchState> {
            //this function should only be called when at least one path exists
            let current_node = path_options[0].get_current_node();
            let new_state_reference = Rc::clone(&current_state);
            let (full_request_1_source, full_request_2_source) =
                current_state.get_container_sources_at_arrival(config, current_node);
            let container_options = SearchState::update_container_options_with_paths(
                container_options_at_node,
                &path_options,
            );
            let mut new_state = SearchState {
                current_node,
                path_options,
                container_options,
                full_request_1_source,
                full_request_2_source,
                requests_visited: current_state.requests_visited,
                previous_state: Option::Some(new_state_reference),
            };
            if current_node != 0 {
                assert!(!new_state.get_request_visited(current_node));
                new_state.set_request_visited(current_node);
            }
            return Rc::new(new_state);
        }

        ///Changes `ContainerOption.compatible_path_options` to match `path_options`
        fn update_container_options_with_paths(
            mut container_options: Vec<ContainerOption>,
            path_options: &Vec<Rc<PathOption>>,
        ) -> Vec<ContainerOption> {
            for (container_index, container_option) in container_options.iter_mut().enumerate() {
                container_option.compatible_path_options = 0;
                for (path_index, path_option) in path_options.iter().enumerate() {
                    if PathOption::decode_loading_mask(
                        path_option.summary.compatible_container_options,
                        container_index,
                    ) {
                        container_option.set_compatible(path_index);
                    }
                }
            }
            return container_options;
        }

        ///Returns `(full_request_1_source, full_request_2_source)` as it will be upon arrival at at `node`
        fn get_container_sources_at_arrival(&self, config: &Config, node: usize) -> (usize, usize) {
            if (0 < node) && (node <= config.get_full_pickup()) {
                if self.full_request_1_source == 0 {
                    return (node, self.full_request_2_source);
                } else if self.full_request_2_source == 0 {
                    return (self.full_request_1_source, node);
                } else {
                    panic!("THIS SHOULD NOT HAPPEN!");
                }
            } else if (config.get_first_full_dropoff() <= node)
                && node < (config.get_first_empty_dropoff())
            {
                let source_node = config.get_pickup_node_for_full_dropoff(node);
                if self.full_request_1_source == source_node {
                    return (0, self.full_request_2_source);
                } else if self.full_request_2_source == source_node {
                    return (self.full_request_1_source, 0);
                } else {
                    panic!("THIS SHOULD NOT HAPPEN!");
                }
            } else {
                return (self.full_request_1_source, self.full_request_2_source);
            }
        }

        ///If `request_node` would be visited from this `SearchState`, what would be the `container_options` of the next `SearchState`?
        /// If there are none, it means that the request cannot be served. Also checks whether the request was already handled.
        pub fn get_container_options_at_node(
            &self,
            config: &Config,
            truck: &Truck,
            request_node: usize,
        ) -> Vec<ContainerOption> {
            if self.get_request_visited(request_node) {
                //since this is also used as a checking interface, this case needs to be handled explicitly
                return Vec::with_capacity(0);
            }
            //number of options can decrease, but never increase
            let mut container_options = Vec::with_capacity(self.container_options.len());
            let request = config.get_request_at_node(request_node);
            if request_node <= config.get_full_pickup() {
                //full pickup request
                for previous_index in 0..self.container_options.len() {
                    let container_number = &self.container_options[previous_index];
                    let num_20 = container_number.num_20 + request.full_20;
                    let num_40 = container_number.num_40 + request.full_40;
                    if num_20 > truck.get_num_20() || num_40 > truck.get_num_40() {
                        //cannot load the new containers with the previous load
                        continue;
                    }
                    container_options.push(ContainerOption {
                        empty_20: container_number.empty_20,
                        empty_40: container_number.empty_40,
                        num_20,
                        num_40,
                        previous_index,
                        last_loading_distance: container_number.last_loading_distance,
                        compatible_path_options: 0, //overwritten later
                    });
                }
            } else if request_node < config.get_first_full_dropoff() {
                //empty pickup request
                for previous_index in 0..self.container_options.len() {
                    let container_number = &self.container_options[previous_index];
                    let num_20 = container_number.num_20 + request.empty_20;
                    let num_40 = container_number.num_40 + request.empty_40;
                    if num_20 > truck.get_num_20() || num_40 > truck.get_num_40() {
                        //cannot load the new containers with the previous load
                        continue;
                    }
                    let empty_20 = container_number.empty_20 + request.empty_20;
                    let empty_40 = container_number.empty_40 + request.empty_40;
                    container_options.push(ContainerOption {
                        empty_20,
                        empty_40,
                        num_20,
                        num_40,
                        previous_index,
                        last_loading_distance: container_number.last_loading_distance,
                        compatible_path_options: 0, //overwritten later
                    });
                }
            } else if request_node < config.get_first_full_dropoff() + config.get_full_pickup() {
                //full delivery
                //since this is also used as a check, verify whether the corresponding container was picked up beforehand
                let source_node = config.get_pickup_node_for_full_dropoff(request_node);
                if !self.get_request_visited(source_node) {
                    return container_options;
                }
                for previous_index in 0..self.container_options.len() {
                    let container_number = &self.container_options[previous_index];
                    //values are negative, so this is effectively a subtraction
                    let num_20 = container_number.num_20 + request.full_20;
                    let num_40 = container_number.num_40 + request.full_40;
                    //cannot lead to invalid options, the necessary containers have to be loaded
                    container_options.push(ContainerOption {
                        empty_20: container_number.empty_20,
                        empty_40: container_number.empty_40,
                        num_20,
                        num_40,
                        previous_index,
                        last_loading_distance: container_number.last_loading_distance,
                        compatible_path_options: 0, //overwritten later
                    });
                }
            } else {
                //empty delivery
                for previous_index in 0..self.container_options.len() {
                    let container_number = &self.container_options[previous_index];
                    //can also just add this because the values are negative
                    let empty_20 = container_number.empty_20 + request.empty_20;
                    let empty_40 = container_number.empty_40 + request.empty_40;
                    if empty_20 < 0 || empty_40 < 0 {
                        //cannot load the new containers with the previous load
                        continue;
                    }
                    let num_20 = container_number.num_20 + request.empty_20;
                    let num_40 = container_number.num_40 + request.empty_40;
                    container_options.push(ContainerOption {
                        empty_20,
                        empty_40,
                        num_20,
                        num_40,
                        previous_index,
                        last_loading_distance: container_number.last_loading_distance,
                        compatible_path_options: 0, //overwritten later
                    });
                }
            }
            return container_options;
        }

        ///When visiting the depot from `self`, containers can be (un-)loaded, which may lead to a new `ContainerNumber`.
        /// But only the new ones are interesting, the others are already available without visiting the depot again.
        /// Considering these old ones as well may lead to longer paths, so only the new ones are added.
        pub fn get_depot_visit_options(
            &self,
            config: &Config,
            truck: &Truck,
        ) -> Vec<ContainerOption> {
            //the number of full containers is always the same
            let org_full_20 = self.container_options[0].num_20 - self.container_options[0].empty_20;
            let org_full_40 = self.container_options[0].num_40 - self.container_options[0].empty_40;
            //iterate over possible combinations of empty containers and see whether they fit next to the full ones
            let container_vec_capacity = 8;
            let mut options: Vec<ContainerOption> = Vec::with_capacity(container_vec_capacity);
            for empty_20 in 0..(truck.get_num_20() + 1) {
                for empty_40 in 0..(truck.get_num_40() + 1) {
                    let num_20 = org_full_20 + empty_20;
                    let num_40 = org_full_40 + empty_40;
                    if num_20 > truck.get_num_20() || num_40 > truck.get_num_40() {
                        //invalid loading, no point in trying this out
                        continue;
                    }
                    //if such a `ContainerOption` already exists, there is no point in adding it - if possible, loading a container earlier
                    //is at least as good as loading it later
                    let mut add = true;
                    for existing_option in &self.container_options {
                        if existing_option.empty_20 == empty_20
                            && existing_option.empty_40 == empty_40
                        {
                            add = false;
                            break;
                        }
                    }
                    if add {
                        for previous_index in 0..self.container_options.len() {
                            //for each possible predecessor, create its successor
                            let previous_option = &self.container_options[previous_index];
                            let compatible_path_options = previous_option.compatible_path_options;
                            let last_loading_distance = (empty_20 - previous_option.empty_20).abs()
                                + (empty_40 - previous_option.empty_40).abs();
                            let new_option = ContainerOption {
                                empty_20,
                                empty_40,
                                num_20,
                                num_40,
                                previous_index,
                                last_loading_distance,
                                compatible_path_options, //at this point, it is the value from the previous `ContainerOption`, overwritten later
                            };
                            //decide what to do with the option
                            let len_before = options.len();
                            options
                                .retain(|x| !new_option.comparable_and_completely_superior_to(&x));
                            if len_before != options.len() {
                                //something was removed for being completely inferior to new_option, add new_option
                                options.push(new_option);
                            } else {
                                let mut no_reason_against_adding = true;
                                for option in &options {
                                    if option.at_least_as_good_as(&new_option) {
                                        no_reason_against_adding = false;
                                        break;
                                    }
                                }
                                if no_reason_against_adding {
                                    options.push(new_option);
                                }
                            }
                        }
                    }
                }
            }
            return options;
        }

        ///Returns whether the given `request` has been visited, makes the binary encoding more accessible.
        pub fn get_request_visited(&self, request_node: usize) -> bool {
            assert!(
                request_node != 0 && request_node < 64,
                "request_node: {}",
                request_node
            );
            //request binary is all 0 with 1 at the request index from the right
            let request_binary: u64 = 1 << (request_node - 1);
            let request_result = self.requests_visited & request_binary;
            return request_result != 0;
        }

        ///Sets the given `request` as visited, makes the binary encoding more accessible
        fn set_request_visited(&mut self, request_node: usize) {
            assert!(request_node != 0 && request_node < 64);
            //request binary is all 0 with 1 at the request index from the right
            let request_binary: u64 = 1 << (request_node - 1);
            self.requests_visited = self.requests_visited | request_binary;
        }

        /// Calculates all the relevant routes from `current_state` to the target node and returns the next `SearchState` already wrapped in `Rc`.
        /// This may be `Option::None` if no route is found
        pub fn route_to_node(
            config: &Config,
            truck: &Truck,
            current_state: &Rc<SearchState>,
            node: usize,
            container_options_at_node: Vec<ContainerOption>,
        ) -> Option<Rc<SearchState>> {
            assert_ne!(current_state.current_node, node);
            //TODO: calculate a more realistic size, there is a maximum number of elements that can be calculated
            let vec_capacity = 20;
            let mut path_options = Vec::with_capacity(vec_capacity);
            //first step, initial filling using the PathOptions from this state
            PathOption::fill_with_path_options_from_previous_state(
                config,
                truck,
                current_state,
                node,
                &mut path_options,
                &container_options_at_node,
            );
            //second step, try using previously found path_options to find a better path to somewhere (not only the depot)
            PathOption::fill_with_following_paths(config, truck, node, &mut path_options);
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
                    container_options_at_node,
                ));
            }
        }

        ///Returns `(load_0, load_1, load_2)`. Each of these encodes the following: The `n`th bit from the right is one, if the `ContainerOption`
        /// at index `n` has a `last_loading_distance` of this value (e.g. 0 for `load_0`)
        fn get_container_masks(container_options_at_node: &Vec<ContainerOption>) -> [u32; 3] {
            let mut loading_array = [0, 0, 0];
            for index in 0..container_options_at_node.len() {
                assert!(index < 32, "More than 8 ContainerOptions!");
                let option = &container_options_at_node[index];
                //key is 1 at the corresponding index
                let key = 1 << index;
                loading_array[option.last_loading_distance as usize] |= key;
            }
            return loading_array;
        }

        //Getter for `current_node`
        pub fn get_current_node(&self) -> usize {
            return self.current_node;
        }

        ///Returns the index of the `PathOption` with the lowest `total_distance` as well as the `total_distance` itself
        pub fn get_best_path_index_and_total_distance(&self) -> (usize, u32) {
            let mut best_index = 0;
            let mut lowest_distance = std::u32::MAX;
            for (index, option) in self.path_options.iter().enumerate() {
                if option.summary.total_distance < lowest_distance {
                    lowest_distance = option.summary.total_distance;
                    best_index = index;
                }
            }
            return (best_index, lowest_distance);
        }
    }

    ///Combines all the data about the current state of loaded containers for a `SearchState`
    #[derive(Clone, Copy)]
    pub struct ContainerOption {
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
        ///the number of containers that were unloaded the last time the depot was visited. May be 0,1 or 2, no other values possible.
        /// Summary metric, lower is better
        last_loading_distance: i8,
        ///the `path_options` at the previous `SearchState` that are compatible with `self`. Summary metric
        compatible_path_options: u32,
    }

    impl ContainerOption {
        ///Checks whether `empty_20` and `empty_40` are identical. `num_20` and `num_40` are not checked because their equivalence should always follow
        /// from the first comparison in the context where this function is called.
        pub fn comparable_to(&self, other: &ContainerOption) -> bool {
            return self.empty_20 == other.empty_20 && self.empty_40 == other.empty_40;
        }
        ///Returns whether `self` is superior to `other` in at least one regard. Does not check whether they are comparable in the first place.
        pub fn partially_superior_to(&self, other: &ContainerOption) -> bool {
            return self.last_loading_distance < other.last_loading_distance
                || !other.covers_completely(self);
        }
        ///Returns whether `self` is at least as good as `other` in every regard. Does not check whether they are comparable in the first place.
        pub fn at_least_as_good_as(&self, other: &ContainerOption) -> bool {
            return self.last_loading_distance <= other.last_loading_distance
                && self.covers_completely(other);
        }

        pub fn comparable_and_completely_superior_to(&self, other: &ContainerOption) -> bool {
            return self.comparable_to(other)
                && self.partially_superior_to(other)
                && self.at_least_as_good_as(other);
        }
        fn covers_completely(&self, other: &ContainerOption) -> bool {
            let difference = self.compatible_path_options ^ other.compatible_path_options;
            let bits_only_in_other = difference & self.compatible_path_options;
            return bits_only_in_other == 0;
        }
        fn set_compatible(mut self, path_index: usize) {
            assert!(path_index < 32);
            let mask = 1 << path_index;
            self.compatible_path_options |= mask;
        }
    }
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

    let root_state = SearchState::start_state(config, truck);
    let mut known_options = KnownRoutesForTruck::new();
    //special case, normal possibly_add() not called otherwise
    known_options.possibly_add(&root_state);
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
    if current_state.get_current_node() != 0 {
        //not at depot, may route there if it makes sense
        let new_container_options_at_depot = current_state.get_depot_visit_options(config, truck);
        let new_recursive_call = new_container_options_at_depot.len() != 0;
        let route_can_be_finished = !current_state.is_carrying_full_container();
        if route_can_be_finished || new_container_options_at_depot.len() > 0 {
            //routing to the depot makes sense
            match SearchState::route_to_node(
                config,
                truck,
                current_state,
                0,
                new_container_options_at_depot,
            ) {
                Option::None => (),
                Option::Some(state) => {
                    if route_can_be_finished {
                        known_options.possibly_add(&state);
                    }
                    if new_recursive_call {
                        solve_for_truck_recursive(config, truck, known_options, &state);
                    }
                }
            }
        }
    }
    //try moving to the requests
    for request_node in 1..config.get_first_afs() {
        //calculate what container_options would exist at the next state. If there are none, the request cannot be served
        let container_options =
            current_state.get_container_options_at_node(config, truck, request_node);
        if container_options.len() != 0 {
            match SearchState::route_to_node(
                config,
                truck,
                &current_state,
                request_node,
                container_options,
            ) {
                Option::None => (), //
                Option::Some(state) => {
                    solve_for_truck_recursive(config, truck, known_options, &state);
                }
            };
        };
    }
}
