extern crate semver;

use semver::Version;
use std::fs;

fn main() {
    let mut version = if let Ok(version) = fs::read_to_string("VERSION") {
        Version::parse(&version)
            .or(Version::parse("0.1.0"))
            .unwrap()
    } else {
        Version::parse("0.1.0").unwrap()
    };

    version.increment_minor();

    fs::write("VERSION", format!("{}", version)).expect("Failed to create version file");
}

