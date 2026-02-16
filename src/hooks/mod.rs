pub mod handler;

use serde_json::json;

/// Generate Claude Code hooks configuration that posts to Conductor
pub fn generate_hooks_config(port: u16) -> serde_json::Value {
    let base_url = format!("http://localhost:{}", port);

    json!({
        "hooks": {
            "Stop": [{
                "hooks": [{
                    "type": "command",
                    "command": format!(
                        "curl -s -X POST {}/api/hooks/stop -H 'Content-Type: application/json' -d \"$(cat)\"",
                        base_url
                    )
                }]
            }],
            "SubagentStop": [{
                "hooks": [{
                    "type": "command",
                    "command": format!(
                        "curl -s -X POST {}/api/hooks/subagent-stop -H 'Content-Type: application/json' -d \"$(cat)\"",
                        base_url
                    )
                }]
            }]
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_hooks_config_structure() {
        let config = generate_hooks_config(3001);
        let hooks = config.get("hooks").expect("should have hooks key");
        assert!(hooks.get("Stop").is_some(), "should have Stop hook");
        assert!(
            hooks.get("SubagentStop").is_some(),
            "should have SubagentStop hook"
        );
    }

    #[test]
    fn test_generate_hooks_config_uses_port() {
        let config = generate_hooks_config(9999);
        let config_str = serde_json::to_string(&config).unwrap();
        assert!(config_str.contains("localhost:9999"));
        assert!(!config_str.contains("localhost:3001"));
    }

    #[test]
    fn test_generate_hooks_config_stop_hook_format() {
        let config = generate_hooks_config(3001);
        let stop = &config["hooks"]["Stop"][0]["hooks"][0];
        assert_eq!(stop["type"], "command");
        let cmd = stop["command"].as_str().unwrap();
        assert!(cmd.contains("/api/hooks/stop"));
        assert!(cmd.contains("curl"));
        assert!(cmd.contains("POST"));
    }

    #[test]
    fn test_generate_hooks_config_subagent_hook_format() {
        let config = generate_hooks_config(3001);
        let hook = &config["hooks"]["SubagentStop"][0]["hooks"][0];
        assert_eq!(hook["type"], "command");
        let cmd = hook["command"].as_str().unwrap();
        assert!(cmd.contains("/api/hooks/subagent-stop"));
    }

    #[test]
    fn test_different_ports() {
        for port in [3000, 3001, 8080, 4567] {
            let config = generate_hooks_config(port);
            let config_str = serde_json::to_string(&config).unwrap();
            assert!(config_str.contains(&format!("localhost:{}", port)));
        }
    }
}

/// Write hooks configuration to a worktree's .claude/settings.json
pub async fn install_hooks(worktree_path: &std::path::Path, port: u16) -> anyhow::Result<()> {
    let claude_dir = worktree_path.join(".claude");
    tokio::fs::create_dir_all(&claude_dir).await?;

    let config = generate_hooks_config(port);
    let settings_path = claude_dir.join("settings.json");

    tokio::fs::write(&settings_path, serde_json::to_string_pretty(&config)?).await?;

    tracing::info!(
        "Installed hooks config at {}",
        settings_path.display()
    );

    Ok(())
}
