use serde::Serialize;
use std::process::Command;

#[derive(Serialize)]
pub struct Hardware {
    arch: &'static str,
    chip: String,
    ram_gb: u64,
    provider: &'static str,
    model: &'static str,
    reason: &'static str,
}

fn sysctl(key: &str) -> String {
    Command::new("/usr/sbin/sysctl")
        .args(["-n", key])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

pub fn recommend(apple_silicon: bool, ram_gb: u64) -> (&'static str, &'static str, &'static str) {
    match (apple_silicon, ram_gb) {
        (false, _) => (
            "groq",
            "",
            "Intel Macs run local vision models on the CPU, which is slow. A free cloud API is faster; \"No AI\" stays offline.",
        ),
        (true, 0..=15) => ("ollama", "qwen3-vl:2b-instruct", "A small local model fits this much memory."),
        (true, 16..=31) => ("ollama", "qwen3-vl:4b-instruct", "Good balance of speed and accuracy for this Mac."),
        (true, _) => ("ollama", "qwen3-vl:8b-instruct", "This Mac has enough memory for a larger local model."),
    }
}

#[tauri::command]
pub fn hardware() -> Hardware {
    let arm = sysctl("hw.optional.arm64") == "1";
    let ram_gb = sysctl("hw.memsize").parse::<u64>().unwrap_or(0) >> 30;
    let (provider, model, reason) = recommend(arm, ram_gb);
    Hardware {
        arch: if arm { "arm64" } else { "x86_64" },
        chip: sysctl("machdep.cpu.brand_string"),
        ram_gb,
        provider,
        model,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::recommend;

    #[test]
    fn recommendations() {
        assert_eq!(recommend(false, 64).0, "groq");
        assert_eq!(recommend(true, 8).1, "qwen3-vl:2b-instruct");
        assert_eq!(recommend(true, 16).1, "qwen3-vl:4b-instruct");
        assert_eq!(recommend(true, 64).1, "qwen3-vl:8b-instruct");
    }
}
