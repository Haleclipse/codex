use std::collections::HashMap;

use toml::Value as TomlValue;

use super::ConfigToml;

const ALLOWED_PLUGIN_NAMES: [&str; 1] = ["translation"];

pub(crate) fn validate_plugins(config: &ConfigToml) -> std::io::Result<()> {
    validate_plugins_in_scope("plugins", &config.plugins)?;
    for (profile_name, profile) in &config.profiles {
        validate_plugins_in_scope(
            &format!("profiles.{profile_name}.plugins"),
            &profile.plugins,
        )?;
    }
    Ok(())
}

fn validate_plugins_in_scope(
    scope: &str,
    plugins: &HashMap<String, TomlValue>,
) -> std::io::Result<()> {
    for plugin_name in plugins.keys() {
        if !ALLOWED_PLUGIN_NAMES.contains(&plugin_name.as_str()) {
            let allowed = ALLOWED_PLUGIN_NAMES.join(", ");
            let path = format!("[{scope}.{plugin_name}]");
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "unknown plugin name `{plugin_name}` found at `{path}`. Allowed plugins: {allowed}."
                ),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::config::ConfigOverrides;
    use crate::config::types::AgentReasoningTranslationConfig;
    use pretty_assertions::assert_eq;
    use std::time::Duration;
    use tempfile::TempDir;

    fn load_config_from_toml(toml: &str, overrides: ConfigOverrides) -> std::io::Result<Config> {
        let cfg: ConfigToml = toml::from_str(toml).expect("TOML deserialization should succeed");
        load_config(cfg, overrides)
    }

    fn load_config(cfg: ConfigToml, overrides: ConfigOverrides) -> std::io::Result<Config> {
        let cwd_temp_dir = TempDir::new()?;
        let cwd = cwd_temp_dir.path().to_path_buf();
        std::fs::write(cwd.join(".git"), "gitdir: fake\n")?;

        let codex_home = TempDir::new()?;
        Config::load_from_base_config_with_overrides(
            cfg,
            ConfigOverrides {
                cwd: Some(cwd),
                ..overrides
            },
            codex_home.path().to_path_buf(),
        )
    }

    #[test]
    fn plugins_rejects_unknown_plugin_name_in_global_scope() -> std::io::Result<()> {
        let toml = r#"
[plugins.unknown_plugin]
enabled = true
"#;

        let err = load_config_from_toml(toml, ConfigOverrides::default())
            .expect_err("unknown plugin should be rejected");

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("unknown_plugin"));
        assert!(err.to_string().contains("translation"));
        Ok(())
    }

    #[test]
    fn plugins_rejects_unknown_plugin_name_in_profile_scope() -> std::io::Result<()> {
        let toml = r#"
[profiles.dev.plugins.unknown_plugin]
enabled = true
"#;

        let err = load_config_from_toml(toml, ConfigOverrides::default())
            .expect_err("unknown plugin should be rejected");

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("profiles.dev"));
        assert!(err.to_string().contains("unknown_plugin"));
        assert!(err.to_string().contains("translation"));
        Ok(())
    }

    #[test]
    fn plugins_allows_translation_plugin_name() -> std::io::Result<()> {
        let toml = r#"
[plugins.translation]
enabled = true
"#;

        let _config = load_config_from_toml(toml, ConfigOverrides::default())?;
        Ok(())
    }

    #[test]
    fn agent_reasoning_translation_disabled_by_default() -> std::io::Result<()> {
        let config = load_config(ConfigToml::default(), ConfigOverrides::default())?;

        assert_eq!(config.agent_reasoning_translation, None);
        Ok(())
    }

    #[test]
    fn agent_reasoning_translation_loads_from_toml() -> std::io::Result<()> {
        let toml = r#"
[translation.agent_reasoning]
command = ["python3", "/tmp/translate.py"]
timeout_ms = 1234
ui_max_wait_ms = 5678
"#;

        let config = load_config_from_toml(toml, ConfigOverrides::default())?;

        assert_eq!(
            config.agent_reasoning_translation,
            Some(AgentReasoningTranslationConfig {
                command: vec!["python3".to_string(), "/tmp/translate.py".to_string()],
                timeout: Duration::from_millis(1234),
                ui_max_wait: Duration::from_millis(5678),
            })
        );
        Ok(())
    }

    #[test]
    fn agent_reasoning_translation_loads_from_plugins_toml() -> std::io::Result<()> {
        let toml = r#"
[plugins.translation.agent_reasoning]
command = ["python3", "/tmp/translate.py"]
timeout_ms = 1234
ui_max_wait_ms = 5678
"#;

        let config = load_config_from_toml(toml, ConfigOverrides::default())?;

        assert_eq!(
            config.agent_reasoning_translation,
            Some(AgentReasoningTranslationConfig {
                command: vec!["python3".to_string(), "/tmp/translate.py".to_string()],
                timeout: Duration::from_millis(1234),
                ui_max_wait: Duration::from_millis(5678),
            })
        );
        Ok(())
    }

    #[test]
    fn agent_reasoning_translation_rejects_new_and_old_in_global_scope() -> std::io::Result<()> {
        let toml = r#"
[plugins.translation.agent_reasoning]
command = ["python3", "/tmp/translate.py"]

[translation.agent_reasoning]
command = ["python3", "/tmp/translate.py"]
"#;

        let err = load_config_from_toml(toml, ConfigOverrides::default())
            .expect_err("new + old in same scope should be rejected");

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(
            err.to_string()
                .contains("[plugins.translation.agent_reasoning]")
        );
        assert!(err.to_string().contains("[translation.agent_reasoning]"));
        Ok(())
    }

    #[test]
    fn agent_reasoning_translation_rejects_new_and_old_in_profile_scope() -> std::io::Result<()> {
        let toml = r#"
[profiles.dev.plugins.translation.agent_reasoning]
command = ["python3", "/tmp/translate.py"]

[profiles.dev.translation.agent_reasoning]
command = ["python3", "/tmp/translate.py"]
"#;

        let err = load_config_from_toml(
            toml,
            ConfigOverrides {
                config_profile: Some("dev".to_string()),
                ..Default::default()
            },
        )
        .expect_err("new + old in same scope should be rejected");

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(
            err.to_string()
                .contains("[profiles.dev.plugins.translation.agent_reasoning]")
        );
        assert!(
            err.to_string()
                .contains("[profiles.dev.translation.agent_reasoning]")
        );
        Ok(())
    }

    #[test]
    fn agent_reasoning_translation_rejects_unknown_fields_in_new_path() -> std::io::Result<()> {
        let toml = r#"
[plugins.translation.agent_reasoning]
command = ["python3", "/tmp/translate.py"]
unknown_field = 1
"#;

        let err = load_config_from_toml(toml, ConfigOverrides::default())
            .expect_err("unknown fields should be rejected");

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(
            err.to_string()
                .contains("[plugins.translation.agent_reasoning]")
        );
        assert!(err.to_string().contains("unknown_field"));
        Ok(())
    }

    #[test]
    fn agent_reasoning_translation_profile_overrides_global() -> std::io::Result<()> {
        let toml = r#"
[translation.agent_reasoning]
command = ["python3", "/tmp/translate.py"]
timeout_ms = 1234

[profiles.no_translate.translation.agent_reasoning]
command = []
"#;

        let config = load_config_from_toml(
            toml,
            ConfigOverrides {
                config_profile: Some("no_translate".to_string()),
                ..Default::default()
            },
        )?;

        assert_eq!(config.agent_reasoning_translation, None);
        Ok(())
    }

    #[test]
    fn agent_reasoning_translation_profile_overrides_global_in_plugins_toml() -> std::io::Result<()>
    {
        let toml = r#"
[plugins.translation.agent_reasoning]
command = ["python3", "/tmp/translate.py"]
timeout_ms = 1234

[profiles.no_translate.plugins.translation.agent_reasoning]
command = []
"#;

        let config = load_config_from_toml(
            toml,
            ConfigOverrides {
                config_profile: Some("no_translate".to_string()),
                ..Default::default()
            },
        )?;

        assert_eq!(config.agent_reasoning_translation, None);
        Ok(())
    }

    #[test]
    fn agent_reasoning_translation_allows_global_legacy_with_profile_new() -> std::io::Result<()> {
        let toml = r#"
[translation.agent_reasoning]
command = ["python3", "/tmp/translate.py"]
timeout_ms = 1234
ui_max_wait_ms = 5678

[profiles.dev.plugins.translation.agent_reasoning]
command = ["python3", "/tmp/translate-dev.py"]
timeout_ms = 2345
ui_max_wait_ms = 3456
"#;

        let config = load_config_from_toml(
            toml,
            ConfigOverrides {
                config_profile: Some("dev".to_string()),
                ..Default::default()
            },
        )?;

        assert_eq!(
            config.agent_reasoning_translation,
            Some(AgentReasoningTranslationConfig {
                command: vec!["python3".to_string(), "/tmp/translate-dev.py".to_string()],
                timeout: Duration::from_millis(2345),
                ui_max_wait: Duration::from_millis(3456),
            })
        );
        Ok(())
    }
}
