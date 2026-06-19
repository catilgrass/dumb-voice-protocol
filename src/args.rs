use std::path::PathBuf;

/// Whether verbose debug output is enabled.
/// Set by `DMVOPArguments::verbose`.
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

#[derive(clap::Parser)]
#[command(name = "dmvop", disable_help_flag = true, disable_version_flag = true)]
pub struct DMVOPArguments {
    // Show help
    #[arg(long = "help", short = 'h')]
    pub help: bool,
    // Verbose output (show debug messages)
    #[arg(long = "verbose", short = 'V')]
    pub verbose: bool,

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
        require_equals = true
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

    // Subnet mask for UDP broadcast (default: only last octet, e.g., "255.255.255.0")
    #[arg(
        long = "subnet-mask",
        alias = "mask",
        default_value = "255.255.255.0",
        require_equals = true
    )]
    pub subnet_mask: String,
}

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

/// Parse a format pattern string and produce a formatted output string
/// from the provided values.
///
/// Supported placeholders:
/// - `%{vol}` — volume 0–100
/// - `%{word}` — transcribed word/text
/// - `%{confid}` / `%{confidence}` — confidence score
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
