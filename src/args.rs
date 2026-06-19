use std::path::PathBuf;

#[derive(clap::Parser)]
#[command(
    no_binary_name = true,
    disable_help_flag = true,
    disable_version_flag = true
)]
pub struct DMVOPArguments {
    // Devices (unix/linux device or WASAPI name)
    #[arg(
        long = "device",
        alias = "dev",
        allow_hyphen_values = true,
        require_equals = true
    )]
    device_name: String,

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
    format_pattern: Option<String>,

    #[arg(
        long = "format-file",
        short = 'S',
        alias = "fmt",
        require_equals = true
    )]
    format_file: Option<PathBuf>,

    // Output
    #[arg(long, short = 'O', require_equals = true, default_value = "stdout")]
    output: OutputMode,

    // MISC
    // Port (default: 5117)
    #[arg(long, short = 'p', default_value_t = 5117, require_equals = true)]
    port: u16,

    // Socket file (default: ./dmvop.sock in current directory)
    #[arg(
        long = "socket-file",
        alias = "socket",
        default_value = "./dmvop.sock",
        require_equals = true
    )]
    socket_file: PathBuf,

    // Subnet mask for UDP broadcast (default: only last octet, e.g., "255.255.255.0")
    #[arg(
        long = "subnet-mask",
        alias = "mask",
        default_value = "255.255.255.0",
        require_equals = true
    )]
    subnet_mask: String,
}

#[derive(Clone)]
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
