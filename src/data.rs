pub enum ContainerType {
    Full20,
    Empty20,
    Full40,
    Empty40,
    NoContainer,
}

pub struct Truck {
    num_20_foot_containers: u32,
    num_40_foot_containers: u32,
    fuel: f32,
}

impl Truck {
    pub fn new(num_20_foot_containers: u32, num_40_foot_containers: u32, fuel: f32) -> Truck {
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

    pub fn get_fuel(&self) -> f32 {
        return self.fuel;
    }
}

pub struct ContainerRequest {
    pub full_20: i32,
    pub empty_20: i32,
    pub full_40: i32,
    pub empty_40: i32,
}

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
        trucks: Vec<Truck>,
        distance_matrix: Vec<u32>,
        time_matrix: Vec<u32>,
        depot_service_time: u32,
        service_times: Vec<u32>,
        earliest_visiting_times: Vec<u32>,
        latest_visiting_times: Vec<u32>,
        requests: Vec<ContainerRequest>,
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
            requests,
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
    pub fn get_depot_service_time(&self) -> u32 {
        return self.depot_service_time;
    }
    pub fn get_service_time(&self, index: usize) -> u32 {
        return self.service_times[index];
    }
    pub fn get_earliest_visiting_time(&self, index: usize) -> u32 {
        return self.earliest_visiting_times[index];
    }
    pub fn get_latest_visiting_time(&self, index: usize) -> u32 {
        return self.latest_visiting_times[index];
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
    pub fn get_request(&self, index: usize) -> &ContainerRequest {
        return &self.requests[index];
    }
}
