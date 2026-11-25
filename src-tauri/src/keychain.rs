use security_framework::passwords::{delete_generic_password, get_generic_password, set_generic_password};

const SERVICE: &str = "io.github.screentoskill";
const ACCOUNTS: [&str; 4] = ["groq", "gemini", "openrouter", "custom"];
const NOT_FOUND: i32 = -25300;

fn check(account: &str) -> Result<(), String> {
    if ACCOUNTS.contains(&account) {
        Ok(())
    } else {
        Err(format!("{account} does not use an API key"))
    }
}

pub fn get(account: &str) -> Result<Option<String>, String> {
    if check(account).is_err() {
        return Ok(None);
    }
    match get_generic_password(SERVICE, account) {
        Ok(bytes) => String::from_utf8(bytes).map(Some).map_err(|e| e.to_string()),
        Err(e) if e.code() == NOT_FOUND => Ok(None),
        Err(e) => Err(format!("Keychain: {e}")),
    }
}

#[tauri::command(async)]
pub fn set_api_key(account: String, key: String) -> Result<(), String> {
    check(&account)?;
    let key = key.trim();
    if key.is_empty() || key.contains(char::is_whitespace) {
        return Err("That does not look like an API key.".into());
    }
    set_generic_password(SERVICE, &account, key.as_bytes()).map_err(|e| format!("Keychain: {e}"))
}

#[tauri::command(async)]
pub fn has_api_key(account: String) -> Result<bool, String> {
    Ok(get(&account)?.is_some())
}

#[tauri::command(async)]
pub fn delete_api_key(account: String) -> Result<(), String> {
    check(&account)?;
    match delete_generic_password(SERVICE, &account) {
        Err(e) if e.code() != NOT_FOUND => Err(format!("Keychain: {e}")),
        _ => Ok(()),
    }
}
