mod args;
pub use args::*;

mod output_protocol;
use clap::Parser;
use output_protocol::OutputProtocol;
use std::path::PathBuf;
use std::sync::Arc;
use vtx_engine::EngineBuilder;

enum OutputChannel {
    Stdout(Arc<output_protocol::StandardOutputProtocol>),
    Stderr(Arc<output_protocol::StandardErrorProtocol>),
    Tcp(Arc<output_protocol::TCPOutputProtocol>),
    Udp(Arc<output_protocol::UDPOutputProtocol>),
    UdpBroadcast(Arc<output_protocol::UDPBroadcastOutputProtocol>),
    #[cfg(unix)]
    Ipc(Arc<output_protocol::IPCOutputProtocol>),
}

impl OutputChannel {
    async fn init(&self) {
        match self {
            OutputChannel::Stdout(p) => p.init().await,
            OutputChannel::Stderr(p) => p.init().await,
            OutputChannel::Tcp(p) => p.init().await,
            OutputChannel::Udp(p) => p.init().await,
            OutputChannel::UdpBroadcast(p) => p.init().await,
            #[cfg(unix)]
            OutputChannel::Ipc(p) => p.init().await,
        }
    }

    async fn send_to_channel(&self, message: &str) {
        match self {
            OutputChannel::Stdout(p) => p.clone().send(message).await,
            OutputChannel::Stderr(p) => p.clone().send(message).await,
            OutputChannel::Tcp(p) => p.clone().send(message).await,
            OutputChannel::Udp(p) => p.clone().send(message).await,
            OutputChannel::UdpBroadcast(p) => p.clone().send(message).await,
            #[cfg(unix)]
            OutputChannel::Ipc(p) => p.clone().send(message).await,
        }
    }
}

