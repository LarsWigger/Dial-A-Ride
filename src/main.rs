fn main() {
    parser::parse(2, 2, 2, 2, 1, 2);
    println!("Hello, world!");
}

mod data {
    pub struct Truck {
        num_20_foot_containers: usize,
        num_40_foot_containers: usize,
        fuel: usize,
    }

    impl Truck {
        pub fn new(
            num_20_foot_containers: usize,
            num_40_foot_containers: usize,
            fuel: usize,
        ) -> Truck {
            return Truck {
                num_20_foot_containers,
                num_40_foot_containers,
                fuel,
            };
        }
    }
    struct Config {}
}

mod parser {
    use crate::data::Truck;
    use std::fs;
    use std::path::Path;

    pub struct DataIdentifier {
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
            if (self.full_pickup > self.empty_delivery) {
                return "A";
            } else {
                return "B";
            };
        }
        fn get_matrix_dimension(&self) -> usize {
            return (2 * self.full_pickup) + self.empty_pickup + self.empty_delivery + self.afs + 2;
        }
        fn get_matrix_file_ending(&self) -> String {
            return format!(
                "{}_F{}_P{}_D_{}S_No{}.txt",
                self.full_pickup,
                self.empty_pickup,
                self.empty_delivery,
                self.afs,
                self.sample_number
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
        fn get_total_capacity_file_name(&self) -> String {
            return format!("Truck{}_T.txt", self.num_trucks);
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
    }

    const BASE_PATH_STR: &str = "C:\\Users\\larsw\\Documents\\Workspaces\\Dial-A-Ride\\data";

    pub fn parse(
        full_pickup: usize,
        empty_pickup: usize,
        empty_delivery: usize,
        afs: usize,
        sample_number: usize,
        scenario: usize,
    ) {
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
        let mut truck_vec = Vec::with_capacity(identifier.num_trucks);
        parse_trucks(&identifier, &base_path, &mut truck_vec);
        //parse matrices
        let matrix_size = identifier.get_matrix_dimension() ^ 2;
        let mut distance_matrix = Vec::with_capacity(matrix_size);
        let distance_matrix_path = base_path.join(identifier.get_distance_matrix_file_name());
        parse_matrix(&distance_matrix_path, &mut distance_matrix);
        let mut time_matrix = Vec::with_capacity(matrix_size);
        let time_matrix_path = base_path.join(identifier.get_time_matrix_file_name());
        parse_matrix(&time_matrix_path, &mut time_matrix);
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
            if (i == 2) {
                //num_trucks
                identifier.num_trucks = entries.next().unwrap().parse().unwrap();
            } else if (i == 7) {
                identifier.t_max = entries.next().unwrap().parse().unwrap();
            } else {
                _ = entries.next();
            }
        }
    }

    fn parse_trucks(identifier: &DataIdentifier, base_path: &Path, truck_vec: &mut Vec<Truck>) {
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
    }

    fn parse_matrix(path: &Path, matrix: &mut Vec<u32>) {
        let matrix_string = fs::read_to_string(path).unwrap();
        for entry in matrix_string.split_whitespace() {
            matrix.push(entry.parse().unwrap());
        }
    }
}
