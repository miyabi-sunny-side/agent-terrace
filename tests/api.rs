use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
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

struct SkillFixture {
    home: PathBuf,
}

static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

impl SkillFixture {
    fn new() -> Self {
        let home = std::env::temp_dir().join(format!(
            "agent-terrace-skills-{}-{}",
            std::process::id(),
            NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&home).unwrap();
        Self { home }
    }

    fn add(&self, relative: impl AsRef<Path>) {
        let directory = self.home.join(relative);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("SKILL.md"), "---\nname: fixture\n---\n").unwrap();
    }
}

impl Drop for SkillFixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.home).unwrap();
    }
}

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
            args: vec!["who".into()],
            stdin: None,
            env_remove: vec![],
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
                args: vec!["who".into()],
                stdin: None,
                env_remove: vec![],
            },
            CommandSpec {
                program: "tmux",
                args: vec!["capture-pane".into(), "-pet".into(), "%38".into()],
                stdin: None,
                env_remove: vec![],
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

fn mailbox(events: Value) -> String {
    json!({"version": 1, "mailbox": "mobile", "events": events}).to_string()
}

fn event(id: u64) -> Value {
    json!({
        "id": id,
        "created_at": "2026-07-21T11:00:00Z",
        "mailbox": "mobile",
        "source_label": "mobile",
        "direction": "out",
        "body": "hello",
        "skill": null,
        "target_name": "claude",
        "target_pane": "%38",
        "reply_to": null
    })
}

fn json_request(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn lists_installed_skills_for_the_registered_agent() {
    let fixture = SkillFixture::new();
    fixture.add(".claude/skills/deliver");
    fixture.add(".claude/skills/bump-tag");
    fs::create_dir_all(fixture.home.join(".claude/skills/not-a-skill")).unwrap();
    let runner = Arc::new(FixtureRunner::new(vec![output(true, WHO)]));
    let app = api_router(AppState::new(runner.clone()).with_home_dir(&fixture.home));
    let response = app
        .oneshot(
            Request::get("/api/agents/%2538/skills")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(
        response_json(response).await,
        json!({"skills": ["bump-tag", "deliver"]})
    );
    assert_eq!(runner.calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn merges_codex_common_and_system_skills_without_duplicates() {
    let fixture = SkillFixture::new();
    fixture.add(".agents/skills/bump-tag");
    fixture.add(".agents/skills/skill-creator");
    fixture.add(".codex/skills/.system/imagegen");
    fixture.add(".codex/skills/.system/skill-creator");
    let who = "codex idle agent-terrace:1.1 (%40) /home/miyabi/project\n";
    let app = api_router(
        AppState::new(Arc::new(FixtureRunner::new(vec![output(true, who)])))
            .with_home_dir(&fixture.home),
    );
    let response = app
        .oneshot(
            Request::get("/api/agents/%2540/skills")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(
        response_json(response).await,
        json!({"skills": ["bump-tag", "imagegen", "skill-creator"]})
    );
}

#[tokio::test]
async fn returns_no_skills_for_a_runtime_without_delivery_syntax() {
    let fixture = SkillFixture::new();
    fixture.add(".cursor/skills/deliver");
    let who = "cursor idle agent-terrace:1.2 (%42) /home/miyabi/project\n";
    let app = api_router(
        AppState::new(Arc::new(FixtureRunner::new(vec![output(true, who)])))
            .with_home_dir(&fixture.home),
    );
    let response = app
        .oneshot(
            Request::get("/api/agents/%2542/skills")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(response_json(response).await, json!({"skills": []}));
}

#[tokio::test]
async fn rejects_unknown_panes_and_removes_the_global_skill_endpoint() {
    let runner = Arc::new(FixtureRunner::new(vec![output(true, WHO)]));
    let response = api_router(AppState::new(runner))
        .oneshot(
            Request::get("/api/agents/%2599/skills")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 404);

    let response = api_router(AppState::new(Arc::new(FixtureRunner::new(vec![]))))
        .oneshot(Request::get("/api/skills").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn lists_letters_with_validated_cursor_arguments() {
    let history = mailbox(json!([event(42), event(43)]));
    let runner = Arc::new(FixtureRunner::new(vec![output(true, &history)]));
    let app = api_router(AppState::new(runner.clone()));
    let response = app
        .oneshot(
            Request::get("/api/letters?after=41&limit=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(response_json(response).await["events"][0]["id"], 42);
    assert_eq!(
        runner.calls.lock().unwrap().as_slice(),
        &[CommandSpec {
            program: "agent-talk",
            args: vec![
                "mailbox-list-v1".into(),
                "mobile".into(),
                "--after".into(),
                "41".into(),
                "--limit".into(),
                "2".into(),
            ],
            stdin: None,
            env_remove: vec!["TMUX_PANE"],
        }]
    );
}

#[tokio::test]
async fn letter_list_defaults_to_the_maximum_limit() {
    let runner = Arc::new(FixtureRunner::new(vec![output(true, &mailbox(json!([])))]));
    let app = api_router(AppState::new(runner.clone()));
    let response = app
        .oneshot(Request::get("/api/letters").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(
        runner.calls.lock().unwrap()[0].args,
        ["mailbox-list-v1", "mobile", "--limit", "500"]
    );
}

#[tokio::test]
async fn rejects_invalid_letter_list_queries_without_running_a_command() {
    for query in ["limit=0", "limit=501", "limit=nope", "after=-1", "extra=1"] {
        let runner = Arc::new(FixtureRunner::new(vec![]));
        let app = api_router(AppState::new(runner.clone()));
        let response = app
            .oneshot(
                Request::get(format!("/api/letters?{query}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 400, "accepted query {query}");
        assert!(runner.calls.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn rejects_untrusted_mailbox_schemas_and_ordering() {
    let invalid = [
        json!({"version": 2, "mailbox": "mobile", "events": []}),
        json!({"version": 1, "mailbox": "other", "events": []}),
        json!({"version": 1, "mailbox": "mobile", "events": [event(2), event(1)]}),
        json!({"version": 1, "mailbox": "mobile", "events": [event(1), event(1)]}),
        json!({"version": 1, "mailbox": "mobile", "events": [{
            "id": 1,
            "created_at": "2026-07-21T11:00:00Z",
            "mailbox": "mobile",
            "source_label": "mobile",
            "direction": "out",
            "body": "hello",
            "skill": null,
            "target_name": "claude",
            "target_pane": "%38;display",
            "reply_to": null
        }]}),
        json!({"version": 1, "mailbox": "mobile", "events": [{"id": 1}]}),
        json!({"version": 1, "mailbox": "mobile", "events": [], "extra": true}),
    ];

    for schema in invalid {
        let runner = Arc::new(FixtureRunner::new(vec![output(true, &schema.to_string())]));
        let response = api_router(AppState::new(runner))
            .oneshot(Request::get("/api/letters").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), 502, "accepted schema {schema}");
    }
}

#[tokio::test]
async fn enforces_after_and_limit_on_mailbox_output() {
    for history in [
        mailbox(json!([event(40)])),
        mailbox(json!([event(42), event(43)])),
    ] {
        let runner = Arc::new(FixtureRunner::new(vec![output(true, &history)]));
        let response = api_router(AppState::new(runner))
            .oneshot(
                Request::get("/api/letters?after=41&limit=1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 502);
    }
}

#[tokio::test]
async fn hides_mailbox_command_failures() {
    let runner = Arc::new(FixtureRunner::new(vec![CommandOutput {
        success: false,
        stdout: String::new(),
        stderr: "private socket path".into(),
    }]));
    let response = api_router(AppState::new(runner))
        .oneshot(Request::get("/api/letters").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), 502);
    let body = response_json(response).await;
    assert_eq!(body["code"], "letter_history_unavailable");
    assert!(!body.to_string().contains("private socket path"));
}

#[tokio::test]
async fn sends_a_letter_body_only_through_stdin() {
    let fixture = SkillFixture::new();
    fixture.add(".claude/skills/bump-tag");
    let runner = Arc::new(FixtureRunner::new(vec![
        output(true, WHO),
        output(true, "sent -> %38: #73\n"),
    ]));
    let app = api_router(AppState::new(runner.clone()).with_home_dir(&fixture.home));
    let response = app
        .oneshot(json_request(
            "POST",
            "/api/letters",
            json!({"agent": "%38", "skill": "bump-tag", "body": "  secret body  "}),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), 201);
    assert_eq!(
        response_json(response).await,
        json!({"id": 73, "status": "sent"})
    );
    assert_eq!(
        runner.calls.lock().unwrap()[1],
        CommandSpec {
            program: "agent-talk",
            args: vec![
                "send".into(),
                "%38".into(),
                "--from".into(),
                "mobile".into(),
                "--skill".into(),
                "bump-tag".into(),
            ],
            stdin: Some("  secret body  ".into()),
            env_remove: vec!["TMUX_PANE"],
        }
    );
}

#[tokio::test]
async fn parses_a_queued_delivery() {
    let runner = Arc::new(FixtureRunner::new(vec![
        output(true, WHO),
        output(true, "queued (busy) -> claude (%38): #74\r\n"),
    ]));
    let response = api_router(AppState::new(runner))
        .oneshot(json_request(
            "POST",
            "/api/letters",
            json!({"agent": "%38", "skill": null, "body": "hello"}),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), 201);
    assert_eq!(
        response_json(response).await,
        json!({"id": 74, "status": "queued"})
    );
}

#[tokio::test]
async fn rejects_invalid_letter_inputs_before_delivery() {
    let long_body = "a".repeat(16_385);
    let long_utf8_body = "界".repeat(5_462);
    let cases = [
        json!({"agent": "%38;display", "skill": null, "body": "hello"}),
        json!({"agent": "%38", "skill": "unknown", "body": "hello"}),
        json!({"agent": "%38", "skill": "deliver;bad", "body": "hello"}),
        json!({"agent": "%38", "skill": null, "body": "  \n "}),
        json!({"agent": "%38", "skill": null, "body": long_body}),
        json!({"agent": "%38", "skill": null, "body": long_utf8_body}),
    ];
    for request in cases {
        let fixture = SkillFixture::new();
        let runner = Arc::new(FixtureRunner::new(vec![output(true, WHO)]));
        let response = api_router(AppState::new(runner.clone()).with_home_dir(&fixture.home))
            .oneshot(json_request("POST", "/api/letters", request))
            .await
            .unwrap();
        assert_eq!(response.status(), 400);
        assert!(runner.calls.lock().unwrap().len() <= 1);
    }
}

#[tokio::test]
async fn reports_an_agent_that_left_before_delivery() {
    let runner = Arc::new(FixtureRunner::new(vec![output(true, WHO)]));
    let response = api_router(AppState::new(runner))
        .oneshot(json_request(
            "POST",
            "/api/letters",
            json!({"agent": "%99", "skill": null, "body": "hello"}),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), 404);
    assert_eq!(response_json(response).await["code"], "unknown_agent");
}

#[tokio::test]
async fn requires_json_for_letter_delivery() {
    let runner = Arc::new(FixtureRunner::new(vec![]));
    let response = api_router(AppState::new(runner))
        .oneshot(
            Request::post("/api/letters")
                .body(Body::from("agent=%38&body=hello"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 415);
}

#[tokio::test]
async fn rejects_malformed_delivery_success_and_hides_failures() {
    for stdout in [
        "sent -> %38: #1\nextra\n",
        "sent -> %38: #nope\n",
        "sent -> : #1\n",
        "sent -> %99: #1\n",
        "delivered -> %38: #1\n",
    ] {
        let runner = Arc::new(FixtureRunner::new(vec![
            output(true, WHO),
            output(true, stdout),
        ]));
        let response = api_router(AppState::new(runner))
            .oneshot(json_request(
                "POST",
                "/api/letters",
                json!({"agent": "%38", "skill": null, "body": "secret"}),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), 502, "accepted stdout {stdout:?}");
    }

    let runner = Arc::new(FixtureRunner::new(vec![
        output(true, WHO),
        CommandOutput {
            success: false,
            stdout: String::new(),
            stderr: "secret body leaked".into(),
        },
    ]));
    let response = api_router(AppState::new(runner))
        .oneshot(json_request(
            "POST",
            "/api/letters",
            json!({"agent": "%38", "skill": null, "body": "secret body leaked"}),
        ))
        .await
        .unwrap();
    let body = response_json(response).await;
    assert_eq!(body["code"], "letter_delivery_failed");
    assert!(!body.to_string().contains("secret body leaked"));
}

#[tokio::test]
async fn has_no_send_keys_command_in_letter_delivery() {
    let runner = Arc::new(FixtureRunner::new(vec![
        output(true, WHO),
        output(true, "sent -> %38: #1\n"),
    ]));
    let app = api_router(AppState::new(runner.clone()));
    let response = app
        .oneshot(json_request(
            "POST",
            "/api/letters",
            json!({"agent": "%38", "skill": null, "body": "hello"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), 201);
    assert!(runner
        .calls
        .lock()
        .unwrap()
        .iter()
        .all(|call| !call.args.iter().any(|arg| arg == "send-keys")));
}
