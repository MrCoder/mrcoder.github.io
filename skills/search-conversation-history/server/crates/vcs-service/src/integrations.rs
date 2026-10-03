//! Connect the local search service to the AI tools already installed on this computer.
//!
//! Nothing here writes anything until `apply` is called with `confirmed`. `plan` exists so the
//! interface can show the exact file, the exact block, and the exact reversal step first.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Name of the entry written into each tool's configuration.
pub const SERVER_NAME: &str = "fast-conversation-search";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFormat {
    Toml,
    Json,
}

/// What the product will start, where it stores data, and what it will not do.
#[derive(Debug, Clone, Serialize)]
pub struct ServiceDisclosure {
    pub executable: String,
    pub command_line: String,
    pub index_path: String,
    pub transport: String,
    pub network: String,
    pub reads: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolTarget {
    pub tool: String,
    pub label: String,
    pub config_path: String,
    pub format: ConfigFormat,
    /// The tool's history store exists on this computer.
    pub detected: bool,
    pub config_exists: bool,
    /// An entry under `SERVER_NAME` is already present.
    pub already_connected: bool,
    /// Exactly what will be added, in the file's own syntax.
    pub snippet: String,
    pub revoke: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IntegrationPlan {
    pub service: ServiceDisclosure,
    pub tools: Vec<ToolTarget>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyOutcome {
    pub tool: String,
    /// `connected`, `already-connected`, `skipped`, or `failed`.
    pub state: String,
    pub config_path: String,
    pub backup_path: Option<String>,
    pub error: Option<String>,
}

/// Where each tool keeps the configuration this product would extend.
pub fn config_path(tool: &str, home: &Path) -> Option<PathBuf> {
    match tool {
        "codex" => Some(home.join(".codex/config.toml")),
        "claude" => Some(home.join(".claude.json")),
        "cursor" => Some(home.join(".cursor/mcp.json")),
        _ => None,
    }
}

fn format_of(tool: &str) -> ConfigFormat {
    match tool {
        "codex" => ConfigFormat::Toml,
        _ => ConfigFormat::Json,
    }
}

fn snippet(tool: &str, executable: &str, database: &str) -> String {
    match format_of(tool) {
        ConfigFormat::Toml => format!(
            "[mcp_servers.{SERVER_NAME}]\ncommand = \"{executable}\"\nargs = [\"mcp\", \"--database\", \"{database}\"]\n"
        ),
        ConfigFormat::Json => serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": {
                SERVER_NAME: {
                    "command": executable,
                    "args": ["mcp", "--database", database]
                }
            }
        }))
        .expect("static json"),
    }
}

fn revoke_text(tool: &str, path: &Path) -> String {
    match format_of(tool) {
        ConfigFormat::Toml => format!(
            "Delete the [mcp_servers.{SERVER_NAME}] block from {}, or restore the .vcs-backup- file written next to it.",
            path.display()
        ),
        ConfigFormat::Json => format!(
            "Delete the \"{SERVER_NAME}\" key under mcpServers in {}, or restore the .vcs-backup- file written next to it.",
            path.display()
        ),
    }
}

fn contains_entry(content: &str, tool: &str) -> bool {
    match format_of(tool) {
        ConfigFormat::Toml => content.contains(&format!("[mcp_servers.{SERVER_NAME}]")),
        ConfigFormat::Json => serde_json::from_str::<serde_json::Value>(content)
            .ok()
            .and_then(|value| value.get("mcpServers").cloned())
            .and_then(|servers| servers.get(SERVER_NAME).cloned())
            .is_some(),
    }
}

/// Describe every change before anything is written.
pub fn plan(home: &Path, executable: &Path, database: &Path) -> IntegrationPlan {
    let executable_text = executable.display().to_string();
    let database_text = database.display().to_string();
    let probes = vcs_adapters::probe();

    let tools = vcs_adapters::SourceKind::ALL
        .iter()
        .filter_map(|source| {
            let tool = source.as_str();
            let path = config_path(tool, home)?;
            let content = std::fs::read_to_string(&path).ok();
            Some(ToolTarget {
                tool: tool.to_string(),
                label: source.label().to_string(),
                format: format_of(tool),
                detected: probes
                    .iter()
                    .find(|probe| probe.source == *source)
                    .map_or(false, |probe| probe.installed),
                config_exists: content.is_some(),
                already_connected: content
                    .as_deref()
                    .map_or(false, |content| contains_entry(content, tool)),
                snippet: snippet(tool, &executable_text, &database_text),
                revoke: revoke_text(tool, &path),
                config_path: path.display().to_string(),
            })
        })
        .collect();

    IntegrationPlan {
        service: ServiceDisclosure {
            executable: executable_text.clone(),
            command_line: format!("{executable_text} mcp --database {database_text}"),
            index_path: database_text,
            transport: "Standard input and output of a local child process. No port is opened.".into(),
            network: "The service makes no network connection and sends no conversation anywhere.".into(),
            reads: "The service answers from the local index only. Original history files stay where they are.".into(),
        },
        tools,
    }
}

