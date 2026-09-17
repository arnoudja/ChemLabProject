use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: String,
    pub database_url: String,
    pub static_dir: Option<PathBuf>,
    pub vite_dev_proxy: Option<String>,
    pub cookie_secure: bool,
    pub signup_enabled: bool,
    pub session_ttl_hours: i64,
}

fn parse_bool_flag(value: Option<&str>, default: bool) -> bool {
    value
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(default)
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let bind_addr = env::var("CHEMLAB_BIND").unwrap_or_else(|_| "127.0.0.1:3847".into());
        let database_url =
            env::var("CHEMLAB_DATABASE_URL").unwrap_or_else(|_| "sqlite://chemlab.db".into());
        let static_dir = env::var("CHEMLAB_STATIC_DIR").ok().map(PathBuf::from);
        let vite_dev_proxy = env::var("CHEMLAB_VITE_PROXY").ok();
        let cookie_secure =
            parse_bool_flag(env::var("CHEMLAB_COOKIE_SECURE").ok().as_deref(), false);
        let signup_enabled =
            parse_bool_flag(env::var("CHEMLAB_SIGNUP_ENABLED").ok().as_deref(), true);
        let session_ttl_hours = env::var("CHEMLAB_SESSION_TTL_HOURS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(24 * 14);

        Ok(Self {
            bind_addr,
            database_url,
            static_dir,
            vite_dev_proxy,
            cookie_secure,
            signup_enabled,
            session_ttl_hours,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::parse_bool_flag;

    #[test]
    fn signup_flag_unset_defaults_enabled() {
        assert!(parse_bool_flag(None, true));
    }

    #[test]
    fn cookie_secure_flag_unset_defaults_false() {
        assert!(!parse_bool_flag(None, false));
    }

    #[test]
    fn bool_flag_true_and_one_are_enabled() {
        assert!(parse_bool_flag(Some("true"), false));
        assert!(parse_bool_flag(Some("TRUE"), false));
        assert!(parse_bool_flag(Some("1"), false));
    }

    #[test]
    fn bool_flag_other_values_are_disabled() {
        assert!(!parse_bool_flag(Some("false"), true));
        assert!(!parse_bool_flag(Some("0"), true));
        assert!(!parse_bool_flag(Some(""), true));
        assert!(!parse_bool_flag(Some("yes"), true));
    }
}