#[tokio::main]
async fn main() {
    // Set up tracing so we can see vtx-engine logs (including transcription errors)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::WARN.into())
                .from_env_lossy(),
        )
        .with_target(false)
        .init();

    let args = match DMVOPArguments::try_parse() {
        Ok(a) => a,
        Err(_) => {
            eprintln!("error: invalid arguments. Use --help for usage.");
            std::process::exit(1);
        }
    };

    // Handle --help immediately
    if args.help {
        print!("{}", HELP_TEXT);
        return;
    }

    // Set global verbose flag
    VERBOSE.store(args.verbose, std::sync::atomic::Ordering::Relaxed);

    // When verbose, let whisper.cpp print its C lib logs to stderr
    if args.verbose {
        vtx_engine::transcription::whisper_ffi::WHISPER_LOGS_ENABLED
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    // ---------------------------------------------------------------
    // --list-models: show available models and exit
    // ---------------------------------------------------------------
    if args.list_models {
        list_models();
        return;
    }

    // ---------------------------------------------------------------
    // --download-model: download a specific model and exit
    // ---------------------------------------------------------------
    if let Some(ref model_name) = args.download_model {
        download_model_cli(model_name).await;
        return;
    }

    // ---------------------------------------------------------------
    // Build the vtx-engine (needed for both listing and capture)
    // ---------------------------------------------------------------
    debug_log!("[dmvop] Initializing voice engine...");

    let model = vtx_engine::WhisperModel::parse_identifier(&args.model).unwrap_or_else(|| {
        eprintln!(
            "[dmvop] Unknown model '{}'. Use --list-models to see available models.",
            args.model
        );
        std::process::exit(1);
    });

    debug_log!(
        "[dmvop] Using model: {} ({})",
        model.config_key(),
        model.display_name()
    );

    let mut builder = EngineBuilder::new().app_name("dmvop").model(model);

    if args.instant {
        debug_log!("[dmvop] Instant mode: aggressive VAD for near-real-time output");
        builder = builder
            .vad_voiced_onset_ms(40)
            .vad_whisper_onset_ms(60)
            .segment_max_duration_ms(800)
            .segment_word_break_grace_ms(100)
            .word_break_segmentation_enabled(true);
    }

    if let Some(ref lang) = args.lang {
        debug_log!("[dmvop] Language hint: {}", lang);
        builder = builder.language(lang.as_str());
    }

    let (engine, mut rx) = builder.build().await.expect("Failed to build vtx-engine");

    // Disable PTT mode so VAD drives automatic segmentation
    engine.set_ptt_mode(false);

    // ---------------------------------------------------------------
    // Check model availability and download if needed
    // ---------------------------------------------------------------
    let model_status = engine.check_model_status();
    if !model_status.available {
        eprintln!("[dmvop] Model not found at: {}", model_status.path);
        eprintln!("[dmvop] Downloading model, please wait...");
        match engine.download_model().await {
            Ok(_) => debug_log!("[dmvop] Model downloaded successfully"),
            Err(e) => {
                eprintln!("[dmvop] Failed to download model: {}", e);
                eprintln!("[dmvop] You can manually download a model from:");
                eprintln!("[dmvop]   https://huggingface.co/ggerganov/whisper.cpp/tree/main");
                eprintln!("[dmvop]   Place it at: {}", model_status.path);
                std::process::exit(1);
            }
        }
    } else {
        debug_log!("[dmvop] Model found: {}", model_status.path);
    }

    // ---------------------------------------------------------------
    // List devices and exit?
    // ---------------------------------------------------------------
    let devices = engine.list_input_devices();

    if args.list_devices {
        if devices.is_empty() {
            eprintln!("[dmvop] No input devices found.");
        } else {
            println!("Available input devices:");
            for (i, dev) in devices.iter().enumerate() {
                println!(
                    "  [{}] {} (id: {}, type: {:?})",
                    i, dev.name, dev.id, dev.source_type
                );
            }
        }
        return;
    }

    // ---------------------------------------------------------------
    // Resolve the format pattern
    // ---------------------------------------------------------------
    let pattern = resolve_format_pattern(args.format_pattern.as_deref(), args.format_file.as_ref());

    // ---------------------------------------------------------------
    // Create and initialize output channels
    // ---------------------------------------------------------------
    let mut channels: Vec<OutputChannel> = Vec::new();

    for mode in &args.output {
        match create_output_channel(mode, args.port, args.socket_file.clone(), &args.subnet_mask) {
            Some(ch) => {
                ch.init().await;
                channels.push(ch);
            }
            None => debug_log!(
                "[dmvop] Warning: failed to create output channel {:?}",
                mode
            ),
        }
    }

    if channels.is_empty() {
        eprintln!("[dmvop] No output channels available. Exiting.");
        std::process::exit(1);
    }

    // ---------------------------------------------------------------
    // Find the requested device and start capture
    // ---------------------------------------------------------------
    let device_name = match &args.device_name {
        Some(n) => n.as_str(),
        None => {
            eprintln!(
                "[dmvop] No device specified. Use --device=<name> or --list-devices to see available devices."
            );
            std::process::exit(1);
        }
    };

    let device = devices
        .iter()
        .find(|d| d.id == device_name || d.name == device_name)
        .or_else(|| devices.first());

    match &device {
        Some(d) => {
            debug_log!("[dmvop] Using input device: {} (id: {})", d.name, d.id);
        }
        None => {
            eprintln!(
                "[dmvop] Device '{}' not found and no fallback available.",
                device_name
            );
            std::process::exit(1);
        }
    }

    engine
        .start_capture(device.map(|d| d.id.clone()), None)
        .await
        .expect("Failed to start audio capture");

    debug_log!("[dmvop] Capture started. Waiting for speech...");

    // ---------------------------------------------------------------
    // Event loop — listen for transcription & audio level events
    // ---------------------------------------------------------------
    let mut last_volume_db: f32 = -60.0;

    loop {
        match rx.recv().await {
            Ok(event) => match event {
                vtx_engine::EngineEvent::TranscriptionComplete(result) => {
                    let raw = maybe_to_pinyin(&result.text, args.use_pinyin);
                    let formatted = format_output(&pattern, &raw, 0.0, last_volume_db);

                    for ch in &channels {
                        ch.send_to_channel(&formatted).await;
                    }
                }
                vtx_engine::EngineEvent::TranscriptionSegment(segment) => {
                    let raw = maybe_to_pinyin(&segment.text, args.use_pinyin);
                    let formatted = format_output(&pattern, &raw, 0.0, last_volume_db);

                    for ch in &channels {
                        ch.send_to_channel(&formatted).await;
                    }
                }
                vtx_engine::EngineEvent::VisualizationData(viz) => {
                    if let Some(ref metrics) = viz.speech_metrics {
                        last_volume_db = metrics.amplitude_db;
                    }
                }
                vtx_engine::EngineEvent::SpeechStarted => {
                    debug_log!("[dmvop] Speech started");
                }
                vtx_engine::EngineEvent::SpeechEnded { duration_ms } => {
                    debug_log!("[dmvop] Speech ended ({}ms)", duration_ms);
                }
                vtx_engine::EngineEvent::CaptureStateChanged { capturing, error } => {
                    if !capturing {
                        eprintln!(
                            "[dmvop] Capture stopped: {}",
                            error.unwrap_or_else(|| "unknown".to_string())
                        );
                        break;
                    }
                }
                vtx_engine::EngineEvent::ModelDownloadProgress { percent } => {
                    debug_log!("[dmvop] Downloading model: {}%", percent);
                }
                vtx_engine::EngineEvent::ModelDownloadComplete { success } => {
                    debug_log!(
                        "[dmvop] Model download {}",
                        if success { "complete" } else { "failed" }
                    );
                }
                _ => {}
            },
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                debug_log!("[dmvop] Warning: missed {} events", n);
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                eprintln!("[dmvop] Engine event stream closed");
                break;
            }
        }
    }

    debug_log!("[dmvop] Shutting down.");
}

