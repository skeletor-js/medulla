use clap::Parser;
use medulla::cli::{
    handle_add_component, handle_add_decision, handle_add_link, handle_add_note, handle_add_prompt,
    handle_add_task, handle_delete, handle_get, handle_init, handle_list, handle_relation_add,
    handle_relation_delete, handle_relation_list, handle_search, handle_serve, handle_tasks_blocked,
    handle_tasks_next, handle_tasks_ready, handle_update, AddEntity, Cli, Commands, RelationAction,
    TasksAction,
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
            AddEntity::Task {
                title,
                status,
                priority,
                due,
                assignee,
                tags,
                relations,
                stdin,
                json,
            } => handle_add_task(title, status, priority, due, assignee, tags, relations, stdin, json),
            AddEntity::Note {
                title,
                note_type,
                tags,
                relations,
                stdin,
                json,
            } => handle_add_note(title, note_type, tags, relations, stdin, json),
            AddEntity::Prompt {
                title,
                template,
                variables,
                output_schema,
                tags,
                stdin,
                json,
            } => handle_add_prompt(title, template, variables, output_schema, tags, stdin, json),
            AddEntity::Component {
                title,
                component_type,
                status,
                owner,
                tags,
                relations,
                stdin,
                json,
            } => handle_add_component(title, component_type, status, owner, tags, relations, stdin, json),
            AddEntity::Link {
                title,
                url,
                link_type,
                tags,
                relations,
                json,
            } => handle_add_link(title, url, link_type, tags, relations, json),
        },
        Commands::List { entity_type, json } => handle_list(entity_type, json),
        Commands::Get { id, json } => handle_get(id, json),
        Commands::Update {
            id,
            title,
            status,
            tags,
            remove_tags,
            relations,
            stdin,
            edit,
            json,
        } => handle_update(id, title, status, tags, remove_tags, relations, stdin, edit, json),
        Commands::Delete { id, force } => handle_delete(id, force),
        Commands::Search { query, json } => handle_search(query, json),
        Commands::Tasks(tasks_cmd) => match tasks_cmd.action {
            TasksAction::Ready { limit, json } => handle_tasks_ready(limit, json),
            TasksAction::Next { json } => handle_tasks_next(json),
            TasksAction::Blocked { id, json } => handle_tasks_blocked(id, json),
        },
        Commands::Serve => handle_serve(),
        Commands::Relation(rel_cmd) => match rel_cmd.action {
            RelationAction::Add {
                source_id,
                target_id,
                relation_type,
                json,
            } => handle_relation_add(source_id, target_id, relation_type, json),
            RelationAction::Delete {
                source_id,
                target_id,
                relation_type,
                json,
            } => handle_relation_delete(source_id, target_id, relation_type, json),
            RelationAction::List { entity_id, json } => handle_relation_list(entity_id, json),
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
