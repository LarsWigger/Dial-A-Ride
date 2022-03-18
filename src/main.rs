fn main() {
    let sample_identifier = Parser::SampleIdentifier::new(2, 2, 2, 2, 1);
    Parser::parse(&sample_identifier);
    println!("Hello, world!");
}

mod Data {
    struct Config {}
}

mod Parser {
    use std::path::Path;

    pub struct SampleIdentifier {
        full_pickup: u32,
        empty_pickup: u32,
        empty_delivery: u32,
        afs: u32,
        number: u32,
    }

    impl SampleIdentifier {
        pub fn new(
            full_pickup: u32,
            empty_pickup: u32,
            empty_delivery: u32,
            afs: u32,
            number: u32,
        ) -> SampleIdentifier {
            return SampleIdentifier {
                full_pickup: full_pickup,
                empty_pickup: empty_pickup,
                empty_delivery: empty_delivery,
                afs: afs,
                number: number,
            };
        }

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
    }

    const BASE_PATH_STR: &str = "C:\\Users\\larsw\\Documents\\Workspaces\\Dial-A-Ride\\data";

    pub fn parse(sample: &SampleIdentifier) {
        let base_path = Path::new(BASE_PATH_STR);
        let base_path = base_path.join(sample.get_base_folder());
        //calculate base_folder, format must match and no special chars
    }
}
