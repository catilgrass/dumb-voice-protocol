use cpal::traits::{DeviceTrait, HostTrait};

/// Pick the best input device from the available list.
///
/// - `"auto"` → query OS default input device via cpal, match against list
/// - `"<name>"` → match by device ID or name; exits if not found
/// - `None` → exits with error
pub fn pick_device<'a>(
    name: &Option<String>,
    devices: &'a [vtx_engine::AudioDevice],
) -> Option<&'a vtx_engine::AudioDevice> {
    let name = match name {
        Some(n) if n == "auto" || n.is_empty() => return pick_auto(devices),
        Some(n) => n.as_str(),
        None => {
            eprintln!(
                "[dmvop] No device specified. Use --device=<name> or --list-devices to see available devices."
            );
            std::process::exit(1);
        }
    };

    // Try exact match by id or name
    if let Some(d) = devices.iter().find(|d| d.id == name || d.name == name) {
        return Some(d);
    }

    eprintln!(
        "[dmvop] Device '{}' not found. Use --list-devices to see available devices.",
        name
    );
    std::process::exit(1);
}

/// Use cpal to find the system's default input device, then match it
/// against the vtx-engine device list by WASAPI ID.
fn pick_auto<'a>(devices: &'a [vtx_engine::AudioDevice]) -> Option<&'a vtx_engine::AudioDevice> {
    let host = cpal::default_host();
    let device = match host.default_input_device() {
        Some(d) => d,
        None => {
            eprintln!("[dmvop] No default input device found.");
            return devices.first();
        }
    };

    // Get the WASAPI device ID from cpal
    let cpal_device_id = match device.id() {
        Ok(id) => id,
        Err(_) => return devices.first(),
    };
    let cpal_raw_id = cpal_device_id.id().to_string();

    // Match against vtx-engine devices by ID (substring — cpal might omit braces)
    for dev in devices {
        let dev_id_clean = dev.id.trim_matches('{').trim_matches('}');
        let cpal_id_clean = cpal_raw_id.trim_matches('{').trim_matches('}');
        if dev_id_clean.contains(cpal_id_clean) || cpal_id_clean.contains(dev_id_clean) {
            return Some(dev);
        }
    }

    // Fallback: match by name (cpal DeviceId Display includes host:raw_id)
    let cpal_display = cpal_device_id.to_string().to_lowercase();
    for dev in devices {
        if cpal_display.contains(&dev.name.to_lowercase())
            || dev.name.to_lowercase().contains(&cpal_display)
        {
            return Some(dev);
        }
    }

    eprintln!(
        "[dmvop] Default device ({}) not matched, using first available.",
        cpal_raw_id
    );
    devices.first()
}
