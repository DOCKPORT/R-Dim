mod presets;

use std::env;
use std::process::{Command, Stdio};

// ============= BACKEND TRAITS =============
trait Backend {
    fn apply(&self, brightness: u8, temp: u8) -> Result<(), String>;
}

// ============= BACKEND IMPLEMENTATIONS =============

// Software backend using xrandr
struct XrandrBackend(String);

impl XrandrBackend {
    fn new(display: String) -> Self {
        Self(display)
    }
}

impl Backend for XrandrBackend {
    fn apply(&self, brightness: u8, temp: u8) -> Result<(), String> {
        let b = (brightness as f32 / 100.0).max(0.05);
        let g = 1.0 - (temp as f32 * 0.0025);
        let bl = 1.0 - (temp as f32 * 0.005);
        let cmd = format!(
            "xrandr --output {} --brightness {:.2} --gamma 1.0:{:.2}:{:.2}",
            self.0, b, g, bl
        );
        Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("xrandr failed: {}", e))?;
        Ok(())
    }
}

// Hardware backend using DDC/CI
struct DDCBackend(String);

impl DDCBackend {
    fn new(display: String) -> Self {
        Self(display)
    }
}

impl Backend for DDCBackend {
    fn apply(&self, brightness: u8, temp: u8) -> Result<(), String> {
        let hw_val = if brightness >= 10 {
            ((brightness - 10) as f32 * 100.0 / 90.0) as u8
        } else {
            0
        };
        let sw_val = if brightness >= 10 {
            1.0
        } else {
            0.3 + (brightness as f32 * 0.7 / 10.0)
        };

        Command::new("ddcutil")
            .args(["setvcp", "10", &hw_val.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("ddcutil failed: {}", e))?;

        let g = 1.0 - (temp as f32 * 0.0025);
        let bl = 1.0 - (temp as f32 * 0.005);
        let cmd = format!(
            "xrandr --output {} --brightness {:.2} --gamma 1.0:{:.2}:{:.2}",
            self.0, sw_val, g, bl
        );
        Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("xrandr failed: {}", e))?;
        Ok(())
    }
}

// ============= DISPLAY DETECTION =============
fn detect_display() -> Option<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg("xrandr --query | grep ' connected' | cut -d' ' -f1")
        .output()
        .ok()?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(|s| s.trim().to_string())
}

fn select_backend(display: &str) -> Box<dyn Backend> {
    if Command::new("sh")
        .arg("-c")
        .arg("which ddcutil")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
    {
        return Box::new(DDCBackend::new(display.to_string()));
    }

    Box::new(XrandrBackend::new(display.to_string()))
}

// ============= MAIN =============
fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        // Get version from Cargo.toml at compile time
        let version = env!("CARGO_PKG_VERSION");

        println!("r-dim v{}", version);
        println!("Usage: r-dim <preset>");
        println!();
        println!("Available presets:");
        for (i, p) in presets::PRESETS.iter().enumerate() {
            println!(
                "  {}: {} (brightness: {}, temp: {})",
                i + 1,
                p.name,
                p.brightness,
                p.temp
            );
        }
        return Ok(());
    }

    let display = detect_display().ok_or("No display detected")?;
    let backend = select_backend(&display);
    let arg = &args[1];

    // Try number
    if let Ok(index) = arg.parse::<usize>() {
        if index >= 1 && index <= presets::PRESETS.len() {
            let preset = &presets::PRESETS[index - 1];
            backend.apply(preset.brightness, preset.temp)?;
            return Ok(());
        }
        return Err(format!(
            "Invalid preset number. Use 1-{}",
            presets::PRESETS.len()
        ));
    }

    // Try name
    for preset in presets::PRESETS.iter() {
        if preset.name.to_lowercase() == arg.to_lowercase() {
            backend.apply(preset.brightness, preset.temp)?;
            return Ok(());
        }
    }

    Err(format!("Unknown preset: {}", arg))
}
