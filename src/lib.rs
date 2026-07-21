use std::{future::Future, pin::Pin, sync::Arc};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::process::Command;

pub type CommandFuture<'a> =
    Pin<Box<dyn Future<Output = Result<CommandOutput, CommandError>> + Send + 'a>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandSpec {
    pub program: &'static str,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Error)]
#[error("command could not be started: {0}")]
pub struct CommandError(#[from] std::io::Error);

pub trait CommandRunner: Send + Sync {
    fn run(&self, command: CommandSpec) -> CommandFuture<'_>;
}

#[derive(Default)]
pub struct ProcessRunner;

impl CommandRunner for ProcessRunner {
    fn run(&self, command: CommandSpec) -> CommandFuture<'_> {
        Box::pin(async move {
            let output = Command::new(command.program)
                .args(&command.args)
                .output()
                .await?;
            Ok(CommandOutput {
                success: output.status.success(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
        })
    }
}

#[derive(Clone)]
pub struct AppState {
    runner: Arc<dyn CommandRunner>,
}

impl AppState {
    pub fn new(runner: Arc<dyn CommandRunner>) -> Self {
        Self { runner }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Agent {
    pub name: String,
    pub state: String,
    pub location: String,
    pub pane_id: String,
    pub cwd: String,
}

#[derive(Debug, Serialize)]
struct AgentsResponse {
    agents: Vec<Agent>,
}

#[derive(Debug, Serialize)]
struct ScreenResponse {
    pane_id: String,
    content: String,
}

#[derive(Debug, Error)]
enum ApiError {
    #[error("agent registry is unavailable")]
    RegistryUnavailable,
    #[error("the requested agent is not registered")]
    UnknownAgent,
    #[error("the requested pane disappeared before it could be captured")]
    PaneUnavailable,
}

#[derive(Serialize)]
struct ErrorResponse {
    code: &'static str,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            Self::RegistryUnavailable => (StatusCode::BAD_GATEWAY, "registry_unavailable"),
            Self::UnknownAgent => (StatusCode::NOT_FOUND, "unknown_agent"),
            Self::PaneUnavailable => (StatusCode::GONE, "pane_unavailable"),
        };
        let message = self.to_string();
        (status, Json(ErrorResponse { code, message })).into_response()
    }
}

pub fn api_router(state: AppState) -> Router {
    Router::new()
        .route("/api/agents", get(list_agents))
        .route("/api/agents/{pane}/screen", get(capture_screen))
        .with_state(state)
}

async fn list_agents(State(state): State<AppState>) -> Result<Json<AgentsResponse>, ApiError> {
    let agents = load_agents(&state).await?;
    Ok(Json(AgentsResponse { agents }))
}

async fn capture_screen(
    State(state): State<AppState>,
    Path(pane): Path<String>,
) -> Result<Json<ScreenResponse>, ApiError> {
    let agents = load_agents(&state).await?;
    if !agents.iter().any(|agent| agent.pane_id == pane) {
        return Err(ApiError::UnknownAgent);
    }

    let output = state
        .runner
        .run(CommandSpec {
            program: "tmux",
            args: vec!["capture-pane".into(), "-pet".into(), pane.clone()],
        })
        .await
        .map_err(|_| ApiError::PaneUnavailable)?;

    if !output.success {
        return Err(ApiError::PaneUnavailable);
    }

    Ok(Json(ScreenResponse {
        pane_id: pane,
        content: output.stdout,
    }))
}

async fn load_agents(state: &AppState) -> Result<Vec<Agent>, ApiError> {
    let output = state
        .runner
        .run(CommandSpec {
            program: "agent-talk",
            args: vec!["who".into()],
        })
        .await
        .map_err(|_| ApiError::RegistryUnavailable)?;

    if !output.success {
        return Err(ApiError::RegistryUnavailable);
    }

    parse_agents(&output.stdout).map_err(|_| ApiError::RegistryUnavailable)
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ParseAgentsError {
    #[error("invalid agent-talk who line {line}: {value}")]
    InvalidLine { line: usize, value: String },
}

pub fn parse_agents(output: &str) -> Result<Vec<Agent>, ParseAgentsError> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
        .map(|(index, line)| parse_agent_line(line, index + 1))
        .collect()
}

fn parse_agent_line(line: &str, line_number: usize) -> Result<Agent, ParseAgentsError> {
    let Some(pane_start) = line.find("(%") else {
        return Err(invalid_line(line_number, line));
    };
    let pane_tail = &line[pane_start..];
    let Some(pane_end_offset) = pane_tail.find(')') else {
        return Err(invalid_line(line_number, line));
    };
    let pane_end = pane_start + pane_end_offset;
    let pane_id = &line[pane_start + 1..pane_end];
    if !is_pane_id(pane_id) {
        return Err(invalid_line(line_number, line));
    }

    let fields: Vec<_> = line[..pane_start].split_whitespace().collect();
    if fields.len() != 3 {
        return Err(invalid_line(line_number, line));
    }
    let cwd = line[pane_end + 1..].trim_start();
    if cwd.is_empty() {
        return Err(invalid_line(line_number, line));
    }

    Ok(Agent {
        name: fields[0].into(),
        state: fields[1].into(),
        location: fields[2].into(),
        pane_id: pane_id.into(),
        cwd: cwd.into(),
    })
}

fn is_pane_id(value: &str) -> bool {
    value.strip_prefix('%').is_some_and(|digits| {
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn invalid_line(line: usize, value: &str) -> ParseAgentsError {
    ParseAgentsError::InvalidLine {
        line,
        value: value.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_who_output_and_preserves_spaces_in_cwd() {
        let output = concat!(
            "claude     idle  agent-terrace:1.0 (%38)  /home/miyabi/projects/sunny-side/agent-terrace\n",
            "codex      busy  work:2.1          (%40)  /tmp/a project/with spaces\n",
        );

        assert_eq!(
            parse_agents(output).unwrap(),
            vec![
                Agent {
                    name: "claude".into(),
                    state: "idle".into(),
                    location: "agent-terrace:1.0".into(),
                    pane_id: "%38".into(),
                    cwd: "/home/miyabi/projects/sunny-side/agent-terrace".into(),
                },
                Agent {
                    name: "codex".into(),
                    state: "busy".into(),
                    location: "work:2.1".into(),
                    pane_id: "%40".into(),
                    cwd: "/tmp/a project/with spaces".into(),
                },
            ]
        );
    }

    #[test]
    fn rejects_lines_without_a_strict_pane_anchor() {
        for line in [
            "claude idle terrace:1.0 %38 /tmp",
            "claude idle terrace:1.0 (%x) /tmp",
            "claude idle terrace:1.0 (%38) ",
            "claude idle extra terrace:1.0 (%38) /tmp",
        ] {
            assert!(parse_agents(line).is_err(), "accepted {line:?}");
        }
    }
}