fn backup(path: &Path) -> Result<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }
    let stamp = crate::now_ms();
    let backup = path.with_file_name(format!(
        "{}.vcs-backup-{stamp}",
        path.file_name().and_then(|name| name.to_str()).unwrap_or("config")
    ));
    std::fs::copy(path, &backup)?;
    Ok(Some(backup))
}

fn write_toml(path: &Path, snippet: &str) -> Result<()> {
    let mut content = std::fs::read_to_string(path).unwrap_or_default();
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    if !content.is_empty() {
        content.push('\n');
    }
    content.push_str(snippet);
    std::fs::write(path, content)?;
    Ok(())
}

fn write_json(path: &Path, executable: &str, database: &str) -> Result<()> {
    let mut document: serde_json::Value = match std::fs::read_to_string(path) {
        Ok(content) if !content.trim().is_empty() => serde_json::from_str(&content)
            .map_err(|error| anyhow!("{} is not valid JSON: {error}", path.display()))?,
        _ => serde_json::json!({}),
    };
    if !document.is_object() {
        return Err(anyhow!("{} does not hold a JSON object", path.display()));
    }
    let servers = document
        .as_object_mut()
        .expect("checked object")
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));
    if !servers.is_object() {
        return Err(anyhow!("mcpServers in {} is not an object", path.display()));
    }
    servers.as_object_mut().expect("checked object").insert(
        SERVER_NAME.to_string(),
        serde_json::json!({ "command": executable, "args": ["mcp", "--database", database] }),
    );
    std::fs::write(path, serde_json::to_string_pretty(&document)? + "\n")?;
    Ok(())
}

/// Write the planned entries. `confirmed` must be true; without it nothing is touched.
pub fn apply(
    home: &Path,
    executable: &Path,
    database: &Path,
    tools: &[String],
    confirmed: bool,
) -> Result<Vec<ApplyOutcome>> {
    if !confirmed {
        return Err(anyhow!("integration changes require explicit confirmation"));
    }
    let executable_text = executable.display().to_string();
    let database_text = database.display().to_string();
    let mut outcomes = Vec::new();
    for tool in tools {
        let Some(path) = config_path(tool, home) else {
            outcomes.push(ApplyOutcome {
                tool: tool.clone(),
                state: "skipped".into(),
                config_path: String::new(),
                backup_path: None,
                error: Some(format!("unknown tool: {tool}")),
            });
            continue;
        };
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        if !existing.is_empty() && contains_entry(&existing, tool) {
            outcomes.push(ApplyOutcome {
                tool: tool.clone(),
                state: "already-connected".into(),
                config_path: path.display().to_string(),
                backup_path: None,
                error: None,
            });
            continue;
        }
        let outcome = (|| -> Result<Option<PathBuf>> {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let backup_path = backup(&path)?;
            match format_of(tool) {
                ConfigFormat::Toml => write_toml(&path, &snippet(tool, &executable_text, &database_text))?,
                ConfigFormat::Json => write_json(&path, &executable_text, &database_text)?,
            }
            Ok(backup_path)
        })();
        match outcome {
            Ok(backup_path) => outcomes.push(ApplyOutcome {
                tool: tool.clone(),
                state: "connected".into(),
                config_path: path.display().to_string(),
                backup_path: backup_path.map(|path| path.display().to_string()),
                error: None,
            }),
            Err(error) => outcomes.push(ApplyOutcome {
                tool: tool.clone(),
                state: "failed".into(),
                config_path: path.display().to_string(),
                backup_path: None,
                error: Some(error.to_string()),
            }),
        }
    }
    Ok(outcomes)
}

