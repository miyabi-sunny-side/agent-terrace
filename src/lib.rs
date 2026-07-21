use std::{future::Future, io::Write, pin::Pin, process::Stdio, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type CommandFuture<'a> =
    Pin<Box<dyn Future<Output = Result<CommandOutput, CommandError>> + Send + 'a>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandSpec {
    pub program: &'static str,
    pub args: Vec<String>,
    pub stdin: Option<String>,
    pub env_remove: Vec<&'static str>,
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
            let output = tokio::task::spawn_blocking(move || {
                let mut process = std::process::Command::new(command.program);
                process.args(&command.args);
                for name in command.env_remove {
                    process.env_remove(name);
                }
                if command.stdin.is_some() {
                    process.stdin(Stdio::piped());
                }
                process.stdout(Stdio::piped()).stderr(Stdio::piped());

                let mut child = process.spawn()?;
                if let Some(input) = command.stdin {
                    child
                        .stdin
                        .take()
                        .expect("stdin was configured as piped")
                        .write_all(input.as_bytes())?;
                }
                child.wait_with_output()
            })
            .await
            .map_err(std::io::Error::other)??;
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
    #[error("letter history is unavailable")]
    LetterHistoryUnavailable,
    #[error("letter delivery failed")]
    LetterDeliveryFailed,
    #[error("invalid letter request")]
    InvalidLetter,
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
            Self::LetterHistoryUnavailable => {
                (StatusCode::BAD_GATEWAY, "letter_history_unavailable")
            }
            Self::LetterDeliveryFailed => (StatusCode::BAD_GATEWAY, "letter_delivery_failed"),
            Self::InvalidLetter => (StatusCode::BAD_REQUEST, "invalid_letter"),
        };
        let message = self.to_string();
        (status, Json(ErrorResponse { code, message })).into_response()
    }
}

pub fn api_router(state: AppState) -> Router {
    Router::new()
        .route("/api/agents", get(list_agents))
        .route("/api/agents/{pane}/screen", get(capture_screen))
        .route("/api/skills", get(list_skills))
        .route("/api/letters", get(list_letters).post(send_letter))
        .with_state(state)
}

const SKILLS: [&str; 2] = ["deliver", "commit"];
const MAX_BODY_BYTES: usize = 16_384;