/// Resolve the format pattern from either the command-line `--format` value
/// or the contents of `--format-file`.
fn resolve_format_pattern(pattern: Option<&str>, file: Option<&PathBuf>) -> String {
    if let Some(path) = file {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let trimmed = content.trim().to_string();
                if !trimmed.is_empty() {
                    return trimmed;
                }
                eprintln!(
                    "[dmvop] Warning: format file {} is empty, using default pattern",
                    path.display()
                );
            }
            Err(e) => {
                eprintln!(
                    "[dmvop] Warning: could not read format file {}: {}",
                    path.display(),
                    e
                );
            }
        }
    }

    pattern
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "%{vol},%{word}".to_string())
}

/// Create an output channel from an [`OutputMode`].
fn create_output_channel(
    mode: &OutputMode,
    port: u16,
    socket_file: PathBuf,
    subnet_mask: &str,
) -> Option<OutputChannel> {
    match mode {
        OutputMode::STDOUT => Some(OutputChannel::Stdout(Arc::new(
            output_protocol::StandardOutputProtocol,
        ))),
        OutputMode::STDERR => Some(OutputChannel::Stderr(Arc::new(
            output_protocol::StandardErrorProtocol,
        ))),
        OutputMode::TCP => Some(OutputChannel::Tcp(Arc::new(
            output_protocol::TCPOutputProtocol::new(port),
        ))),
        OutputMode::UDP => Some(OutputChannel::Udp(Arc::new(
            output_protocol::UDPOutputProtocol::new(port),
        ))),
        OutputMode::UDP_BROADCAST => Some(OutputChannel::UdpBroadcast(Arc::new(
            output_protocol::UDPBroadcastOutputProtocol::new(port, subnet_mask),
        ))),
        OutputMode::IPC => {
            #[cfg(unix)]
            {
                Some(OutputChannel::Ipc(Arc::new(
                    output_protocol::IPCOutputProtocol::new(socket_file),
                )))
            }
            #[cfg(not(unix))]
            {
                let _ = socket_file;
                eprintln!("[dmvop] IPC (Unix domain socket) is not supported on this platform");
                None
            }
        }
    }
}

/// Print all available Whisper models and their sizes.
fn list_models() {
    println!("Available Whisper models:");
    for model in vtx_engine::WhisperModel::all_in_size_order() {
        let size = model.size_mb();
        let size_str = if size >= 1024 {
            format!("{:.1} GB", size as f64 / 1024.0)
        } else {
            format!("{} MB", size)
        };
        println!(
            "  {:20}  {}  ({})",
            model.config_key(),
            size_str,
            model.display_name()
        );
    }
}

/// Download a specific Whisper model by identifier.
async fn download_model_cli(model_name: &str) {
    let model = match vtx_engine::WhisperModel::parse_identifier(model_name) {
        Some(m) => m,
        None => {
            eprintln!(
                "[dmvop] Unknown model '{}'. Use --list-models to see available models.",
                model_name
            );
            std::process::exit(1);
        }
    };

    eprintln!(
        "[dmvop] Building engine with model '{}'...",
        model.config_key()
    );

    let (engine, _rx) = match EngineBuilder::new()
        .app_name("dmvop")
        .model(model)
        .build()
        .await
    {
        Ok(e) => e,
        Err(e) => {
            eprintln!("[dmvop] Failed to build engine: {}", e);
            std::process::exit(1);
        }
    };

    let status = engine.check_model_status();
    if status.available {
        debug_log!("[dmvop] Model already exists at: {}", status.path);
        return;
    }

    debug_log!(
        "[dmvop] Downloading {} ({} MB)...",
        model.config_key(),
        model.size_mb()
    );

    match engine.download_model().await {
        Ok(_) => {
            eprintln!("[dmvop] Model downloaded to: {}", status.path);
        }
        Err(e) => {
            eprintln!("[dmvop] Failed to download model: {}", e);
            eprintln!("[dmvop] You can manually download from:");
            eprintln!("[dmvop]   {}", model.download_url());
            eprintln!("[dmvop]   Place it at: {}", status.path);
            std::process::exit(1);
        }
    }
}
