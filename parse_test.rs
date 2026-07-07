use chrono::NaiveDateTime;
fn main() {
    println!("{:?}", NaiveDateTime::parse_from_str("01-May-2026", "%d-%b-%Y"));
    println!("{:?}", NaiveDateTime::parse_from_str("01-May-2026 00:00:00", "%d-%b-%Y %H:%M:%S"));
}
