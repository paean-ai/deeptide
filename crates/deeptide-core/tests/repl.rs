use deeptide_core::{
    AgentBackend, AgentRequest, AgentResponse, AgentUsage, ReplEvent, ReplSession,
};

#[test]
fn repl_routes_plain_input_to_agent_loop() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    let events = repl.submit("hello");

    assert_eq!(
        events,
        vec![ReplEvent::Output(String::from("assistant reply"))]
    );
    assert_eq!(repl.agent_loop().messages().len(), 2);
}

#[test]
fn repl_executes_help_command() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    let events = repl.submit("/help");

    let output = only_output(events);
    assert!(output.contains("Deeptide commands"));
    assert!(output.contains("/exit"));
    assert!(output.contains("/cost"));
    assert!(output.contains("/read"));
}

#[test]
fn repl_exit_command_requests_exit() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    assert_eq!(repl.submit("/exit"), vec![ReplEvent::Exit]);
}

#[test]
fn repl_cost_command_uses_agent_loop_usage() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let _ = repl.submit("hello");

    let output = only_output(repl.submit("/cost"));

    assert!(output.contains("Cost breakdown"));
    assert!(output.contains("Total:"));
    assert!(output.contains("(4 in, 2 out)"));
}

#[test]
fn repl_clear_resets_agent_loop_state() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let _ = repl.submit("hello");

    let output = only_output(repl.submit("/clear"));

    assert!(output.contains("Conversation cleared."));
    assert!(repl.agent_loop().messages().is_empty());
}

#[test]
fn repl_read_command_reads_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("notes.txt"), "alpha\nbeta\ngamma\n").expect("write fixture");

    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_cwd(temp.path());
    let output = only_output(repl.submit("/read notes.txt --offset 2 --limit 1"));

    assert_eq!(output, "2\tbeta");
}

fn only_output(events: Vec<ReplEvent>) -> String {
    match events.as_slice() {
        [ReplEvent::Output(output)] => output.clone(),
        other => panic!("expected one output event, got {other:?}"),
    }
}

struct StaticBackend;

impl AgentBackend for StaticBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        Ok(AgentResponse {
            content: String::from("assistant reply"),
            usage: Some(AgentUsage::new(4, 2, 0, 0, 10)),
        })
    }
}
