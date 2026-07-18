fn main() {
    let now = chrono::Local::now();
    let current_month = now.format("%b").to_string(); // e.g. "Jul"
    let current_month_year = now.format("%b-%Y").to_string(); // e.g. "Jul-2026"
}
