use std::process::Command;
use serde_json::{json, Value};

pub fn run_adb(args: &[&str], serial: Option<&str>) -> Result<String, String> {
    let mut cmd = Command::new("adb");
    if let Some(s) = serial {
        cmd.arg("-s").arg(s);
    }
    cmd.args(args);

    let output = cmd.output().map_err(|e| format!("Failed to execute adb: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        
        let mut msg = if stderr.is_empty() {
            if stdout.is_empty() {
                "Unknown error".to_string()
            } else {
                stdout
            }
        } else {
            format!("{} (stdout: {})", stderr, stdout)
        };

        if msg.contains("more than one device/emulator") {
            msg.push_str("\nHint: Use --serial <serial> or -s <serial> to specify a device.");
        }
        
        return Err(format!("ADB error: {}", msg));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn list_devices() -> Result<Value, String> {
    let output = run_adb(&["devices", "-l"], None)?;
    let mut devices = Vec::new();
    
    for line in output.lines().skip(1) {
        if line.trim().is_empty() { continue; }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            devices.push(json!({
                "serial": parts[0],
                "state": parts[1],
                "details": parts[2..].join(" ")
            }));
        }
    }
    
    Ok(json!({ "devices": devices }))
}

pub fn connect(host: &str, serial: Option<&str>) -> Result<Value, String> {
    let output = run_adb(&["connect", host], serial)?;
    Ok(json!({ "stdout": output.trim() }))
}

pub fn disconnect(host: Option<&str>, serial: Option<&str>) -> Result<Value, String> {
    let mut args = vec!["disconnect"];
    if let Some(h) = host {
        args.push(h);
    }
    let output = run_adb(&args, serial)?;
    Ok(json!({ "stdout": output.trim() }))
}

pub fn shell(command: &str, serial: Option<&str>) -> Result<Value, String> {
    let output = run_adb(&["shell", command], serial)?;
    Ok(json!({ "stdout": output.trim() }))
}

pub fn get_device_info(serial: Option<&str>) -> Result<Value, String> {
    let model = run_adb(&["shell", "getprop", "ro.product.model"], serial)?;
    let manufacturer = run_adb(&["shell", "getprop", "ro.product.manufacturer"], serial)?;
    let android_version = run_adb(&["shell", "getprop", "ro.build.version.release"], serial)?;
    let sdk = run_adb(&["shell", "getprop", "ro.build.version.sdk"], serial)?;
    
    Ok(json!({
        "info": {
            "model": model.trim(),
            "manufacturer": manufacturer.trim(),
            "android_version": android_version.trim(),
            "sdk": sdk.trim()
        }
    }))
}

pub fn input_key(keycode: &str, serial: Option<&str>) -> Result<Value, String> {
    wake_screen(serial)?;
    run_adb(&["shell", "input", "keyevent", keycode], serial)?;
    Ok(json!({ "status": "success", "action": "key", "keycode": keycode }))
}

pub fn input_tap(x: i32, y: i32, serial: Option<&str>) -> Result<Value, String> {
    wake_screen(serial)?;
    run_adb(&["shell", "input", "tap", &x.to_string(), &y.to_string()], serial)?;
    Ok(json!({ "status": "success", "action": "tap", "x": x, "y": y }))
}

pub fn input_text(text: &str, serial: Option<&str>) -> Result<Value, String> {
    wake_screen(serial)?;
    // Escape spaces and special chars for shell
    let escaped = text.replace(" ", "%s").replace("'", "\\'");
    run_adb(&["shell", "input", "text", &escaped], serial)?;
    Ok(json!({ "status": "success", "action": "text", "value": text }))
}

pub fn input_swipe(x1: i32, y1: i32, x2: i32, y2: i32, duration: i32, serial: Option<&str>) -> Result<Value, String> {
    wake_screen(serial)?;
    run_adb(&["shell", "input", "swipe", &x1.to_string(), &y1.to_string(), &x2.to_string(), &y2.to_string(), &duration.to_string()], serial)?;
    Ok(json!({ "status": "success", "action": "swipe" }))
}

pub fn install(path: &str, serial: Option<&str>) -> Result<Value, String> {
    let output = run_adb(&["install", "-r", path], serial)?;
    Ok(json!({ "stdout": output.trim() }))
}

pub fn uninstall(package: &str, serial: Option<&str>) -> Result<Value, String> {
    let output = run_adb(&["uninstall", package], serial)?;
    Ok(json!({ "stdout": output.trim() }))
}

pub fn list_packages(serial: Option<&str>) -> Result<Value, String> {
    let output = run_adb(&["shell", "pm", "list", "packages"], serial)?;
    let packages: Vec<String> = output.lines()
        .filter_map(|l| l.strip_prefix("package:").map(|s| s.trim().to_string()))
        .collect();
    Ok(json!({ "stdout": packages.join("\n") }))
}

pub fn start_activity(target: &str, serial: Option<&str>) -> Result<Value, String> {
    let output = run_adb(&["shell", "am", "start", "-n", target], serial)?;
    Ok(json!({ "stdout": output.trim() }))
}

pub fn stop_package(package: &str, serial: Option<&str>) -> Result<Value, String> {
    let output = run_adb(&["shell", "am", "force-stop", package], serial)?;
    Ok(json!({ "stdout": output.trim() }))
}

pub fn wake_screen(serial: Option<&str>) -> Result<(), String> {
    // Check if screen is already on
    let output = run_adb(&["shell", "dumpsys", "power"], serial)?;
    if output.contains("mWakefulness=Awake") {
        return Ok(());
    }

    // 224 is KEYCODE_WAKEUP
    run_adb(&["shell", "input", "keyevent", "224"], serial)?;
    // Wait for screen to turn on
    std::thread::sleep(std::time::Duration::from_millis(1000));
    Ok(())
}

pub fn screenshot(path: Option<&str>, serial: Option<&str>) -> Result<Value, String> {
    let remote = "/sdcard/screenshot.png";
    // If path is not provided, use screenshot_{serial}.png to avoid conflicts
    let default_name = format!("screenshot_{}.png", serial.unwrap_or("default"));
    let local = path.unwrap_or(&default_name);
    
    // Ensure screen is on
    wake_screen(serial)?;
    
    run_adb(&["shell", "screencap", "-p", remote], serial)?;
    run_adb(&["pull", remote, local], serial)?;
    run_adb(&["shell", "rm", remote], serial)?;
    
    Ok(json!({ "status": "success", "path": local, "message": format!("Screenshot saved to {}", local) }))
}

pub fn record(path: &str, duration: u64, serial: Option<&str>) -> Result<Value, String> {
    let remote = "/sdcard/screenrecord.mp4";
    
    // This is tricky because screenrecord blocks. We need to run it, wait, then kill it or let it finish.
    // For now, let's just run it with a time limit if possible, or use the time limit flag --time-limit
    
    let duration_str = duration.to_string();
    run_adb(&["shell", "screenrecord", "--time-limit", &duration_str, remote], serial)?;
    run_adb(&["pull", remote, path], serial)?;
    run_adb(&["shell", "rm", remote], serial)?;
    
    Ok(json!({ "status": "success", "path": path, "message": format!("Recording saved to {}", path) }))
}

pub fn push(local: &str, remote: &str, serial: Option<&str>) -> Result<Value, String> {
    let output = run_adb(&["push", local, remote], serial)?;
    Ok(json!({ "stdout": output.trim() }))
}

pub fn pull(remote: &str, local: Option<&str>, serial: Option<&str>) -> Result<Value, String> {
    let mut args = vec!["pull", remote];
    if let Some(l) = local {
        args.push(l);
    }
    let output = run_adb(&args, serial)?;
    Ok(json!({ "stdout": output.trim() }))
}
