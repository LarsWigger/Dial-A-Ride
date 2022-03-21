pub struct Truck {
    num_20_foot_containers: u32,
    num_40_foot_containers: u32,
    fuel: u32,
}
impl Truck {
    pub fn new(num_20_foot_containers: u32, num_40_foot_containers: u32, fuel: u32) -> Truck {
        return Truck {
            num_20_foot_containers,
            num_40_foot_containers,
            fuel,
        };
    }

    pub fn get_num_20_foot_containers(&self) -> u32 {
        return self.num_20_foot_containers;
    }

    pub fn get_num_40_foot_containers(&self) -> u32 {
        return self.num_40_foot_containers;
    }

    pub fn get_fuel(&self) -> u32 {
        return self.fuel;
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
    matrix_dimension: usize,
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
        let matrix_dimension = (2 * full_pickup) + empty_pickup + empty_delivery + afs + 2;
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
        };
    }
    pub fn get_distance_between(&self, i: usize, j: usize) -> u32 {
        return self.distance_matrix[i * self.matrix_dimension + j];
    }
    pub fn get_time_between(&self, i: usize, j: usize) -> u32 {
        return self.time_matrix[i * self.matrix_dimension + j];
    }
    pub fn get_num_trucks(&self) -> usize {
        return self.trucks.len();
    }
    pub fn get_truck(&self, index: usize) -> &Truck {
        return &self.trucks[index];
    }
}