/// Remove the entry this product added, leaving every other setting in place.
///
/// The completion screen offers disconnection, so reversal has to be an action the product performs,
/// not a paragraph telling the reader which lines to delete.
pub fn remove(home: &Path, tools: &[String]) -> Result<Vec<ApplyOutcome>> {
    let mut outcomes = Vec::new();
    for tool in tools {
        let Some(path) = config_path(tool, home) else {
            outcomes.push(ApplyOutcome {
                tool: tool.clone(),
                state: "skipped".into(),
                config_path: String::new(),
                backup_path: None,
                error: Some(format!("unknown tool: {tool}")),
            });
            continue;
        };
        let Ok(content) = std::fs::read_to_string(&path) else {
            outcomes.push(ApplyOutcome {
                tool: tool.clone(),
                state: "not-connected".into(),
                config_path: path.display().to_string(),
                backup_path: None,
                error: None,
            });
            continue;
        };
        if !contains_entry(&content, tool) {
            outcomes.push(ApplyOutcome {
                tool: tool.clone(),
                state: "not-connected".into(),
                config_path: path.display().to_string(),
                backup_path: None,
                error: None,
            });
            continue;
        }
        let outcome = (|| -> Result<Option<PathBuf>> {
            let backup_path = backup(&path)?;
            match format_of(tool) {
                ConfigFormat::Toml => std::fs::write(&path, without_toml_block(&content))?,
                ConfigFormat::Json => {
                    let mut document: serde_json::Value = serde_json::from_str(&content)
                        .map_err(|error| anyhow!("{} is not valid JSON: {error}", path.display()))?;
                    if let Some(servers) = document
                        .get_mut("mcpServers")
                        .and_then(|value| value.as_object_mut())
                    {
                        servers.remove(SERVER_NAME);
                    }
                    std::fs::write(&path, serde_json::to_string_pretty(&document)? + "\n")?;
                }
            }
            Ok(backup_path)
        })();
        match outcome {
            Ok(backup_path) => outcomes.push(ApplyOutcome {
                tool: tool.clone(),
                state: "disconnected".into(),
                config_path: path.display().to_string(),
                backup_path: backup_path.map(|path| path.display().to_string()),
                error: None,
            }),
            Err(error) => outcomes.push(ApplyOutcome {
                tool: tool.clone(),
                state: "failed".into(),
                config_path: path.display().to_string(),
                backup_path: None,
                error: Some(error.to_string()),
            }),
        }
    }
    Ok(outcomes)
}

