//! Add command implementation
//!
//! The `lash add` command creates a new task in a Lash markdown file.

use anyhow::{Context as AnyhowContext, Result};
use clap::Args;
use lash_cli::theme::CliTheme;
use lash_core::creation::service::TaskCreationService;
use lash_types::creation::{
    FileTarget, InsertPosition, ParentRef, TaskCreationRequest, TaskCreationRequestBuilder,
};
use lash_types::status::TaskStatus;
use std::path::PathBuf;

use crate::commands::add_dependency_check::{
    emit_depends_on_warnings, file_target_relative_path, output_depends_on_errors,
    validate_depends_on, UnresolvedDependency,
};
use crate::utils::file_discovery::find_project_root;

/// Arguments for the add command
#[derive(Args, Debug, Clone)]
#[allow(clippy::struct_excessive_bools)] // CLI flags are inherently boolean
pub struct AddArgs {
    /// The task title (required)
    #[arg(required = true)]
    pub title: String,

    /// Target file path (creates if doesn't exist)
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    /// Title for new file header (only used when creating new file)
    #[arg(long)]
    pub file_title: Option<String>,

    /// Description for new file's ## Description section
    #[arg(long)]
    pub file_description: Option<String>,

    /// Parent task ID
    #[arg(short, long)]
    pub parent: Option<String>,

    /// Insert after this task ID
    #[arg(long)]
    pub after: Option<String>,

    /// Insert before this task ID
    #[arg(long)]
    pub before: Option<String>,

    /// Labels (comma-separated, repeatable: -l backend -l urgent)
    #[arg(short, long, value_delimiter = ',')]
    pub label: Vec<String>,

    /// Task owner
    #[arg(short, long)]
    pub owner: Option<String>,

    /// Time estimate (e.g., 30m, 2h, 1d, 2w)
    #[arg(short, long)]
    pub estimate: Option<String>,

    /// Initial status (open, done, waived, blocked)
    #[arg(long, default_value = "open")]
    pub status: String,

    /// Explicit task ID
    #[arg(long)]
    pub id: Option<String>,

    /// Dependencies (comma-separated, repeatable)
    #[arg(long, value_delimiter = ',')]
    pub depends_on: Vec<String>,

    /// Allow --depends-on targets that don't exist yet (warn instead of error)
    #[arg(long)]
    pub allow_forward_ref: bool,

    /// Agent note text
    #[arg(long)]
    pub agent_note: Option<String>,

    /// Output format (text, json)
    #[arg(long, default_value = "text")]
    pub format: String,

    /// Validate without creating
    #[arg(long)]
    pub dry_run: bool,

    /// Interactive mode (prompt for missing fields)
    #[arg(short, long)]
    pub interactive: bool,

    /// Disable colored output
    #[arg(long)]
    pub no_color: bool,

    /// Project root (detected automatically if None)
    ///
    /// Populated from the global `--root` flag by `main.rs`, matching every
    /// other command. Previously `add` ignored `--root` entirely and always
    /// re-derived the project root from the process's current directory,
    /// which could silently target the wrong project.
    #[arg(skip)]
    pub project_root: Option<PathBuf>,
}

