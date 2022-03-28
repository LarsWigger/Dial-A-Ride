#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Truck {
    num_20_foot_containers: i32,
    num_40_foot_containers: i32,
    //in 0.01l to avoid floating point operations, works only with the given consumption rate of 0.45l/km and refuel rate of 10l/min
    fuel: u32,
}

impl Truck {
    pub fn new(num_20_foot_containers: i32, num_40_foot_containers: i32, fuel: u32) -> Truck {
        return Truck {
            num_20_foot_containers,
            num_40_foot_containers,
            fuel: fuel * 100,
        };
    }

    pub fn get_num_20_foot_containers(&self) -> i32 {
        return self.num_20_foot_containers as i32;
    }

    pub fn get_num_40_foot_containers(&self) -> i32 {
        return self.num_40_foot_containers as i32;
    }

    pub fn get_fuel(&self) -> u32 {
        return self.fuel;
    }

    pub fn get_minutes_for_refueling(&self, old_fuel: u32) -> u32 {
        let fuel_needed = self.fuel - old_fuel;
        //integer division, 11/10=1
        let minutes_needed_at_least = fuel_needed / 10;
        //check if something was was (likely)
        if minutes_needed_at_least * 10 == fuel_needed {
            return minutes_needed_at_least;
        } else {
            return minutes_needed_at_least + 1;
        }
    }
}

pub struct ContainerRequest {
    pub full_20: i32,
    pub empty_20: i32,
    pub full_40: i32,
    pub empty_40: i32,
}

///100*0.45$
const FUEL_CONSUMPTION_PER_KM: u32 = 45;

pub struct Config {
    full_pickup: usize,
    empty_pickup: usize,
    empty_delivery: usize,
    afs: usize,
    trucks: Vec<Truck>,
    ///in km, value is 5/6 of time_matrix
    distance_matrix: Vec<u32>,
    ///in minutes, value is 6/5 of time_matrix
    time_matrix: Vec<u32>,
    ///in minutes
    depot_service_time: u32,
    ///in minutes
    service_times: Vec<u32>,
    ///in minutes
    earliest_visiting_times: Vec<u32>,
    ///in minutes
    latest_visiting_times: Vec<u32>,
    ///in minutes
    matrix_dimension: usize,
    ///requests
    requests: Vec<ContainerRequest>,
}
impl Config {
    pub fn new(
        full_pickup: usize,
        empty_pickup: usize,
        empty_delivery: usize,
        afs: usize,
        mut trucks: Vec<Truck>,
        distance_matrix: Vec<u32>,
        time_matrix: Vec<u32>,
        depot_service_time: u32,
        service_times: Vec<u32>,
        earliest_visiting_times: Vec<u32>,
        latest_visiting_times: Vec<u32>,
        requests: Vec<ContainerRequest>,
    ) -> Config {
        let matrix_dimension = (2 * full_pickup) + empty_pickup + empty_delivery + afs + 2;
        trucks.sort();
        return Config {
            full_pickup,
            empty_pickup,
            empty_delivery,
            afs,
            trucks,
            distance_matrix,
            time_matrix,
            depot_service_time,
            service_times,
            earliest_visiting_times,
            latest_visiting_times,
            matrix_dimension,
            requests,
        };
    }
    pub fn get_distance_between(&self, from: usize, to: usize) -> u32 {
        return self.distance_matrix[from * self.matrix_dimension + to];
    }
    pub fn get_time_between(&self, from: usize, to: usize) -> u32 {
        return self.time_matrix[from * self.matrix_dimension + to];
    }
    pub fn get_num_trucks(&self) -> usize {
        return self.trucks.len();
    }
    pub fn get_truck(&self, index: usize) -> &Truck {
        return &self.trucks[index];
    }
    pub fn get_depot_service_time(&self) -> u32 {
        return self.depot_service_time;
    }
    pub fn get_service_time_at_request_node(&self, request_node: usize) -> u32 {
        return self.service_times[request_node - 1];
    }
    pub fn get_earliest_visiting_time_at_request_node(&self, request_node: usize) -> u32 {
        return self.earliest_visiting_times[request_node - 1];
    }
    pub fn get_latest_visiting_time_at_request_node(&self, request_node: usize) -> u32 {
        return self.latest_visiting_times[request_node - 1];
    }
    pub fn get_full_pickup(&self) -> usize {
        return self.full_pickup;
    }
    pub fn get_empty_pickup(&self) -> usize {
        return self.empty_pickup;
    }
    pub fn get_empty_delivery(&self) -> usize {
        return self.empty_delivery;
    }
    pub fn get_afs(&self) -> usize {
        return self.afs;
    }
    pub fn get_request_at_node(&self, request_node: usize) -> &ContainerRequest {
        return &self.requests[request_node - 1];
    }
    pub fn fuel_needed_for_route(&self, from: usize, to: usize) -> u32 {
        return self.get_distance_between(from, to) * FUEL_CONSUMPTION_PER_KM;
    }
    pub fn get_first_afs(&self) -> usize {
        return 2 * self.full_pickup + self.empty_pickup + self.empty_delivery + 1;
    }
    pub fn get_pick_node_for_full_dropoff(&self, dropoff_node: usize) -> usize {
        assert!(self.empty_pickup + self.full_pickup < dropoff_node);
        return dropoff_node - self.empty_pickup - self.full_pickup;
    }
    pub fn get_dummy_depot(&self) -> usize {
        return self.matrix_dimension;
    }
    pub fn get_first_full_dropoff(&self) -> usize {
        return self.full_pickup + self.empty_delivery + 1;
    }
    pub fn get_first_empty_dropoff(&self) -> usize {
        return self.get_first_full_dropoff() + self.full_pickup;
    }
}

pub struct Solution {}
