use clap::Parser;
use medulla::cli::{
    handle_add_decision, handle_get, handle_init, handle_list, AddEntity, Cli, Commands,
};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { yes, no } => handle_init(yes, no),
        Commands::Add(add) => match add.entity {
            AddEntity::Decision {
                title,
                status,
                tags,
                relations,
                stdin,
                edit,
                json,
            } => handle_add_decision(title, status, tags, relations, stdin, edit, json),
        },
        Commands::List { entity_type, json } => handle_list(entity_type, json),
        Commands::Get { id, json } => handle_get(id, json),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
