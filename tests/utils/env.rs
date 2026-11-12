pub fn path() -> String {
    std::env::var("PATH").expect("PATH")
}
