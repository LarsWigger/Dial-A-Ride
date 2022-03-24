use crate::data::Config;
use crate::data::ContainerRequest;
use crate::data::Truck;
use std::fs;
use std::path::Path;

struct DataIdentifier {
    full_pickup: usize,
    empty_pickup: usize,
    empty_delivery: usize,
    afs: usize,
    sample_number: usize,
    scenario: usize,
    num_trucks: usize,
    t_max: usize,
}

impl DataIdentifier {
    fn get_base_folder(&self) -> String {
        return format!(
            "{}f{}p{}d{}s{}",
            self.full_pickup,
            self.empty_pickup,
            self.empty_delivery,
            self.afs,
            self.get_data_type()
        );
    }
    fn get_data_type(&self) -> &str {
        if self.full_pickup > self.empty_delivery {
            return "A";
        } else {
            return "B";
        };
    }
    fn get_matrix_dimension(&self) -> usize {
        return self.get_request_number() + self.afs + 2;
    }
    fn get_matrix_file_ending(&self) -> String {
        return format!(
            "{}_F{}_P{}_D_{}S_No{}.txt",
            self.full_pickup, self.empty_pickup, self.empty_delivery, self.afs, self.sample_number
        );
    }
    fn get_distance_matrix_file_name(&self) -> String {
        return format!("DistanceMatrix{}", self.get_matrix_file_ending());
    }
    fn get_time_matrix_file_name(&self) -> String {
        return format!("TimeMatrix{}", self.get_matrix_file_ending());
    }
    fn get_fuel_file_name(&self) -> String {
        return format!("FuelLevel{}_T.txt", self.num_trucks);
    }
    fn get_resouce_file_name(&self) -> String {
        return format!("Resource{}_T.txt", self.num_trucks);
    }
    fn get_base_file_name(&self) -> String {
        return format!(
            "{}f{}p{}d{}s{}n{}.txt",
            self.full_pickup,
            self.empty_pickup,
            self.empty_delivery,
            self.afs,
            self.sample_number,
            self.get_data_type()
        );
    }
    fn get_variables_file_name(&self) -> String {
        return format!(
            "variables{}_N{}_T{}_P_No{}.txt",
            self.get_request_number(),
            self.t_max,
            self.full_pickup,
            self.sample_number
        );
    }
    fn get_request_number(&self) -> usize {
        return 2 * self.full_pickup + self.empty_pickup + self.empty_delivery;
    }
    fn get_demand_file_name(&self) -> String {
        return format!(
            "demand{}_F{}_P{}_D.txt",
            self.full_pickup, self.empty_pickup, self.empty_delivery
        );
    }
}

const BASE_PATH_STR: &str = "C:\\Users\\larsw\\Documents\\Workspaces\\Dial-A-Ride\\data";

pub fn parse(
    full_pickup: usize,
    empty_pickup: usize,
    empty_delivery: usize,
    afs: usize,
    sample_number: usize,
    scenario: usize,
) -> Config {
    //setup data needed for parsing
    let mut identifier = DataIdentifier {
        full_pickup: full_pickup,
        empty_pickup: empty_pickup,
        empty_delivery: empty_delivery,
        afs: afs,
        sample_number: sample_number,
        scenario: scenario,
        num_trucks: 0, //to be overwritten
        t_max: 0,      //to be overwritten
    };
    let base_path = Path::new(BASE_PATH_STR).join(identifier.get_base_folder());
    parse_num_trucks_and_t_max(&mut identifier, &base_path);
    println!(
        "num_trucks: {}; t_max: {}",
        identifier.num_trucks, identifier.t_max
    );
    //parse trucks
    let truck_vec = parse_trucks(&identifier, &base_path);
    //parse matrices
    let matrix_size = identifier.get_matrix_dimension() ^ 2;
    let mut distance_matrix = Vec::with_capacity(matrix_size);
    let distance_matrix_path = base_path.join(identifier.get_distance_matrix_file_name());
    parse_matrix(&distance_matrix_path, &mut distance_matrix);
    let mut time_matrix = Vec::with_capacity(matrix_size);
    let time_matrix_path = base_path.join(identifier.get_time_matrix_file_name());
    parse_matrix(&time_matrix_path, &mut time_matrix);
    //parse time constraints
    let (depot_service_time, service_times, earliest_visiting_times, latest_visiting_times) =
        parse_time_constraints(&identifier, &base_path);
    //parse requests
    let requests = parse_requests(&identifier, &base_path);
    //create and return config
    return Config::new(
        full_pickup,
        empty_pickup,
        empty_delivery,
        afs,
        truck_vec,
        distance_matrix,
        time_matrix,
        depot_service_time,
        service_times,
        earliest_visiting_times,
        latest_visiting_times,
        requests,
    );
}

fn parse_requests(identifier: &DataIdentifier, base_path: &Path) -> Vec<ContainerRequest> {
    let request_number = identifier.get_request_number();
    let mut request_vec = Vec::with_capacity(request_number);
    let path = base_path.join(identifier.get_demand_file_name());
    let demand_string = fs::read_to_string(path).unwrap();
    let mut demand_entries = demand_string.split_whitespace();
    for _ in 0..request_number {
        let full_20 = demand_entries.next().unwrap().parse().unwrap();
        let empty_20 = demand_entries.next().unwrap().parse().unwrap();
        let full_40 = demand_entries.next().unwrap().parse().unwrap();
        let empty_40 = demand_entries.next().unwrap().parse().unwrap();
        let request = ContainerRequest {
            full_20,
            empty_20,
            full_40,
            empty_40,
        };
        request_vec.push(request);
    }
    return request_vec;
}

