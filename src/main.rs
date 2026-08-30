mod presets;

use std::env;
use std::fs::{File, metadata, read_to_string};
use std::io::Write;
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

// Hardware backend using sysfs
struct SysfsBackend {
    display: String,
    path: String,
    max_val: u32,
}

impl SysfsBackend {
    fn new(display: String, path: String) -> Result<Self, String> {
        let max_val = read_to_string(format!("{}/max_brightness", path))
            .map_err(|e| format!("Failed to read max_brightness: {}", e))?
            .trim()
            .parse::<u32>()
            .map_err(|e| format!("Invalid max_brightness value: {}", e))?;
        Ok(Self {
            display,
            path,
            max_val,
        })
    }
}

impl Backend for SysfsBackend {
    fn apply(&self, brightness: u8, temp: u8) -> Result<(), String> {
        let hw_val = if brightness >= 10 {
            ((((brightness - 10) as f32 * 100.0 / 90.0) / 100.0) * self.max_val as f32) as u32
        } else {
            0
        };
        let sw_val = if brightness >= 10 {
            1.0
        } else {
            0.3 + (brightness as f32 * 0.7 / 10.0)
        };

        let mut file = File::create(format!("{}/brightness", self.path))
            .map_err(|e| format!("Failed to open brightness file: {}", e))?;
        write!(file, "{}", hw_val).map_err(|e| format!("Failed to write brightness: {}", e))?;

        let g = 1.0 - (temp as f32 * 0.0025);
        let bl = 1.0 - (temp as f32 * 0.005);
        let cmd = format!(
            "xrandr --output {} --brightness {:.2} --gamma 1.0:{:.2}:{:.2}",
            self.display, sw_val, g, bl
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
    if let Ok(entries) = std::fs::read_dir("/sys/class/backlight") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.join("brightness").exists()
                && metadata(path.join("brightness")).is_ok()
                && let Ok(backend) =
                    SysfsBackend::new(display.to_string(), path.to_string_lossy().to_string())
            {
                return Box::new(backend);
            }
        }
    }

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
