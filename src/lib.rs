use std::path::PathBuf;

pub const SERVICE_NAME: &str = "dopamine-sideload";

pub fn data_dir() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("DOPAMINE_HOME") {
        return Ok(PathBuf::from(path));
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .ok_or_else(|| "Nie można ustalić katalogu danych użytkownika (HOME/XDG_DATA_HOME).".to_string())?;
    Ok(base.join(SERVICE_NAME))
}

pub fn ipa_path() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("Dopamine.ipa"))
}

pub fn get_secret(account: &str) -> Result<String, String> {
    let entry = keyring::Entry::new(SERVICE_NAME, account)
        .map_err(|error| format!("Nie można otworzyć menedżera haseł systemu: {error}"))?;
    let secret = entry
        .get_password()
        .map_err(|error| format!("Brak poświadczenia '{account}' w menedżerze haseł: {error}"))?;
    if secret.is_empty() {
        return Err(format!("Poświadczenie '{account}' jest puste."));
    }
    Ok(secret)
}

pub fn set_secret(account: &str, secret: &str) -> Result<(), String> {
    keyring::Entry::new(SERVICE_NAME, account)
        .map_err(|error| format!("Nie można otworzyć menedżera haseł systemu: {error}"))?
        .set_password(secret)
        .map_err(|error| format!("Nie można zapisać poświadczenia w menedżerze haseł systemu: {error}"))
}