/// Drop the `[mcp_servers.<name>]` table and its keys, keeping every other table intact.
fn without_toml_block(content: &str) -> String {
    let header = format!("[mcp_servers.{SERVER_NAME}]");
    let mut out: Vec<&str> = Vec::new();
    let mut inside = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == header {
            inside = true;
            // Drop a blank line that only separated this block from the previous one.
            while out.last().map_or(false, |last| last.trim().is_empty()) {
                out.pop();
            }
            continue;
        }
        if inside {
            if trimmed.starts_with('[') {
                inside = false;
            } else {
                continue;
            }
        }
        out.push(line);
    }
    let mut text = out.join("\n");
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_home(name: &str) -> PathBuf {
        let home = std::env::temp_dir().join(format!("vcs-integration-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        home
    }

    fn paths() -> (PathBuf, PathBuf) {
        (PathBuf::from("/opt/vcs"), PathBuf::from("/tmp/index.sqlite"))
    }

    #[test]
    fn nothing_is_written_without_confirmation() {
        let home = temporary_home("unconfirmed");
        let (executable, database) = paths();
        let result = apply(&home, &executable, &database, &["codex".into()], false);
        assert!(result.is_err());
        assert!(!home.join(".codex/config.toml").exists());
    }

    #[test]
    fn the_plan_shows_the_exact_block_for_each_format() {
        let home = temporary_home("plan");
        let (executable, database) = paths();
        let plan = plan(&home, &executable, &database);
        let codex = plan.tools.iter().find(|tool| tool.tool == "codex").unwrap();
        assert!(codex.snippet.contains("[mcp_servers.fast-conversation-search]"));
        assert!(codex.snippet.contains("/opt/vcs"));
        let claude = plan.tools.iter().find(|tool| tool.tool == "claude").unwrap();
        assert!(claude.snippet.contains("\"mcpServers\""));
        assert!(plan.service.network.contains("no network connection"));
    }

    #[test]
    fn an_existing_toml_config_keeps_its_content_and_gains_a_backup() {
        let home = temporary_home("toml");
        let (executable, database) = paths();
        let path = home.join(".codex/config.toml");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "model = \"gpt-5\"\n").unwrap();
        let outcomes = apply(&home, &executable, &database, &["codex".into()], true).unwrap();
        assert_eq!(outcomes[0].state, "connected");
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.starts_with("model = \"gpt-5\"\n"));
        assert!(written.contains("[mcp_servers.fast-conversation-search]"));
        let backup = outcomes[0].backup_path.as_ref().unwrap();
        assert_eq!(std::fs::read_to_string(backup).unwrap(), "model = \"gpt-5\"\n");
    }

    #[test]
    fn an_existing_json_config_keeps_its_other_keys() {
        let home = temporary_home("json");
        let (executable, database) = paths();
        let path = home.join(".claude.json");
        std::fs::write(&path, r#"{"theme":"dark","mcpServers":{"other":{"command":"x"}}}"#).unwrap();
        apply(&home, &executable, &database, &["claude".into()], true).unwrap();
        let document: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(document["theme"], "dark");
        assert_eq!(document["mcpServers"]["other"]["command"], "x");
        assert_eq!(document["mcpServers"][SERVER_NAME]["command"], "/opt/vcs");
    }

    #[test]
    fn applying_twice_reports_already_connected_and_writes_nothing_new() {
        let home = temporary_home("twice");
        let (executable, database) = paths();
        apply(&home, &executable, &database, &["claude".into()], true).unwrap();
        let first = std::fs::read_to_string(home.join(".claude.json")).unwrap();
        let outcomes = apply(&home, &executable, &database, &["claude".into()], true).unwrap();
        assert_eq!(outcomes[0].state, "already-connected");
        assert_eq!(std::fs::read_to_string(home.join(".claude.json")).unwrap(), first);
    }

    #[test]
    fn disconnecting_removes_only_this_products_entry_from_toml() {
        let home = temporary_home("remove-toml");
        let (executable, database) = paths();
        let path = home.join(".codex/config.toml");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "model = \"gpt-5\"\n\n[mcp_servers.other]\ncommand = \"x\"\n").unwrap();
        apply(&home, &executable, &database, &["codex".into()], true).unwrap();
        assert!(std::fs::read_to_string(&path).unwrap().contains(SERVER_NAME));

        let outcomes = remove(&home, &["codex".into()]).unwrap();
        assert_eq!(outcomes[0].state, "disconnected");
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(!written.contains(SERVER_NAME));
        assert!(written.contains("model = \"gpt-5\""));
        assert!(written.contains("[mcp_servers.other]"));
    }

    #[test]
    fn disconnecting_removes_only_this_products_key_from_json() {
        let home = temporary_home("remove-json");
        let (executable, database) = paths();
        let path = home.join(".claude.json");
        std::fs::write(&path, r#"{"theme":"dark","mcpServers":{"other":{"command":"x"}}}"#).unwrap();
        apply(&home, &executable, &database, &["claude".into()], true).unwrap();
        remove(&home, &["claude".into()]).unwrap();
        let document: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(document["theme"], "dark");
        assert_eq!(document["mcpServers"]["other"]["command"], "x");
        assert!(document["mcpServers"].get(SERVER_NAME).is_none());
    }

    #[test]
    fn disconnecting_a_tool_that_was_never_connected_changes_nothing() {
        let home = temporary_home("remove-absent");
        let path = home.join(".claude.json");
        std::fs::write(&path, r#"{"theme":"dark"}"#).unwrap();
        let outcomes = remove(&home, &["claude".into(), "codex".into()]).unwrap();
        assert_eq!(outcomes[0].state, "not-connected");
        assert_eq!(outcomes[1].state, "not-connected");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), r#"{"theme":"dark"}"#);
    }

    #[test]
    fn a_config_file_that_is_not_json_fails_that_tool_without_writing() {
        let home = temporary_home("broken");
        let (executable, database) = paths();
        let path = home.join(".claude.json");
        std::fs::write(&path, "not json at all").unwrap();
        let outcomes = apply(&home, &executable, &database, &["claude".into()], true).unwrap();
        assert_eq!(outcomes[0].state, "failed");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "not json at all");
    }
}
