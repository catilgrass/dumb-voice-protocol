use std::path::PathBuf;

/// Whether verbose debug output is enabled.
pub static VERBOSE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Print a debug message only when `--verbose` is set.
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        if $crate::args::VERBOSE.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!($($arg)*);
        }
    };
}

/// The full help text embedded at compile time.
pub static HELP_TEXT: &str = include_str!("../help.txt");

// ===========================================================================
// CLI arguments (clap)
// ===========================================================================

#[derive(clap::Parser)]
#[command(name = "dmvop", disable_help_flag = true, disable_version_flag = true)]
pub struct DMVOPArguments {
    // Show help
    #[arg(long = "help", short = 'h')]
    pub help: bool,

    // Instant mode — aggressive VAD for near-real-time output
    #[arg(long = "instant")]
    pub instant: bool,

    // Verbose output (show debug messages)
    #[arg(long = "verbose", short = 'V')]
    pub verbose: bool,

    // Config file path
    #[arg(long = "config", require_equals = true)]
    pub config: Option<PathBuf>,

    // List all available input devices and exit
    #[arg(long = "list-devices", alias = "list", short = 'L')]
    pub list_devices: bool,

    // List all available Whisper models and exit
    #[arg(long = "list-models")]
    pub list_models: bool,

    // Download a specific Whisper model and exit
    #[arg(long = "download-model", alias = "get-model", require_equals = true)]
    pub download_model: Option<String>,

    // Language hint for Whisper (e.g. en, zh, ja). Auto-detects if not set.
    #[arg(long = "lang", require_equals = true)]
    pub lang: Option<String>,

    // Whisper model to use (e.g. tiny, base, small, medium, large-v3)
    #[arg(
        long = "model",
        short = 'm',
        default_value = "base_en",
        require_equals = true
    )]
    pub model: String,

    // Devices (unix/linux device or WASAPI name)
    #[arg(
        long = "device",
        alias = "dev",
        allow_hyphen_values = true,
        require_equals = true,
        default_value = "auto"
    )]
    pub device_name: Option<String>,

    // Format
    // Use %{param} to represent a parameter
    //
    // Supported parameters:
    // vol   : volume 0-100
    // word  : word
    // confid: confidence
    #[arg(
        long = "format",
        short,
        alias = "fmt",
        default_value = "%{vol},%{word}",
        require_equals = true
    )]
    pub format_pattern: Option<String>,

    #[arg(long = "format-file", short = 'S', require_equals = true)]
    pub format_file: Option<PathBuf>,

    // Output (can be specified multiple times)
    #[arg(
        long,
        short = 'O',
        require_equals = true,
        default_value = "stdout",
        num_args = 1
    )]
    pub output: Vec<OutputMode>,

    // MISC
    // Port (default: 5117)
    #[arg(long, short = 'p', default_value_t = 5117, require_equals = true)]
    pub port: u16,

    // Socket file (default: ./dmvop.sock in current directory)
    #[arg(
        long = "socket-file",
        alias = "socket",
        default_value = "./dmvop.sock",
        require_equals = true
    )]
    pub socket_file: PathBuf,

    // Custom directory for model files
    #[arg(long = "models-dir", alias = "models", require_equals = true)]
    pub models_dir: Option<PathBuf>,

    // Subnet mask for UDP broadcast (default: only last octet, e.g., "255.255.255.255.0")
    #[arg(
        long = "subnet-mask",
        alias = "mask",
        default_value = "255.255.255.0",
        require_equals = true
    )]
    pub subnet_mask: String,

    // Post-processing pipeline, e.g. --post="+pinyin()" or --post="+reverse"
    #[arg(long = "post", require_equals = true)]
    pub post: Option<String>,
}

// ===========================================================================
// Configuration file (serde) — mirrors DMVOPArguments with all-Option fields
// ===========================================================================

/// Mirrors [`DMVOPArguments`] as a TOML-serialisable config.
/// Every field is `Option` so the merge logic can tell what was explicitly set.
#[derive(serde::Deserialize, Default)]
#[serde(default)]
pub struct DMVOPConfig {
    pub instant: Option<bool>,
    pub lang: Option<String>,
    pub model: Option<String>,
    pub device: Option<String>,
    pub format: Option<String>,
    pub format_file: Option<PathBuf>,
    pub output: Option<Vec<String>>,
    pub port: Option<u16>,
    pub socket_file: Option<PathBuf>,
    pub models_dir: Option<PathBuf>,
    pub subnet_mask: Option<String>,
    pub post: Option<String>,
}

/// Resolve a path in the config file relative to the config file's directory.
fn resolve_config_path(config_file: &PathBuf, path: &PathBuf) -> PathBuf {
    if path.is_absolute() {
        path.clone()
    } else if let Some(parent) = config_file.parent() {
        parent.join(path)
    } else {
        path.clone()
    }
}

/// Load config file, then merge CLI args on top.
/// Config paths are resolved relative to the config file's directory.
pub fn load_and_merge_config(config: Option<&PathBuf>) -> Option<DMVOPConfig> {
    let config_path = config.cloned().or_else(|| {
        let fallback = PathBuf::from("./dmvop.toml");
        if fallback.exists() {
            Some(fallback)
        } else {
            None
        }
    });

    let config_path = config_path?;
    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "[dmvop] Failed to read config '{}': {}",
                config_path.display(),
                e
            );
            std::process::exit(1);
        }
    };

    let mut cfg: DMVOPConfig = match toml::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "[dmvop] Failed to parse config '{}': {}",
                config_path.display(),
                e
            );
            std::process::exit(1);
        }
    };

    // Resolve relative paths
    if let Some(ref mut f) = cfg.format_file {
        *f = resolve_config_path(&config_path, f);
    }
    if let Some(ref mut d) = cfg.models_dir {
        *d = resolve_config_path(&config_path, d);
    }
    if let Some(ref mut s) = cfg.socket_file {
        *s = resolve_config_path(&config_path, s);
    }

    Some(cfg)
}