#[derive(Debug, Serialize)]
struct SkillsResponse {
    skills: [&'static str; 2],
}

async fn list_skills() -> Json<SkillsResponse> {
    Json(SkillsResponse { skills: SKILLS })
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct LettersQuery {
    after: Option<u64>,
    limit: Option<u16>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MailboxResponse {
    version: u8,
    mailbox: String,
    events: Vec<MailboxEvent>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MailboxEvent {
    id: u64,
    created_at: String,
    mailbox: String,
    source_label: String,
    direction: Direction,
    body: String,
    skill: Option<String>,
    target_name: String,
    target_pane: String,
    reply_to: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Direction {
    Out,
    In,
}

async fn list_letters(
    State(state): State<AppState>,
    Query(query): Query<LettersQuery>,
) -> Result<Json<MailboxResponse>, ApiError> {
    let limit = query.limit.unwrap_or(500);
    if !(1..=500).contains(&limit) {
        return Err(ApiError::InvalidLetter);
    }

    let mut args = vec!["mailbox-list-v1".into(), "mobile".into()];
    if let Some(after) = query.after {
        args.extend(["--after".into(), after.to_string()]);
    }
    args.extend(["--limit".into(), limit.to_string()]);
    let output = state
        .runner
        .run(CommandSpec {
            program: "agent-talk",
            args,
            stdin: None,
            env_remove: vec!["TMUX_PANE"],
        })
        .await
        .map_err(|_| ApiError::LetterHistoryUnavailable)?;
    if !output.success {
        return Err(ApiError::LetterHistoryUnavailable);
    }

    let response: MailboxResponse =
        serde_json::from_str(&output.stdout).map_err(|_| ApiError::LetterHistoryUnavailable)?;
    validate_mailbox_response(&response, query.after, u64::from(limit))?;
    Ok(Json(response))
}

fn validate_mailbox_response(
    response: &MailboxResponse,
    after: Option<u64>,
    limit: u64,
) -> Result<(), ApiError> {
    if response.version != 1
        || response.mailbox != "mobile"
        || response.events.len() as u64 > limit
        || response.events.iter().any(|event| {
            event.mailbox != "mobile"
                || !is_pane_id(&event.target_pane)
                || after.is_some_and(|after| event.id <= after)
        })
        || response
            .events
            .windows(2)
            .any(|pair| pair[0].id >= pair[1].id)
    {
        return Err(ApiError::LetterHistoryUnavailable);
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SendLetterRequest {
    agent: String,
    skill: Option<String>,
    body: String,
}

#[derive(Debug, Serialize)]
struct SendLetterResponse {
    id: u64,
    status: &'static str,
}

async fn send_letter(
    State(state): State<AppState>,
    Json(request): Json<SendLetterRequest>,
) -> Result<(StatusCode, Json<SendLetterResponse>), ApiError> {
    if !is_pane_id(&request.agent)
        || request.body.trim().is_empty()
        || request.body.len() > MAX_BODY_BYTES
    {
        return Err(ApiError::InvalidLetter);
    }
    if let Some(skill) = request.skill.as_deref() {
        if !is_safe_token(skill) || !SKILLS.contains(&skill) {
            return Err(ApiError::InvalidLetter);
        }
    }

    let agents = load_agents(&state)
        .await
        .map_err(|_| ApiError::LetterDeliveryFailed)?;
    if !agents.iter().any(|agent| agent.pane_id == request.agent) {
        return Err(ApiError::UnknownAgent);
    }

    let mut args = vec![
        "send".into(),
        request.agent.clone(),
        "--from".into(),
        "mobile".into(),
    ];
    if let Some(skill) = request.skill {
        args.extend(["--skill".into(), skill]);
    }
    let output = state
        .runner
        .run(CommandSpec {
            program: "agent-talk",
            args,
            stdin: Some(request.body),
            env_remove: vec!["TMUX_PANE"],
        })
        .await
        .map_err(|_| ApiError::LetterDeliveryFailed)?;
    if !output.success {
        return Err(ApiError::LetterDeliveryFailed);
    }
    let (status, id) = parse_delivery_stdout(&output.stdout, &request.agent)?;
    Ok((StatusCode::CREATED, Json(SendLetterResponse { id, status })))
}

fn is_safe_token(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn parse_delivery_stdout(
    stdout: &str,
    expected_pane: &str,
) -> Result<(&'static str, u64), ApiError> {
    let line = stdout
        .strip_suffix("\r\n")
        .or_else(|| stdout.strip_suffix('\n'))
        .unwrap_or(stdout);
    if line.contains(['\r', '\n']) {
        return Err(ApiError::LetterDeliveryFailed);
    }
    let (status, rest) = if let Some(rest) = line.strip_prefix("sent -> ") {
        ("sent", rest)
    } else if let Some(rest) = line.strip_prefix("queued (busy) -> ") {
        ("queued", rest)
    } else {
        return Err(ApiError::LetterDeliveryFailed);
    };
    let (target, id) = rest
        .rsplit_once(": #")
        .ok_or(ApiError::LetterDeliveryFailed)?;
    let target_matches = target == expected_pane
        || target
            .strip_suffix(')')
            .and_then(|target| target.rsplit_once(" ("))
            .is_some_and(|(_, pane)| pane == expected_pane);
    if !target_matches || id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ApiError::LetterDeliveryFailed);
    }
    let id = id
        .parse::<u64>()
        .map_err(|_| ApiError::LetterDeliveryFailed)?;
    Ok((status, id))
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
            stdin: None,
            env_remove: vec![],
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
            stdin: None,
            env_remove: vec![],
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
