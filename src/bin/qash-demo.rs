#[path = "../demo.rs"]
mod demo;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match demo::run_demo_cli(&args) {
        Ok(()) => {}
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    }
}
