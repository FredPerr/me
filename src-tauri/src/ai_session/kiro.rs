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
        vec!["chat".to_string(), "--no-interactive".to_string()]
    }

    fn prompt_delivery(&self) -> PromptDelivery {
        PromptDelivery::Argument
    }

    fn agent_arguments(&self, agent: &str) -> Vec<String> {
        vec!["--agent".to_string(), agent.to_string()]
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
            shell: None,
            environment: Vec::new(),
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
    fn headless_arguments_use_non_interactive_chat() {
        let adapter = KiroAdapter;

        assert_eq!(
            adapter.headless_arguments(),
            vec!["chat", "--no-interactive"]
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

        // Then the prompt is the last argument, after the headless + agent flags.
        assert_eq!(
            spec.arguments,
            vec![
                "chat",
                "--no-interactive",
                "--agent",
                "reviewer",
                "do the thing"
            ]
        );
        assert!(spec.stdin_payload.is_none());
    }
}
