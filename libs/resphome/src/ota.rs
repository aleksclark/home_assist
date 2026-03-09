#[derive(Debug, Clone, PartialEq)]
pub struct OtaConfig {
    pub password: Option<String>,
    pub port: u16,
    pub safe_mode: bool,
}

impl Default for OtaConfig {
    fn default() -> Self {
        Self {
            password: None,
            port: 3232,
            safe_mode: true,
        }
    }
}

impl OtaConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_password(mut self, password: &str) -> Self {
        self.password = Some(password.to_string());
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn with_safe_mode(mut self, enabled: bool) -> Self {
        self.safe_mode = enabled;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtaState {
    Idle,
    InProgress,
    Success,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ota_config_default() {
        let config = OtaConfig::new();
        assert!(config.password.is_none());
        assert_eq!(config.port, 3232);
        assert!(config.safe_mode);
    }

    #[test]
    fn test_ota_config_builder() {
        let config = OtaConfig::new()
            .with_password("secret123")
            .with_port(8266)
            .with_safe_mode(false);

        assert_eq!(config.password.as_deref(), Some("secret123"));
        assert_eq!(config.port, 8266);
        assert!(!config.safe_mode);
    }

    #[test]
    fn test_ota_state_variants() {
        let states = [OtaState::Idle, OtaState::InProgress, OtaState::Success, OtaState::Error];
        for (i, s) in states.iter().enumerate() {
            for (j, o) in states.iter().enumerate() {
                assert_eq!(i == j, s == o);
            }
        }
    }
}
