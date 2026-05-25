use std::env;
use std::path::Path;

fn main() {
    // 1. Grab the arguments passed in the terminal
    let args: Vec<String> = env::args().skip(1).collect();
    println!("{:?}", env::args());
    // 2. Check if the user actually gave us a file
    if args.is_empty() {
        eprintln!("Error: You need to provide a file path.");
        std::process::exit(1);
    }

    // 3. Get the file path
    let input_path = &args[0];
    let path = Path::new(input_path);

    // 4. Check if the file actually exists on the hard drive
    if !path.exists() {
        eprintln!("Error: The file '{}' does not exist.", input_path);
        std::process::exit(1);
    }

    println!("Success! Found the file: {}", input_path);
}