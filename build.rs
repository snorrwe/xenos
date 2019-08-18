extern crate chrono;
use std::fs;

fn main() {
    let now = chrono::Utc::now();
    let now = format!("{}", now);
    fs::write("VERSION", now).expect("Failed to create version file");
}
