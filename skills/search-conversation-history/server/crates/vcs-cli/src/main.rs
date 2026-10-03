//! `vcs` — the command line face of Visible Conversation Search.
//!
//! Every subcommand runs against the same local index the desktop application uses. Nothing here
//! contacts the network, and no subcommand writes to a history store.

mod server;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use vcs_service::{bench, integrations, map, App, IndexRequest};

#[derive(Parser)]
#[command(name = "vcs", about = "Local, private search over visible AI conversations")]
struct Cli {
    /// Index file. Defaults to ~/.local/share/visible-conversation-search/index.sqlite
    #[arg(long, global = true)]
    database: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List the history stores installed on this computer.
    Detect {
        /// Normalize every store to report exact visible-record counts. Slow on a large history.
        #[arg(long)]
        exact: bool,
        /// Limit the window to the last N days.
        #[arg(long)]
        days: Option<u32>,
    },
    /// Build or refresh the local index.
    Index {
        #[arg(long)]
        source: Vec<String>,
        #[arg(long)]
        days: Option<u32>,
    },
    /// Search visible conversation text, returning JSON snippets with pagination and filters.
    Search {
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value = "snippet", value_parser = ["snippet", "full"])]
        format: String,
        #[arg(long, default_value_t = 0)]
        offset: usize,
        #[arg(long, value_parser = ["codex", "claude", "cursor"])]
        source: Option<String>,
        #[arg(long)]
        cwd_prefix: Option<String>,
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        until: Option<String>,
        #[arg(long, default_value = "oldest", value_parser = ["oldest", "newest"], conflicts_with = "newest")]
        order: String,
        /// Compatibility alias for --order newest.
        #[arg(long)]
        newest: bool,
    },
    /// Read the full indexed visible record identified by a search result.
    Record { record_id: String },
    /// Print the visible exchange around one record.
    Context {
        #[arg(long)]
        session: String,
        #[arg(long)]
        timestamp: Option<String>,
        #[arg(long, default_value_t = 3)]
        span: usize,
    },
    /// Propose phrases for the speed comparison.
    Suggest {
        #[arg(long, default_value_t = 3)]
        count: usize,
    },
    /// Measure the native search path against the local index.
    Bench {
        query: String,
        #[arg(long, default_value_t = 200)]
        limit: usize,
        /// Repeat both sides N times and report the best of each.
        #[arg(long, default_value_t = 1)]
        repeat: usize,
    },
    /// Group the last N days of visible conversation by project and topic.
    Map {
        #[arg(long, default_value_t = 7)]
        days: u32,
    },
    /// Show exactly what connecting this product to each AI tool would change.
    Plan,
    /// Write those changes. Requires --confirm.
    Connect {
        #[arg(long)]
        tool: Vec<String>,
        #[arg(long)]
        confirm: bool,
    },
    /// Remove the entry this product added from each tool's configuration.
    Disconnect {
        #[arg(long)]
        tool: Vec<String>,
    },
    /// Serve the local HTTP bridge for the desktop interface during development.
    Serve {
        #[arg(long, default_value = "127.0.0.1:8787")]
        address: String,
        /// Directory of built interface files to serve at `/`.
        #[arg(long)]
        ui: Option<PathBuf>,
    },
    /// Speak the Model Context Protocol on stdin and stdout, for an AI tool to call.
    Mcp,
}

fn absolute(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path;
    }
    std::env::current_dir().map(|cwd| cwd.join(&path)).unwrap_or(path)
}

