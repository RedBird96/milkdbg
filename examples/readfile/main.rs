fn main() {
    if let Ok(contents) = std::fs::read_to_string("main.rs") {
        println!("{}", contents);
    }
}