#[derive(Debug, Clone, PartialEq)]
pub enum PowerSaveMode {
    None,
    Light,
    High,
}

impl PowerSaveMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Light => "light",
            Self::High => "high",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "none" => Self::None,
            "light" => Self::Light,
            "high" => Self::High,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FallbackAp {
    pub ssid: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WifiConfig {
    pub ssid: String,
    pub password: String,
    pub fast_connect: bool,
    pub power_save_mode: PowerSaveMode,
    pub output_power: Option<f32>,
    pub enable_on_boot: bool,
    pub force_11bg: bool,
    pub fallback_ap: Option<FallbackAp>,
}

impl WifiConfig {
    pub fn new(ssid: &str, password: &str) -> Self {
        Self {
            ssid: ssid.to_string(),
            password: password.to_string(),
            fast_connect: false,
            power_save_mode: PowerSaveMode::None,
            output_power: None,
            enable_on_boot: true,
            force_11bg: false,
            fallback_ap: None,
        }
    }

    pub fn with_fast_connect(mut self) -> Self {
        self.fast_connect = true;
        self
    }

    pub fn with_power_save(mut self, mode: PowerSaveMode) -> Self {
        self.power_save_mode = mode;
        self
    }

    pub fn with_output_power(mut self, dbm: f32) -> Self {
        self.output_power = Some(dbm);
        self
    }

    pub fn with_force_11bg(mut self) -> Self {
        self.force_11bg = true;
        self
    }

    pub fn with_fallback_ap(mut self, ssid: &str, password: &str) -> Self {
        self.fallback_ap = Some(FallbackAp {
            ssid: ssid.to_string(),
            password: password.to_string(),
        });
        self
    }

    pub fn stable_preset(ssid: &str, password: &str) -> Self {
        Self::new(ssid, password)
            .with_fast_connect()
            .with_power_save(PowerSaveMode::None)
            .with_output_power(8.5)
            .with_force_11bg()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiState {
    Disconnected,
    Connecting,
    Connected,
    GotIp,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WifiStatus {
    pub state: WifiState,
    pub ip: Option<String>,
    pub ssid: Option<String>,
    pub rssi: Option<i32>,
    pub mac: Option<String>,
}

impl WifiStatus {
    pub fn disconnected() -> Self {
        Self {
            state: WifiState::Disconnected,
            ip: None,
            ssid: None,
            rssi: None,
            mac: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wifi_config_basic() {
        let config = WifiConfig::new("MySSID", "MyPass");
        assert_eq!(config.ssid, "MySSID");
        assert_eq!(config.password, "MyPass");
        assert!(!config.fast_connect);
        assert_eq!(config.power_save_mode, PowerSaveMode::None);
        assert!(config.output_power.is_none());
        assert!(config.enable_on_boot);
        assert!(!config.force_11bg);
        assert!(config.fallback_ap.is_none());
    }

    #[test]
    fn test_wifi_config_stable_preset() {
        let config = WifiConfig::stable_preset("ClarkUltra", "deadbeef00");
        assert_eq!(config.ssid, "ClarkUltra");
        assert!(config.fast_connect);
        assert_eq!(config.power_save_mode, PowerSaveMode::None);
        assert_eq!(config.output_power, Some(8.5));
        assert!(config.force_11bg);
    }

    #[test]
    fn test_wifi_config_with_fallback() {
        let config = WifiConfig::new("Main", "pass")
            .with_fallback_ap("Fallback", "fallback_pass");
        let ap = config.fallback_ap.unwrap();
        assert_eq!(ap.ssid, "Fallback");
        assert_eq!(ap.password, "fallback_pass");
    }

    #[test]
    fn test_wifi_config_builder_chain() {
        let config = WifiConfig::new("test", "pass")
            .with_fast_connect()
            .with_power_save(PowerSaveMode::Light)
            .with_output_power(10.0)
            .with_force_11bg();

        assert!(config.fast_connect);
        assert_eq!(config.power_save_mode, PowerSaveMode::Light);
        assert_eq!(config.output_power, Some(10.0));
        assert!(config.force_11bg);
    }

    #[test]
    fn test_power_save_mode_roundtrip() {
        let modes = [PowerSaveMode::None, PowerSaveMode::Light, PowerSaveMode::High];
        for mode in &modes {
            assert_eq!(PowerSaveMode::from_str(mode.as_str()), *mode);
        }
    }

    #[test]
    fn test_power_save_mode_unknown() {
        assert_eq!(PowerSaveMode::from_str("unknown"), PowerSaveMode::None);
    }

    #[test]
    fn test_wifi_status_disconnected() {
        let status = WifiStatus::disconnected();
        assert_eq!(status.state, WifiState::Disconnected);
        assert!(status.ip.is_none());
        assert!(status.mac.is_none());
    }

    #[test]
    fn test_wifi_state_variants() {
        let states = [
            WifiState::Disconnected,
            WifiState::Connecting,
            WifiState::Connected,
            WifiState::GotIp,
            WifiState::Failed,
        ];
        for (i, state) in states.iter().enumerate() {
            for (j, other) in states.iter().enumerate() {
                if i == j {
                    assert_eq!(state, other);
                } else {
                    assert_ne!(state, other);
                }
            }
        }
    }
}
