fn main() {
    // Compute the current year from UNIX epoch seconds.
    // 31_556_952 = average Gregorian year in seconds (365.2425 days).
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before UNIX epoch")
        .as_secs();
    let year = 1970 + secs / 31_556_952;
    println!("cargo:rustc-env=ROUTECRAB_BUILD_YEAR={year}");
}