/// Merge config file values into CLI args (CLI wins).
pub fn apply_config(args: &mut DMVOPArguments, cfg: &DMVOPConfig) {
    if let Some(v) = cfg.instant {
        if !args.instant {
            args.instant = v;
        }
    }
    if let Some(ref v) = cfg.lang {
        if args.lang.is_none() {
            args.lang = Some(v.clone());
        }
    }
    if let Some(ref v) = cfg.model {
        args.model = v.clone();
    }
    if let Some(ref v) = cfg.device {
        if args.device_name.is_none() {
            args.device_name = Some(v.clone());
        }
    }
    if let Some(ref v) = cfg.format {
        if args.format_pattern.is_none() {
            args.format_pattern = Some(v.clone());
        }
    }
    if let Some(ref v) = cfg.format_file {
        if args.format_file.is_none() {
            args.format_file = Some(v.clone());
        }
    }
    if let Some(ref v) = cfg.output {
        if args.output.is_empty() {
            args.output = v.iter().filter_map(|s| s.parse().ok()).collect();
        }
    }
    if let Some(v) = cfg.port {
        args.port = v;
    }
    if let Some(ref v) = cfg.socket_file {
        args.socket_file = v.clone();
    }
    if let Some(ref v) = cfg.models_dir {
        if args.models_dir.is_none() {
            args.models_dir = Some(v.clone());
        }
    }
    if let Some(ref v) = cfg.subnet_mask {
        args.subnet_mask = v.clone();
    }
    if let Some(ref v) = cfg.post {
        if args.post.is_none() {
            args.post = Some(v.clone());
        }
    }
}

// ===========================================================================
// Output mode enum
// ===========================================================================

#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
pub enum OutputMode {
    /// Establish a basic TCP service, broadcasting output to all connected sockets
    TCP,
    /// Establish a basic UDP service, broadcasting output to all connected sockets
    UDP,
    /// Broadcast output via UDP broadcast
    UDP_BROADCAST,
    /// Send signal to specified IPC socket
    IPC,
    /// Write directly to stdout
    STDOUT,
    /// Write directly to stderr
    STDERR,
}

impl std::str::FromStr for OutputMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tcp" => Ok(OutputMode::TCP),
            "udp" => Ok(OutputMode::UDP),
            "udp-broadcast" => Ok(OutputMode::UDP_BROADCAST),
            "ipc" => Ok(OutputMode::IPC),
            "stdout" => Ok(OutputMode::STDOUT),
            "stderr" => Ok(OutputMode::STDERR),
            _ => Err(format!(
                "'{}' is not a valid output mode. Valid modes: tcp, udp, udp-broadcast, ipc, stdout, stderr",
                s
            )),
        }
    }
}

// ===========================================================================
// Format helpers
// ===========================================================================

pub fn format_output(pattern: &str, word: &str, confidence: f32, volume: f32) -> String {
    let mut result = pattern.to_string();

    // Volume: clamp dB range (~-60 to 0) to 0–100 scale
    // Typical speech is around -30 dB to -12 dB
    let vol_normalized = ((volume + 60.0).clamp(0.0, 60.0) / 60.0 * 100.0) as u32;

    // Replace known placeholders
    result = result.replace("%{vol}", &vol_normalized.to_string());
    result = result.replace("%{word}", word);
    result = result.replace("%{confid}", &format!("{:.1}", confidence));
    result = result.replace("%{confidence}", &format!("{:.1}", confidence));

    result
}

/// Parse a post-processor spec in the form `+function_name(arg1, arg2, ...)`.
/// Returns `(name, args)`.
pub fn parse_post_spec(spec: &str) -> (&str, Vec<&str>) {
    let spec = spec.trim();
    if !spec.starts_with('+') {
        return (spec, vec![]);
    }
    let inner = &spec[1..];
    if let Some(paren) = inner.find('(') {
        if inner.ends_with(')') {
            let name = inner[..paren].trim();
            let args_str = inner[paren + 1..inner.len() - 1].trim();
            let args: Vec<&str> = if args_str.is_empty() {
                vec![]
            } else {
                // Simple comma split (no nested parens, no escape handling for now)
                args_str.split(',').map(|a| a.trim()).collect()
            };
            return (name, args);
        }
    }
    // No parens → no args
    (inner.trim(), vec![])
}

/// Run the post-processing pipeline on the given text.
pub fn run_post_process(text: &str, spec: Option<&str>) -> String {
    let Some(spec) = spec else {
        return text.to_string();
    };
    let (name, args) = parse_post_spec(spec);
    match crate::post_proc::run(text, name, &args) {
        Some(result) => result,
        None => {
            eprintln!("[dmvop] Unknown post-processor: '{}'", name);
            text.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_output() {
        let s = format_output("%{vol},%{word},%{confid}", "hello", 95.0, -12.0);
        assert_eq!(s, "80,hello,95.0");
    }

    #[test]
    fn test_format_output_no_volume() {
        let s = format_output("text: %{word}", "world", 0.0, -60.0);
        assert_eq!(s, "text: world");
    }
}