fn parse_num_trucks_and_t_max(identifier: &mut DataIdentifier, base_path: &Path) {
    let path = base_path.join(identifier.get_base_file_name());
    println!("Parsing {} ...", path.display());
    let file_string = fs::read_to_string(path).unwrap();
    let mut lines = file_string.lines();
    //remove header and the lines before the intended one
    for _ in 0..identifier.scenario {
        _ = lines.next()
    }
    let mut entries = lines.next().unwrap().split_whitespace();
    for i in 0..10 {
        if i == 2 {
            //num_trucks
            identifier.num_trucks = entries.next().unwrap().parse().unwrap();
        } else if i == 7 {
            identifier.t_max = entries.next().unwrap().parse().unwrap();
        } else {
            _ = entries.next();
        }
    }
}

fn parse_trucks(identifier: &DataIdentifier, base_path: &Path) -> Vec<Truck> {
    let mut truck_vec = Vec::with_capacity(identifier.num_trucks);
    let resource_path = base_path.join(identifier.get_resouce_file_name());
    let resource_string = fs::read_to_string(resource_path).unwrap();
    let mut resource_lines = resource_string.lines();
    let fuel_path = base_path.join(identifier.get_fuel_file_name());
    let fuel_string = fs::read_to_string(fuel_path).unwrap();
    let mut fuel_entries = fuel_string.split_whitespace();
    for _ in 0..identifier.num_trucks {
        let mut resource_entries = resource_lines.next().unwrap().split_whitespace();
        let num_20_foot_containers = resource_entries.next().unwrap().parse().unwrap();
        let num_40_foot_containers = resource_entries.next().unwrap().parse().unwrap();
        let fuel = fuel_entries.next().unwrap().parse().unwrap();
        let truck = Truck::new(num_20_foot_containers, num_40_foot_containers, fuel);
        truck_vec.push(truck);
    }
    return truck_vec;
}

fn parse_matrix(path: &Path, matrix: &mut Vec<u32>) {
    let matrix_string = fs::read_to_string(path).unwrap();
    for entry in matrix_string.split_whitespace() {
        matrix.push(entry.parse().unwrap());
    }
}

fn parse_time_constraints(
    identifier: &DataIdentifier,
    base_path: &Path,
) -> (u32, Vec<u32>, Vec<u32>, Vec<u32>) {
    let path = base_path.join(identifier.get_variables_file_name());
    let contraint_string = fs::read_to_string(path).unwrap();
    let mut constraint_lines = contraint_string.lines();
    let num_entries = identifier.get_matrix_dimension() - 2 - identifier.afs;
    //service times
    let mut service_time_entries = constraint_lines.next().unwrap().split_whitespace();
    let depot_service_time: u32 = service_time_entries.next().unwrap().parse().unwrap();
    let mut service_times: Vec<u32> = Vec::with_capacity(num_entries);
    for _ in 0..num_entries {
        service_times.push(service_time_entries.next().unwrap().parse().unwrap());
    }
    //earliest times
    let earliest_time_entries = constraint_lines.next().unwrap().split_whitespace();
    let mut earliest_visiting_times: Vec<u32> = Vec::with_capacity(num_entries);
    for entry in earliest_time_entries {
        earliest_visiting_times.push(entry.parse().unwrap());
    }
    //latest times
    let latest_time_entries = constraint_lines.next().unwrap().split_whitespace();
    let mut latest_visiting_times: Vec<u32> = Vec::with_capacity(num_entries);
    for entry in latest_time_entries {
        latest_visiting_times.push(entry.parse().unwrap());
    }
    return (
        depot_service_time,
        service_times,
        earliest_visiting_times,
        latest_visiting_times,
    );
}

#[cfg(test)]
mod parser_test {
    use crate::parser;
    #[test]
    fn parse_2_2_2_2_1_2() {
        let config = parser::parse(2, 2, 2, 2, 1, 2);
        //test distance matrix
        assert_eq!(config.get_distance_between(0, 0), 0);
        assert_eq!(config.get_distance_between(3, 2), 199);
        //test time matrix
        assert_eq!(config.get_time_between(10, 9), 94);
        //trucks
        assert_eq!(config.get_num_trucks(), 6);
        let truck = config.get_truck(4);
        assert_eq!(truck.get_num_20_foot_containers(), 2);
        assert_eq!(truck.get_num_40_foot_containers(), 0);
        assert_eq!(truck.get_fuel(), 28500);
        //times
        assert_eq!(config.get_depot_service_time(), 0);
        assert_eq!(config.get_service_time(0), 10);
        assert_eq!(config.get_earliest_visiting_time(7), 404);
        assert_eq!(config.get_latest_visiting_time(6), 802);
        //base values
        assert_eq!(config.get_full_pickup(), 2);
        assert_eq!(config.get_empty_pickup(), 2);
        assert_eq!(config.get_empty_delivery(), 2);
        assert_eq!(config.get_afs(), 2);
        //request
        let request = config.get_request_at_node(1);
        assert_eq!(request.full_20, 1);
        assert_eq!(request.empty_20, 0);
        assert_eq!(request.full_40, 0);
        assert_eq!(request.empty_40, 0);
        let request = config.get_request_at_node(2);
        assert_eq!(request.full_20, 1);
        assert_eq!(request.empty_20, 0);
        assert_eq!(request.full_40, 0);
        assert_eq!(request.empty_40, 0);
        let request = config.get_request_at_node(4);
        assert_eq!(request.full_20, 0);
        assert_eq!(request.empty_20, 1);
        assert_eq!(request.full_40, 0);
        assert_eq!(request.empty_40, 0);
        let request = config.get_request_at_node(8);
        assert_eq!(request.full_20, 0);
        assert_eq!(request.empty_20, -1);
        assert_eq!(request.full_40, 0);
        assert_eq!(request.empty_40, 0);
    }
}
