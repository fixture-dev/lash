//! Drift-guard tests: assert that the static agent content in
//! `lash_agent::content` stays in sync with the clap-defined CLI surface.
//!
//! When a new subcommand is added to the CLI but not reflected in
//! `content::TOP_LEVEL_SUBCOMMANDS` or `content::cli_reference()`, these tests
//! fail loudly so the agent documentation is kept up-to-date alongside code.

use clap::CommandFactory;
use lash::cli::LashCli;
use lash_agent::content;

/// Names of subcommands intentionally hidden from agent documentation.
///
/// These exist on the CLI for shell internals but are not relevant for AI
/// agents using Lash for task management.
const HIDDEN_FROM_AGENT_DOCS: &[&str] = &[
    "completion", // shell completion generator
    "playground", // demo helper
    "tui",        // interactive UI, not relevant to non-interactive agents
    "help",       // clap built-in
];

#[test]
fn top_level_subcommands_match_clap() {
    let cmd = LashCli::command();
    let clap_names: Vec<String> = cmd
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .filter(|n| n != "help")
        .collect();

    let declared: Vec<String> = content::TOP_LEVEL_SUBCOMMANDS
        .iter()
        .map(|s| (*s).to_string())
        .collect();

    let missing: Vec<&String> = clap_names
        .iter()
        .filter(|n| !declared.contains(n))
        .collect();
    let extra: Vec<&String> = declared
        .iter()
        .filter(|n| !clap_names.contains(n))
        .collect();

    assert!(
        missing.is_empty() && extra.is_empty(),
        "TOP_LEVEL_SUBCOMMANDS drift detected.\n\
         Missing from content::TOP_LEVEL_SUBCOMMANDS (defined in clap but not declared): {missing:?}\n\
         Extra in content::TOP_LEVEL_SUBCOMMANDS (declared but not defined in clap): {extra:?}\n\n\
         Update crates/lash-agent/src/content.rs to keep agent docs in sync."
    );
}

#[test]
fn cli_reference_mentions_each_user_facing_subcommand() {
    let text = content::cli_reference();
    let cmd = LashCli::command();

    let missing: Vec<String> = cmd
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .filter(|n| !HIDDEN_FROM_AGENT_DOCS.contains(&n.as_str()))
        .filter(|n| !text.contains(&format!("lash {n}")))
        .collect();

    assert!(
        missing.is_empty(),
        "cli_reference() is missing user-facing subcommands: {missing:?}\n\n\
         Add `lash {{name}}` examples to crates/lash-agent/src/content.rs::cli_reference(),\n\
         or add the command to HIDDEN_FROM_AGENT_DOCS in this test if it should not appear\n\
         in agent documentation."
    );
}
