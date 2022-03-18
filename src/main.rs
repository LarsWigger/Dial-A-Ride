fn main() {
    let config = parser::parse(2, 2, 2, 2, 1);
    println!("Hello, world!");
}

mod data {
    pub struct Config {
        distance_matrix: Vec<u32>,
        time_matrix: Vec<u32>,
    }
    impl Config {
        pub fn new(distance_matrix: Vec<u32>, time_matrix: Vec<u32>) -> Config {
            return Config {
                distance_matrix: distance_matrix,
                time_matrix: time_matrix,
            };
        }
    }
}

mod parser {
    use crate::data::Config;
    use std::fs;
    use std::path::Path;

    pub struct SampleIdentifier {
        full_pickup: u32,
        empty_pickup: u32,
        empty_delivery: u32,
        afs: u32,
        number: u32,
    }

    impl SampleIdentifier {
        fn get_base_folder(&self) -> String {
            //how is this defined? unclear whether delivery also counts
            let data_type = if self.full_pickup > self.empty_delivery {
                "A"
            } else {
                "B"
            };
            return format!(
                "{}f{}p{}d{}s{}",
                self.full_pickup, self.empty_pickup, self.empty_delivery, self.afs, data_type
            );
        }
        fn get_matrix_size(&self) -> u32 {
            return self.full_pickup + self.empty_pickup + self.empty_delivery + self.afs;
        }
        fn get_matrix_file_ending(&self) -> String {
            return format!(
                "{}_F{}_P{}_D_{}S_No{}.txt",
                self.full_pickup, self.empty_pickup, self.empty_delivery, self.afs, self.number
            );
        }
        fn get_distance_matrix_file_name(&self) -> String {
            return format!("DistanceMatrix{}", self.get_matrix_file_ending());
        }
        fn get_time_matrix_file_name(&self) -> String {
            return format!("TimeMatrix{}", self.get_matrix_file_ending());
        }
    }

    const BASE_PATH_STR: &str = "C:\\Users\\larsw\\Documents\\Workspaces\\Dial-A-Ride\\data";

    pub fn parse(
        full_pickup: u32,
        empty_pickup: u32,
        empty_delivery: u32,
        afs: u32,
        number: u32,
    ) -> Config {
        let sample = SampleIdentifier {
            full_pickup: full_pickup,
            empty_pickup: empty_pickup,
            empty_delivery: empty_delivery,
            afs: afs,
            number: number,
        };
        //calculate base_path
        let base_path = Path::new(BASE_PATH_STR);
        let base_path = base_path.join(sample.get_base_folder());
        //parse matrices
        let matrix_size = (sample.get_matrix_size() ^ 2) as usize;
        let mut distance_matrix = Vec::with_capacity(matrix_size);
        parse_matrix(
            &mut distance_matrix,
            &base_path.join(sample.get_distance_matrix_file_name()),
        );
        let mut time_matrix = Vec::with_capacity(matrix_size);
        parse_matrix(
            &mut time_matrix,
            &base_path.join(sample.get_time_matrix_file_name()),
        );
        return crate::data::Config::new(distance_matrix, time_matrix);
    }

    fn parse_matrix(matrix: &mut Vec<u32>, path: &Path) {
        //read file
        println!("Parsing matrix from {} ...", path.display());
        let file_string = fs::read_to_string(path).expect("Unable to parse matrix");
        for number in file_string.split_whitespace() {
            matrix.push(number.parse().unwrap());
        }
    }
}
