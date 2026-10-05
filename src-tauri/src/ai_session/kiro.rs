//! Adapter for the Kiro CLI.
//!
//! Kiro runs headlessly via `kiro-cli chat --no-interactive "<prompt>"`, with
//! the prompt passed as a positional argument (it may also be supplied over
//! stdin; we use the argument form here). Authentication for headless runs is
//! done with an API key provided through the environment by the caller.
//!
//! See the Kiro headless docs: https://kiro.dev/docs/cli/headless/

use super::adapter::{CliAdapter, PromptDelivery};

pub struct KiroAdapter;

impl CliAdapter for KiroAdapter {
    fn id(&self) -> &'static str {
        "kiro"
    }

    fn display_name(&self) -> &'static str {
        "Kiro CLI"
    }

    fn default_command(&self) -> &'static str {
        "kiro-cli"
    }

    fn headless_arguments(&self) -> Vec<String> {
        // `stream-json` emits ACP events as JSON Lines on stdout (and implies
        // non-interactive), which the frontend parses into rich chat parts.
        vec![
            "chat".to_string(),
            "--no-interactive".to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
        ]
    }

    fn prompt_delivery(&self) -> PromptDelivery {
        PromptDelivery::Argument
    }

    fn prompt_needs_options_terminator(&self) -> bool {
        true
    }

    fn agent_arguments(&self, agent: &str) -> Vec<String> {
        vec!["--agent".to_string(), agent.to_string()]
    }

    fn resume_arguments(&self, resume_id: &str) -> Vec<String> {
        vec!["--resume-id".to_string(), resume_id.to_string()]
    }

    fn trust_all_arguments(&self) -> Vec<String> {
        // `-a` / `--trust-all-tools`: run tools without confirmation. Safe
        // because a workspace permissions.yaml deny floor still blocks
        // dangerous operations (deny wins over trust).
        vec!["-a".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::super::adapter::{PromptDelivery, SpawnRequest};
    use super::*;

    fn request() -> SpawnRequest {
        SpawnRequest {
            prompt: "do the thing".to_string(),
            working_directory: "/tmp/project".to_string(),
            agent: None,
            resume_id: None,
            shell: None,
            environment: Vec::new(),
            trust_all_tools: false,
        }
    }

    #[test]
    fn exposes_stable_identity() {
        let adapter = KiroAdapter;

        assert_eq!(adapter.id(), "kiro");
        assert_eq!(adapter.default_command(), "kiro-cli");
        assert_eq!(adapter.prompt_delivery(), PromptDelivery::Argument);
    }

    #[test]
    fn headless_arguments_use_non_interactive_streaming_json() {
        let adapter = KiroAdapter;

        assert_eq!(
            adapter.headless_arguments(),
            vec!["chat", "--no-interactive", "--output-format", "stream-json"]
        );
    }

    #[test]
    fn trust_all_arguments_pass_the_trust_flag() {
        let adapter = KiroAdapter;

        assert_eq!(adapter.trust_all_arguments(), vec!["-a"]);
    }

    #[test]
    fn build_spec_appends_trust_flag_when_requested() {
        let adapter = KiroAdapter;
        let mut spawn_request = request();
        spawn_request.trust_all_tools = true;

        let spec = adapter
            .build_spec(&spawn_request, Some("/bin/echo"))
            .expect("spec should build for an existing binary");

        // Trust flag sits after the headless flags and before the prompt, with
        // the `--` options terminator immediately preceding the prompt.
        assert_eq!(
            spec.arguments,
            vec![
                "chat",
                "--no-interactive",
                "--output-format",
                "stream-json",
                "-a",
                "--",
                "do the thing"
            ]
        );
    }

    #[test]
    fn agent_arguments_pass_agent_flag() {
        let adapter = KiroAdapter;

        assert_eq!(
            adapter.agent_arguments("reviewer"),
            vec!["--agent", "reviewer"]
        );
    }

    #[test]
    fn resume_arguments_pass_the_resume_id() {
        let adapter = KiroAdapter;

        assert_eq!(
            adapter.resume_arguments("abc-123"),
            vec!["--resume-id", "abc-123"]
        );
    }

    #[test]
    fn build_spec_inserts_resume_id_before_the_prompt() {
        let adapter = KiroAdapter;
        let mut spawn_request = request();
        spawn_request.resume_id = Some("conv-9".to_string());

        let spec = adapter
            .build_spec(&spawn_request, Some("/bin/echo"))
            .expect("spec should build for an existing binary");

        assert_eq!(
            spec.arguments,
            vec![
                "chat",
                "--no-interactive",
                "--output-format",
                "stream-json",
                "--resume-id",
                "conv-9",
                "--",
                "do the thing"
            ]
        );
    }

    #[test]
    fn build_spec_omits_resume_id_when_empty() {
        let adapter = KiroAdapter;
        let mut spawn_request = request();
        spawn_request.resume_id = Some(String::new());

        let spec = adapter
            .build_spec(&spawn_request, Some("/bin/echo"))
            .expect("spec should build for an existing binary");

        assert!(!spec.arguments.iter().any(|arg| arg == "--resume-id"));
    }

    #[test]
    fn build_spec_appends_prompt_as_final_argument() {
        // Given an explicit binary path so resolution does not touch the system.
        let adapter = KiroAdapter;
        let mut spawn_request = request();
        spawn_request.agent = Some("reviewer".to_string());

        // When building a spec against an existing binary (use /bin/echo as a
        // stand-in that is guaranteed to exist on unix test runners).
        let spec = adapter
            .build_spec(&spawn_request, Some("/bin/echo"))
            .expect("spec should build for an existing binary");

        // Then the prompt is the last argument, after the headless + agent
        // flags and the `--` options terminator.
        assert_eq!(
            spec.arguments,
            vec![
                "chat",
                "--no-interactive",
                "--output-format",
                "stream-json",
                "--agent",
                "reviewer",
                "--",
                "do the thing"
            ]
        );
        assert!(spec.stdin_payload.is_none());
    }

    #[test]
    fn prompt_declares_options_terminator() {
        let adapter = KiroAdapter;

        assert!(adapter.prompt_needs_options_terminator());
    }

    #[test]
    fn build_spec_keeps_a_dash_prefixed_prompt_as_a_positional_value() {
        // A prompt that begins with "- " (a Markdown bullet list) must not be
        // parsed as a CLI flag; the `--` terminator guarantees it stays the
        // positional prompt.
        let adapter = KiroAdapter;
        let mut spawn_request = request();
        spawn_request.prompt = "- do the first thing\n- then the second".to_string();

        let spec = adapter
            .build_spec(&spawn_request, Some("/bin/echo"))
            .expect("spec should build for an existing binary");

        assert_eq!(
            spec.arguments,
            vec![
                "chat",
                "--no-interactive",
                "--output-format",
                "stream-json",
                "--",
                "- do the first thing\n- then the second"
            ]
        );
    }
}