fn print(value: &impl serde::Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    // An AI tool starts the MCP server from its own working directory, so a relative index path
    // written into a configuration file would not resolve. Absolutize before anything stores it.
    let database = absolute(cli.database.unwrap_or_else(vcs_service::default_database));
    let app = App::new(database.clone());

    match cli.command {
        Command::Detect { exact, days } => {
            if exact {
                print(&app.detect(days))?;
            } else {
                print(&app.probe())?;
            }
        }
        Command::Index { source, days } => {
            let progress = app.build_index(&IndexRequest { sources: source, days })?;
            print(&progress)?;
        }
        Command::Search { query, limit, format, offset, source, cwd_prefix, since, until, order, newest } => {
            let arguments = search_arguments(query, limit, format, offset, source, cwd_prefix, since, until, order, newest);
            print(&vcs_service::mcp::call_json(&app, "search_visible_conversations", &arguments)?)?
        }
        Command::Record { record_id } => {
            print(&vcs_service::mcp::call_json(&app, "read_conversation_record", &serde_json::json!({"record_id":record_id}))?)?
        }
        Command::Context { session, timestamp, span } => {
            print(&app.context(&session, timestamp.as_deref(), span)?)?
        }
        Command::Suggest { count } => print(&app.suggested_phrases(count)?)?,
        Command::Bench { query, limit, repeat } => {
            let mut best: Option<bench::BenchResult> = None;
            for _ in 0..repeat.max(1) {
                let result = bench::run(&app, &query, limit)?;
                best = Some(match best {
                    None => result,
                    Some(previous) => {
                        let mut merged = previous.clone();
                        merged.raw.elapsed_ms = previous.raw.elapsed_ms.min(result.raw.elapsed_ms);
                        merged.indexed.elapsed_ms =
                            previous.indexed.elapsed_ms.min(result.indexed.elapsed_ms);
                        merged.speedup = if merged.raw.available && merged.indexed.elapsed_ms > 0.0 {
                            Some(merged.raw.elapsed_ms / merged.indexed.elapsed_ms)
                        } else {
                            None
                        };
                        merged
                    }
                });
            }
            print(&best.expect("at least one run"))?;
        }
        Command::Map { days } => print(&map::build(&app, days)?)?,
        Command::Plan => {
            let executable = std::env::current_exe()?;
            print(&integrations::plan(
                &vcs_adapters::home_directory(),
                &executable,
                &database,
            ))?
        }
        Command::Connect { tool, confirm } => {
            let executable = std::env::current_exe()?;
            let tools = if tool.is_empty() {
                vec!["codex".to_string(), "claude".to_string()]
            } else {
                tool
            };
            print(&integrations::apply(
                &vcs_adapters::home_directory(),
                &executable,
                &database,
                &tools,
                confirm,
            )?)?
        }
        Command::Disconnect { tool } => {
            let tools = if tool.is_empty() {
                vec!["codex".to_string(), "claude".to_string(), "cursor".to_string()]
            } else {
                tool
            };
            print(&integrations::remove(&vcs_adapters::home_directory(), &tools)?)?
        }
        Command::Serve { address, ui } => server::serve(app, &address, ui)?,
        Command::Mcp => vcs_service::mcp::serve(app)?,
    }
    Ok(())
}

fn search_arguments(query: String, limit: usize, format: String, offset: usize,
    source: Option<String>, cwd_prefix: Option<String>, since: Option<String>, until: Option<String>,
    order: String, newest: bool) -> serde_json::Value {
    let mut value = serde_json::json!({"query":query,"limit":limit,"format":format,"offset":offset,"order":if newest {"newest"} else {&order}});
    for (key, argument) in [("source", source), ("cwd_prefix", cwd_prefix), ("since", since), ("until", until)] {
        if let Some(argument) = argument { value[key] = argument.into(); }
    }
    value
}

#[cfg(test)]
mod cli_tests {
    use super::*;
    #[test]
    fn defaults_are_snippets_with_twenty_hits() {
        match Cli::try_parse_from(["vcs", "search", "phrase"]).unwrap().command {
            Command::Search {format, limit, offset, order, ..} => {
                assert_eq!(format, "snippet"); assert_eq!(limit,20); assert_eq!(offset,0); assert_eq!(order,"oldest");
            }
            _ => panic!("wrong command"),
        }
    }
    #[test]
    fn parses_all_filters_and_record() {
        let cli = Cli::try_parse_from(["vcs","search","phrase","--format","full","--offset","4","--source","codex","--cwd-prefix","/tmp/project ü","--since","2026-01-01","--until","2027-01-01","--order","newest"]).unwrap();
        match cli.command { Command::Search {format, offset, source, cwd_prefix, since, until, order, ..} => {
            assert_eq!(format,"full"); assert_eq!(offset,4); assert_eq!(source.as_deref(),Some("codex"));
            assert_eq!(cwd_prefix.as_deref(),Some("/tmp/project ü")); assert!(since.is_some() && until.is_some());assert_eq!(order,"newest");
        }, _ => panic!("wrong command") }
        assert!(matches!(Cli::try_parse_from(["vcs","record","id"]).unwrap().command,Command::Record {..}));
    }
    #[test]
    fn preserves_newest_alias_and_rejects_invalid_options() {
        assert!(Cli::try_parse_from(["vcs","search","x","--newest"]).is_ok());
        assert!(Cli::try_parse_from(["vcs","search","x","--format","invalid"]).is_err());
        assert!(Cli::try_parse_from(["vcs","search","x","--source","invalid"]).is_err());
        assert!(Cli::try_parse_from(["vcs","search","x","--order","newest","--newest"]).is_err());
    }
}
