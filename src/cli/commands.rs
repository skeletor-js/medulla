use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "medulla")]
#[command(version, about = "A git-native, AI-accessible knowledge engine")]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new medulla project in the current directory
    Init {
        /// Accept all optional features without prompting
        #[arg(long, conflicts_with = "no")]
        yes: bool,

        /// Decline all optional features without prompting
        #[arg(long, conflicts_with = "yes")]
        no: bool,
    },

    /// Add a new entity
    Add(AddCommand),

    /// List entities
    List {
        /// Entity type to list (decision, task, note, etc.)
        #[arg(value_name = "TYPE")]
        entity_type: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Get a single entity by ID
    Get {
        /// Entity ID (sequence number like "3" or UUID prefix like "a1b2c")
        id: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Args, Debug)]
pub struct AddCommand {
    #[command(subcommand)]
    pub entity: AddEntity,
}

#[derive(Subcommand, Debug)]
pub enum AddEntity {
    /// Add a new decision
    Decision {
        /// Decision title
        title: String,

        /// Decision status (proposed, accepted, deprecated, superseded)
        #[arg(long, default_value = "proposed")]
        status: String,

        /// Tags (can be specified multiple times)
        #[arg(long = "tag", short = 't')]
        tags: Vec<String>,

        /// Relations in format "type:target_id" (can be specified multiple times)
        #[arg(long = "relation", short = 'r')]
        relations: Vec<String>,

        /// Read content from stdin
        #[arg(long)]
        stdin: bool,

        /// Open $EDITOR for content
        #[arg(long, short = 'e')]
        edit: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}
