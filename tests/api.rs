use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use agent_terrace::{
    api_router, AppState, CommandError, CommandFuture, CommandOutput, CommandRunner, CommandSpec,
};
use axum::{body::Body, http::Request};
use serde_json::{json, Value};
use tower::ServiceExt;

struct FixtureRunner {
    calls: Mutex<Vec<CommandSpec>>,
    outputs: Mutex<VecDeque<CommandOutput>>,
}

impl FixtureRunner {
    fn new(outputs: Vec<CommandOutput>) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            outputs: Mutex::new(outputs.into()),
        }
    }
}

impl CommandRunner for FixtureRunner {
    fn run(&self, command: CommandSpec) -> CommandFuture<'_> {
        self.calls.lock().unwrap().push(command);
        let output = self.outputs.lock().unwrap().pop_front().unwrap();
        Box::pin(async move { Ok::<_, CommandError>(output) })
    }
}

fn output(success: bool, stdout: &str) -> CommandOutput {
    CommandOutput {
        success,
        stdout: stdout.into(),
        stderr: String::new(),
    }
}

const WHO: &str = "claude idle agent-terrace:1.0 (%38) /home/miyabi/a project\n";

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn lists_agents_from_the_registry() {
    let runner = Arc::new(FixtureRunner::new(vec![output(true, WHO)]));
    let app = api_router(AppState::new(runner.clone()));
    let response = app
        .oneshot(Request::get("/api/agents").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(
        response_json(response).await,
        json!({"agents": [{
            "name": "claude",
            "state": "idle",
            "location": "agent-terrace:1.0",
            "pane_id": "%38",
            "cwd": "/home/miyabi/a project"
        }]})
    );
    assert_eq!(
        runner.calls.lock().unwrap().as_slice(),
        &[CommandSpec {
            program: "agent-talk",
            args: vec!["who".into()]
        }]
    );
}

#[tokio::test]
async fn captures_only_a_registered_pane_with_separate_arguments() {
    let runner = Arc::new(FixtureRunner::new(vec![
        output(true, WHO),
        output(true, "\u{1b}[31mworking\u{1b}[0m\n"),
    ]));
    let app = api_router(AppState::new(runner.clone()));
    let response = app
        .oneshot(
            Request::get("/api/agents/%2538/screen")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(
        response_json(response).await,
        json!({"pane_id": "%38", "content": "\u{1b}[31mworking\u{1b}[0m\n"})
    );
    assert_eq!(
        runner.calls.lock().unwrap().as_slice(),
        &[
            CommandSpec {
                program: "agent-talk",
                args: vec!["who".into()]
            },
            CommandSpec {
                program: "tmux",
                args: vec!["capture-pane".into(), "-pet".into(), "%38".into()]
            }
        ]
    );
}

#[tokio::test]
async fn rejects_unknown_or_injected_panes_before_tmux() {
    for pane in ["%2599", "%2538%253Bdisplay-message"] {
        let runner = Arc::new(FixtureRunner::new(vec![output(true, WHO)]));
        let app = api_router(AppState::new(runner.clone()));
        let response = app
            .oneshot(
                Request::get(format!("/api/agents/{pane}/screen"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 404);
        assert_eq!(runner.calls.lock().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn reports_a_disappearing_pane_without_panicking() {
    let runner = Arc::new(FixtureRunner::new(vec![
        output(true, WHO),
        output(false, ""),
    ]));
    let app = api_router(AppState::new(runner));
    let response = app
        .oneshot(
            Request::get("/api/agents/%2538/screen")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 410);
    assert_eq!(
        response_json(response).await,
        json!({
            "code": "pane_unavailable",
            "message": "the requested pane disappeared before it could be captured"
        })
    );
}

#[tokio::test]
async fn has_no_terminal_input_route() {
    let runner = Arc::new(FixtureRunner::new(vec![]));
    let app = api_router(AppState::new(runner));

    for path in ["/api/send-keys", "/api/send", "/api/agents/%2538/input"] {
        let response = app
            .clone()
            .oneshot(Request::post(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), 404, "unexpected route: {path}");
    }
}