/// Execute the add command
///
/// # Arguments
///
/// * `args` - Add command arguments
///
/// # Returns
///
/// Exit code: 0 (success), 1 (validation error), 3 (creation error)
pub fn execute(args: &AddArgs) -> Result<i32> {
    // Load theme based on no_color flag and output format
    let theme = if args.format == "json" {
        None
    } else {
        CliTheme::load(None, !args.no_color)?
    };

    // 1. Find project root, honoring the global `--root` override when
    // given (previously `add` ignored `--root` and always re-derived the
    // root from the process cwd, unlike every other command).
    let project_root = if let Some(root) = &args.project_root {
        root.clone()
    } else {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        find_project_root(&cwd)
    };

    tracing::info!(
        project_root = %project_root.display(),
        title = %args.title,
        "Starting task creation"
    );

    // 2. Build TaskCreationRequest from args
    let request = build_request(args, &project_root)?;

    // 3. Validate --depends-on refs against on-disk project state before
    // doing anything else (GitHub issue #27). A dangling reference is a
    // hard error by default — nothing is created or written. Passing
    // --allow-forward-ref downgrades that to a warning for the legitimate
    // case of creating tasks before their dependencies exist.
    let mut depends_on_warnings: Vec<UnresolvedDependency> = Vec::new();
    if !args.depends_on.is_empty() {
        let target_path = file_target_relative_path(&request.file_target, &project_root);
        let validation = validate_depends_on(
            &project_root,
            target_path.as_deref(),
            &args.depends_on,
            args.allow_forward_ref,
        );
        emit_depends_on_warnings(&validation.warnings, &args.format, theme.as_ref());
        if validation.has_errors() {
            output_depends_on_errors(&validation.errors, &args.format, theme.as_ref())?;
            return Ok(1);
        }
        depends_on_warnings = validation.warnings;
    }

    // 4. Handle dry-run mode
    if args.dry_run {
        return handle_dry_run(&request, args);
    }

    // 5. Create service and execute
    let config = lash_types::config::LashConfig::from_root(&project_root)
        .unwrap_or_else(|_| lash_types::config::LashConfig::default());
    let service = TaskCreationService::new(config.clone());

    match service.create_task(&request) {
        Ok(result) => {
            // 6. Re-index to update the database with the new task
            // This ensures subsequent queries (lash list, lash show) see the new task
            reindex_project(&project_root, &config)?;

            output_success(args, &result, &depends_on_warnings, theme.as_ref())?;
            Ok(0)
        }
        Err(errors) => {
            output_errors(&errors, &args.format, theme.as_ref())?;
            Ok(1)
        }
    }
}

/// Re-index the project after task creation
fn reindex_project(
    project_root: &std::path::Path,
    config: &lash_types::config::LashConfig,
) -> Result<()> {
    use lash_db::{open_database, Indexer, IndexerConfig};

    let db_path = project_root.join(".lash").join("lash.db");
    let conn = open_database(&db_path).context("Failed to open database for re-indexing")?;

    let indexer_config = IndexerConfig::new(project_root.to_path_buf()).with_incremental(true);
    let mut indexer = Indexer::new(&conn, indexer_config, config);

    indexer
        .index_project()
        .context("Failed to re-index after task creation")?;

    Ok(())
}

/// Build a `TaskCreationRequest` from command-line arguments
fn build_request(args: &AddArgs, project_root: &std::path::Path) -> Result<TaskCreationRequest> {
    let mut builder = TaskCreationRequestBuilder::new(&args.title);

    // File target
    if let Some(ref path) = args.file {
        let abs_path = if path.is_absolute() {
            path.clone()
        } else {
            project_root.join(path)
        };

        if abs_path.exists() {
            builder = builder.file_path(abs_path);
        } else {
            builder = builder.new_file(
                abs_path,
                args.file_title.clone(),
                args.file_description.clone(),
            );
        }
    }

    // Parent
    if let Some(ref parent_id) = args.parent {
        builder = builder.parent_id(parent_id);
    }

    // Position
    if let Some(ref task_id) = args.after {
        builder = builder.after(task_id);
    } else if let Some(ref task_id) = args.before {
        builder = builder.before(task_id);
    }

    // Labels
    for label in &args.label {
        builder = builder.label(label);
    }

    // Other fields
    if let Some(ref owner) = args.owner {
        builder = builder.owner(owner);
    }
    if let Some(ref estimate) = args.estimate {
        builder = builder.estimate(estimate);
    }
    if let Some(ref id) = args.id {
        builder = builder.id(id);
    }
    for dep in &args.depends_on {
        builder = builder.depends_on(dep);
    }
    if let Some(ref note) = args.agent_note {
        builder = builder.agent_note(note);
    }

    // Status
    let status = parse_status(&args.status)?;
    builder = builder.status(status);

    Ok(builder.build())
}

