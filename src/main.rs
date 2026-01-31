use clap::Parser;
use medulla::cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { yes, no } => {
            println!("Init called with yes={}, no={}", yes, no);
            Ok(())
        }
        Commands::Add(add) => {
            println!("Add called: {:?}", add);
            Ok(())
        }
        Commands::List { entity_type, json } => {
            println!("List called: type={:?}, json={}", entity_type, json);
            Ok(())
        }
        Commands::Get { id, json } => {
            println!("Get called: id={}, json={}", id, json);
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
