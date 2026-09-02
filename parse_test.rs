use std::fs::File;
use std::io::BufReader;

fn main() {
    let file = File::open("test_config.json").unwrap();
    let reader = BufReader::new(file);
    let config: serde_json::Value = serde_json::from_reader(reader).unwrap();
    println!("{:?}", config);
}
