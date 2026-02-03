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

    /// Update an existing entity
    Update {
        /// Entity ID (sequence number like "3" or UUID prefix like "a1b2c")
        id: String,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New status (for decisions: proposed, accepted, deprecated, superseded)
        #[arg(long)]
        status: Option<String>,

        /// Add tags (can be specified multiple times)
        #[arg(long = "tag", short = 't')]
        tags: Vec<String>,

        /// Remove tags (can be specified multiple times)
        #[arg(long = "remove-tag")]
        remove_tags: Vec<String>,

        /// Add relations in format "type:target_id" (can be specified multiple times)
        #[arg(long = "relation", short = 'r')]
        relations: Vec<String>,

        /// Read new content from stdin
        #[arg(long)]
        stdin: bool,

        /// Open $EDITOR for content
        #[arg(long, short = 'e')]
        edit: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Delete an entity
    Delete {
        /// Entity ID (sequence number like "3" or UUID prefix like "a1b2c")
        id: String,

        /// Skip confirmation prompt
        #[arg(long, short = 'f')]
        force: bool,
    },

    /// Search for entities
    Search {
        /// Search query
        query: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Task queue commands (ready, blocked, next)
    Tasks(TasksCommand),

    /// Start the MCP server
    Serve,
}

#[derive(Args, Debug)]
pub struct TasksCommand {
    #[command(subcommand)]
    pub action: TasksAction,
}

#[derive(Subcommand, Debug)]
pub enum TasksAction {
    /// List tasks with no unresolved blockers (ready to work on)
    Ready {
        /// Maximum number of tasks to show
        #[arg(long, short = 'n', default_value = "50")]
        limit: u32,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show the single highest-priority ready task
    Next {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List blocked tasks and what blocks them
    Blocked {
        /// Task ID to show blockers for (optional, shows all blocked tasks if omitted)
        #[arg(value_name = "ID")]
        id: Option<String>,

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

    /// Add a new task
    Task {
        /// Task title
        title: String,

        /// Task status (todo, in_progress, done, blocked)
        #[arg(long, default_value = "todo")]
        status: String,

        /// Priority (low, normal, high, urgent)
        #[arg(long, default_value = "normal")]
        priority: String,

        /// Due date (YYYY-MM-DD)
        #[arg(long)]
        due: Option<String>,

        /// Assignee
        #[arg(long)]
        assignee: Option<String>,

        /// Tags (can be specified multiple times)
        #[arg(long = "tag", short = 't')]
        tags: Vec<String>,

        /// Relations in format "type:target_id"
        #[arg(long = "relation", short = 'r')]
        relations: Vec<String>,

        /// Read content from stdin
        #[arg(long)]
        stdin: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Add a new note
    Note {
        /// Note title
        title: String,

        /// Note type (e.g., "meeting", "research", "idea")
        #[arg(long = "type")]
        note_type: Option<String>,

        /// Tags (can be specified multiple times)
        #[arg(long = "tag", short = 't')]
        tags: Vec<String>,

        /// Relations in format "type:target_id"
        #[arg(long = "relation", short = 'r')]
        relations: Vec<String>,

        /// Read content from stdin
        #[arg(long)]
        stdin: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Add a new prompt template
    Prompt {
        /// Prompt title
        title: String,

        /// Template text (use {{var}} for variables)
        #[arg(long)]
        template: Option<String>,

        /// Variables (can be specified multiple times)
        #[arg(long = "var")]
        variables: Vec<String>,

        /// Output JSON schema
        #[arg(long)]
        output_schema: Option<String>,

        /// Tags (can be specified multiple times)
        #[arg(long = "tag", short = 't')]
        tags: Vec<String>,

        /// Read template from stdin
        #[arg(long)]
        stdin: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Add a new component
    Component {
        /// Component title/name
        title: String,

        /// Component type (e.g., "service", "library", "api")
        #[arg(long = "type")]
        component_type: Option<String>,

        /// Status (active, deprecated, planned)
        #[arg(long, default_value = "active")]
        status: String,

        /// Owner
        #[arg(long)]
        owner: Option<String>,

        /// Tags (can be specified multiple times)
        #[arg(long = "tag", short = 't')]
        tags: Vec<String>,

        /// Relations in format "type:target_id"
        #[arg(long = "relation", short = 'r')]
        relations: Vec<String>,

        /// Read content from stdin
        #[arg(long)]
        stdin: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Add a new link
    Link {
        /// Link title/description
        title: String,

        /// URL (required)
        #[arg(long)]
        url: String,

        /// Link type (e.g., "documentation", "issue", "pr")
        #[arg(long = "type")]
        link_type: Option<String>,

        /// Tags (can be specified multiple times)
        #[arg(long = "tag", short = 't')]
        tags: Vec<String>,

        /// Relations in format "type:target_id"
        #[arg(long = "relation", short = 'r')]
        relations: Vec<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}