/// Parse a status string into a `TaskStatus`
fn parse_status(s: &str) -> Result<TaskStatus> {
    match s.to_lowercase().as_str() {
        "open" | "[ ]" => Ok(TaskStatus::Open),
        "done" | "[x]" => Ok(TaskStatus::Done),
        "waived" | "[-]" => Ok(TaskStatus::Waived),
        "blocked" | "[!]" => Ok(TaskStatus::Blocked),
        _ => Err(anyhow::anyhow!("invalid status: {s}")),
    }
}

/// Output success result
fn output_success(
    args: &AddArgs,
    result: &lash_types::creation::TaskCreationResult,
    depends_on_warnings: &[UnresolvedDependency],
    theme: Option<&CliTheme>,
) -> Result<()> {
    if args.format == "json" {
        // Output JSON. Forward-ref warnings (GitHub issue #27) are folded in
        // here rather than interleaved on stderr, since JSON output is
        // machine-consumed and stderr text would not be structured.
        let json = serde_json::json!({
            "success": true,
            "task_id": result.task_id,
            "file_path": result.file_path,
            "line_number": result.line_number,
            "is_new_file": result.is_new_file,
            "warnings": depends_on_warnings.iter().map(|w| serde_json::json!({
                "code": "E_CREATE_DEPENDENCY_NOT_FOUND",
                "target": w.target,
                "message": w.reason,
                "suggestions": w.suggestions,
            })).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else if let Some(t) = theme {
        // Text output with colors
        if result.is_new_file {
            println!(
                "{} task {} in new file {}",
                t.style_success("Created"),
                t.style_label(&result.task_id),
                t.style_info(&result.file_path.display().to_string())
            );
        } else {
            println!(
                "{} task {} at {}:{}",
                t.style_success("Created"),
                t.style_label(&result.task_id),
                t.style_info(&result.file_path.display().to_string()),
                t.style_muted(&result.line_number.to_string())
            );
        }
    } else {
        // Text output without colors
        if result.is_new_file {
            println!(
                "Created task '{}' in new file {}",
                result.task_id,
                result.file_path.display()
            );
        } else {
            println!(
                "Created task '{}' at {}:{}",
                result.task_id,
                result.file_path.display(),
                result.line_number
            );
        }
    }
    Ok(())
}

/// Output errors
fn output_errors(
    errors: &[lash_types::creation_errors::TaskCreationError],
    format: &str,
    theme: Option<&CliTheme>,
) -> Result<()> {
    if format == "json" {
        let json = serde_json::json!({
            "success": false,
            "errors": errors.iter().map(|e| {
                serde_json::json!({
                    "code": e.error_code(),
                    "message": e.message(),
                    "help": e.help(),
                })
            }).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else if let Some(t) = theme {
        for err in errors {
            eprintln!(
                "{} [{}]: {}",
                t.style_error("Error"),
                err.error_code(),
                err.message()
            );
            eprintln!("  {}: {}", t.style_info("Help"), err.help());
        }
    } else {
        for err in errors {
            eprintln!("Error [{}]: {}", err.error_code(), err.message());
            eprintln!("  Help: {}", err.help());
        }
    }
    Ok(())
}

/// Handle dry-run mode
#[allow(clippy::unnecessary_wraps)]
fn handle_dry_run(request: &TaskCreationRequest, _args: &AddArgs) -> Result<i32> {
    // In dry-run mode, we just validate and show what would be created
    println!("Validation passed. Task would be created:");
    println!("  Title: {}", request.title);

    // File target
    match &request.file_target {
        FileTarget::Current => println!("  File: <current>"),
        FileTarget::Path(path) => println!("  File: {}", path.display()),
        FileTarget::NewFile { path, title, .. } => {
            println!("  File: {} (new)", path.display());
            if let Some(t) = title {
                println!("  File Title: {t}");
            }
        }
        FileTarget::ContainingTask(ref_) => println!("  File: containing {ref_}"),
    }

    // Parent
    match &request.parent {
        ParentRef::None => println!("  Parent: <none>"),
        ParentRef::Id(id) => println!("  Parent: {id}"),
        ParentRef::FullRef(ref_) => println!("  Parent: {ref_}"),
        ParentRef::AppendAtDepth(depth) => println!("  Parent: at depth {depth}"),
    }

    // Position
    match &request.position {
        InsertPosition::Append => println!("  Position: append"),
        InsertPosition::AtIndex(idx) => println!("  Position: at index {idx}"),
        InsertPosition::Before(id) => println!("  Position: before {id}"),
        InsertPosition::After(id) => println!("  Position: after {id}"),
    }

    // Status
    if let Some(ref status) = request.status {
        println!("  Status: {status:?}");
    }

    // ID
    if let Some(ref id) = request.id {
        println!("  ID: {id}");
    }

    // Labels
    if !request.labels.is_empty() {
        println!("  Labels: {}", request.labels.join(", "));
    }

    // Owner
    if let Some(ref owner) = request.owner {
        println!("  Owner: {owner}");
    }

    // Estimate
    if let Some(ref estimate) = request.estimate {
        println!("  Estimate: {estimate}");
    }

    // Dependencies
    if !request.depends_on.is_empty() {
        println!("  Depends on: {}", request.depends_on.join(", "));
    }

    // Agent note
    if let Some(ref note) = request.agent_note {
        println!("  Agent note: {note}");
    }

    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_status_open() {
        assert_eq!(parse_status("open").unwrap(), TaskStatus::Open);
        assert_eq!(parse_status("[ ]").unwrap(), TaskStatus::Open);
    }

    #[test]
    fn test_parse_status_done() {
        assert_eq!(parse_status("done").unwrap(), TaskStatus::Done);
        assert_eq!(parse_status("[x]").unwrap(), TaskStatus::Done);
    }

    #[test]
    fn test_parse_status_waived() {
        assert_eq!(parse_status("waived").unwrap(), TaskStatus::Waived);
        assert_eq!(parse_status("[-]").unwrap(), TaskStatus::Waived);
    }

    #[test]
    fn test_parse_status_blocked() {
        assert_eq!(parse_status("blocked").unwrap(), TaskStatus::Blocked);
        assert_eq!(parse_status("[!]").unwrap(), TaskStatus::Blocked);
    }

    #[test]
    fn test_parse_status_invalid() {
        assert!(parse_status("invalid").is_err());
    }

    #[test]
    fn test_build_request_basic() {
        let args = AddArgs {
            title: "Test task".to_string(),
            file: None,
            file_title: None,
            file_description: None,
            parent: None,
            after: None,
            before: None,
            label: vec![],
            owner: None,
            estimate: None,
            status: "open".to_string(),
            id: None,
            depends_on: vec![],
            allow_forward_ref: false,
            agent_note: None,
            format: "text".to_string(),
            dry_run: false,
            interactive: false,
            no_color: true,
            project_root: None,
        };

        let project_root = PathBuf::from("/tmp");
        let request = build_request(&args, &project_root).unwrap();

        assert_eq!(request.title, "Test task");
        assert_eq!(request.parent, ParentRef::None);
        assert_eq!(request.position, InsertPosition::Append);
        assert_eq!(request.status, Some(TaskStatus::Open));
    }

    #[test]
    fn test_build_request_with_file() {
        let args = AddArgs {
            title: "Test task".to_string(),
            file: Some(PathBuf::from("test.md")),
            file_title: None,
            file_description: None,
            parent: None,
            after: None,
            before: None,
            label: vec![],
            owner: None,
            estimate: None,
            status: "open".to_string(),
            id: None,
            depends_on: vec![],
            allow_forward_ref: false,
            agent_note: None,
            format: "text".to_string(),
            dry_run: false,
            interactive: false,
            no_color: true,
            project_root: None,
        };

        let project_root = PathBuf::from("/tmp");
        let request = build_request(&args, &project_root).unwrap();

        // Since test.md doesn't exist, it should be NewFile
        match request.file_target {
            FileTarget::NewFile { path, .. } => {
                assert_eq!(path, PathBuf::from("/tmp/test.md"));
            }
            _ => panic!("Expected NewFile target"),
        }
    }

    #[test]
    fn test_build_request_with_parent() {
        let args = AddArgs {
            title: "Child task".to_string(),
            file: None,
            file_title: None,
            file_description: None,
            parent: Some("parent-task".to_string()),
            after: None,
            before: None,
            label: vec![],
            owner: None,
            estimate: None,
            status: "open".to_string(),
            id: None,
            depends_on: vec![],
            allow_forward_ref: false,
            agent_note: None,
            format: "text".to_string(),
            dry_run: false,
            interactive: false,
            no_color: true,
            project_root: None,
        };

        let project_root = PathBuf::from("/tmp");
        let request = build_request(&args, &project_root).unwrap();

        assert_eq!(request.parent, ParentRef::Id("parent-task".to_string()));
    }

    #[test]
    fn test_build_request_with_labels() {
        let args = AddArgs {
            title: "Test task".to_string(),
            file: None,
            file_title: None,
            file_description: None,
            parent: None,
            after: None,
            before: None,
            label: vec!["backend".to_string(), "urgent".to_string()],
            owner: None,
            estimate: None,
            status: "open".to_string(),
            id: None,
            depends_on: vec![],
            allow_forward_ref: false,
            agent_note: None,
            format: "text".to_string(),
            dry_run: false,
            interactive: false,
            no_color: true,
            project_root: None,
        };

        let project_root = PathBuf::from("/tmp");
        let request = build_request(&args, &project_root).unwrap();

        assert_eq!(request.labels, vec!["backend", "urgent"]);
    }

    #[test]
    fn test_build_request_with_position_after() {
        let args = AddArgs {
            title: "Test task".to_string(),
            file: None,
            file_title: None,
            file_description: None,
            parent: None,
            after: Some("task-1".to_string()),
            before: None,
            label: vec![],
            owner: None,
            estimate: None,
            status: "open".to_string(),
            id: None,
            depends_on: vec![],
            allow_forward_ref: false,
            agent_note: None,
            format: "text".to_string(),
            dry_run: false,
            interactive: false,
            no_color: true,
            project_root: None,
        };

        let project_root = PathBuf::from("/tmp");
        let request = build_request(&args, &project_root).unwrap();

        assert_eq!(
            request.position,
            InsertPosition::After("task-1".to_string())
        );
    }

    #[test]
    fn test_build_request_with_position_before() {
        let args = AddArgs {
            title: "Test task".to_string(),
            file: None,
            file_title: None,
            file_description: None,
            parent: None,
            after: None,
            before: Some("task-1".to_string()),
            label: vec![],
            owner: None,
            estimate: None,
            status: "open".to_string(),
            id: None,
            depends_on: vec![],
            allow_forward_ref: false,
            agent_note: None,
            format: "text".to_string(),
            dry_run: false,
            interactive: false,
            no_color: true,
            project_root: None,
        };

        let project_root = PathBuf::from("/tmp");
        let request = build_request(&args, &project_root).unwrap();

        assert_eq!(
            request.position,
            InsertPosition::Before("task-1".to_string())
        );
    }

    #[test]
    fn test_build_request_with_metadata() {
        let args = AddArgs {
            title: "Test task".to_string(),
            file: None,
            file_title: None,
            file_description: None,
            parent: None,
            after: None,
            before: None,
            label: vec![],
            owner: Some("alice".to_string()),
            estimate: Some("2h".to_string()),
            status: "open".to_string(),
            id: Some("custom-id".to_string()),
            depends_on: vec!["dep1".to_string(), "dep2".to_string()],
            allow_forward_ref: false,
            agent_note: Some("Important note".to_string()),
            format: "text".to_string(),
            dry_run: false,
            interactive: false,
            no_color: true,
            project_root: None,
        };

        let project_root = PathBuf::from("/tmp");
        let request = build_request(&args, &project_root).unwrap();

        assert_eq!(request.owner, Some("alice".to_string()));
        assert_eq!(request.estimate, Some("2h".to_string()));
        assert_eq!(request.id, Some("custom-id".to_string()));
        assert_eq!(request.depends_on, vec!["dep1", "dep2"]);
        assert_eq!(request.agent_note, Some("Important note".to_string()));
    }
}
