use tauri::command;

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {name}")
}

#[command(rename_all = "snake_case")]
pub async fn save_file(path: String, data: Vec<u8>) -> Result<(), String> {
    std::fs::write(path, data).map_err(|e| e.to_string())
}

pub fn not_a_command() -> u32 {
    1
}
