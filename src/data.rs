pub struct Truck {
    num_20_foot_containers: usize,
    num_40_foot_containers: usize,
    fuel: usize,
}

impl Truck {
    pub fn new(num_20_foot_containers: usize, num_40_foot_containers: usize, fuel: usize) -> Truck {
        return Truck {
            num_20_foot_containers,
            num_40_foot_containers,
            fuel,
        };
    }
}
pub struct Config {
    full_pickup: usize,
    empty_pickup: usize,
    empty_delivery: usize,
    afs: usize,
    trucks: Vec<Truck>,
    distance_matrix: Vec<u32>,
    time_matrix: Vec<u32>,
    depot_service_time: u32,
    service_times: Vec<u32>,
    earliest_visiting_times: Vec<u32>,
    latest_visiting_times: Vec<u32>,
}

impl Config {
    pub fn new(
        full_pickup: usize,
        empty_pickup: usize,
        empty_delivery: usize,
        afs: usize,
        trucks: Vec<Truck>,
        distance_matrix: Vec<u32>,
        time_matrix: Vec<u32>,
        depot_service_time: u32,
        service_times: Vec<u32>,
        earliest_visiting_times: Vec<u32>,
        latest_visiting_times: Vec<u32>,
    ) -> Config {
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
        };
    }
}
