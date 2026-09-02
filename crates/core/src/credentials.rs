use std::{fs, path::PathBuf};

use crate::{AppError, AppResult};

const SERVICE: &str = "sshcli";

fn profile_key(profile_name: &str) -> String {
    profile_name
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn user(profile_name: &str) -> String {
    format!("profile.{}", profile_key(profile_name))
}

fn fallback_path(profile_name: &str) -> PathBuf {
    crate::config::config_dir()
        .join("secrets")
        .join(format!("{}.dpapi", profile_key(profile_name)))
}

fn entry(profile_name: &str) -> Result<keyring::Entry, keyring::Error> {
    keyring::Entry::new(SERVICE, &user(profile_name))
}

fn legacy_entry(profile_name: &str) -> Result<keyring::Entry, keyring::Error> {
    keyring::Entry::new(SERVICE, &format!("profile:{profile_name}"))
}

pub fn set(profile_name: &str, secret: &str) -> Result<(), keyring::Error> {
    entry(profile_name)?.set_password(secret)
}

pub fn get_optional(profile_name: &str) -> AppResult<Option<String>> {
    match entry(profile_name).and_then(|entry| entry.get_password()) {
        Ok(secret) => return Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => {}
        Err(error) => {
            #[cfg(windows)]
            if fallback_path(profile_name).exists() {
                return get_fallback(profile_name).map(Some);
            }
            return Err(keyring_error(profile_name, error));
        }
    }
    match legacy_entry(profile_name).and_then(|entry| entry.get_password()) {
        Ok(secret) => return Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => {}
        Err(error) => {
            #[cfg(windows)]
            if fallback_path(profile_name).exists() {
                return get_fallback(profile_name).map(Some);
            }
            return Err(keyring_error(profile_name, error));
        }
    }
    #[cfg(windows)]
    {
        if fallback_path(profile_name).exists() {
            return get_fallback(profile_name).map(Some);
        }
    }
    Ok(None)
}

pub fn set_verified(profile_name: &str, secret: &str) -> AppResult<()> {
    let mut errors = Vec::new();
    match set(profile_name, secret).and_then(|_| entry(profile_name)?.get_password()) {
        Ok(saved) if saved == secret => {
            delete_fallback(profile_name)?;
            return Ok(());
        }
        Ok(_) => errors.push("keyring read-back did not match the supplied secret".to_string()),
        Err(error) => errors.push(format!("keyring: {error}")),
    }

    match set_fallback(profile_name, secret).and_then(|_| get_fallback(profile_name)) {
        Ok(saved) if saved == secret => Ok(()),
        Ok(_) => Err(AppError::Credential(format!(
            "credential verification failed for profile {profile_name}: {}; fallback read-back did not match",
            errors.join("; ")
        ))),
        Err(error) => Err(AppError::Credential(format!(
            "cannot save credential for profile {profile_name}: {}; fallback: {error}",
            errors.join("; ")
        ))),
    }
}

pub fn delete(profile_name: &str) -> AppResult<()> {
    let primary = entry(profile_name).and_then(|entry| entry.delete_credential());
    let legacy = legacy_entry(profile_name).and_then(|entry| entry.delete_credential());
    let fallback = delete_fallback(profile_name);
    let mut errors = Vec::new();
    for result in [primary, legacy] {
        if let Err(error) = result {
            if !matches!(error, keyring::Error::NoEntry) {
                errors.push(error.to_string());
            }
        }
    }
    if let Err(error) = fallback {
        errors.push(error.to_string());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(AppError::Credential(format!(
            "cannot delete credential for profile {profile_name}: {}",
            errors.join("; ")
        )))
    }
}

fn keyring_error(profile_name: &str, error: keyring::Error) -> AppError {
    AppError::Credential(format!("cannot read credential for profile {profile_name}: {error}"))
}

#[cfg(windows)]
fn set_fallback(profile_name: &str, secret: &str) -> AppResult<()> {
    let path = fallback_path(profile_name);
    let temporary = path.with_extension("dpapi.tmp");
    let encrypted = dpapi_protect(secret.as_bytes())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&temporary, encrypted)?;
    replace_file(&temporary, &path)?;
    Ok(())
}

#[cfg(not(windows))]
fn set_fallback(profile_name: &str, _secret: &str) -> AppResult<()> {
    Err(AppError::Credential(format!(
        "no encrypted fallback is available for profile {profile_name} on this platform"
    )))
}

#[cfg(windows)]
fn get_fallback(profile_name: &str) -> AppResult<String> {
    let encrypted = fs::read(fallback_path(profile_name))?;
    String::from_utf8(dpapi_unprotect(&encrypted)?)
        .map_err(|error| AppError::Credential(format!("invalid encrypted credential: {error}")))
}

#[cfg(not(windows))]
fn get_fallback(profile_name: &str) -> AppResult<String> {
    Err(AppError::Credential(format!(
        "no encrypted fallback is available for profile {profile_name} on this platform"
    )))
}

fn delete_fallback(profile_name: &str) -> AppResult<()> {
    match fs::remove_file(fallback_path(profile_name)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(windows)]
fn replace_file(source: &std::path::Path, destination: &std::path::Path) -> AppResult<()> {
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::winbase::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH};

    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        return Err(AppError::Credential(format!(
            "cannot replace encrypted credential: Windows error {}",
            unsafe { GetLastError() }
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn dpapi_protect(data: &[u8]) -> AppResult<Vec<u8>> {
    use std::{mem::MaybeUninit, ptr};
    use winapi::shared::minwindef::DWORD;
    use winapi::um::dpapi::{CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN};
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::winbase::LocalFree;
    use winapi::um::wincrypt::DATA_BLOB;

    let mut input = DATA_BLOB {
        cbData: data.len() as DWORD,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = MaybeUninit::<DATA_BLOB>::zeroed();
    if unsafe {
        CryptProtectData(
            &mut input,
            ptr::null(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            output.as_mut_ptr(),
        )
    } == 0
    {
        return Err(AppError::Credential(format!(
            "CryptProtectData failed with Windows error {}",
            unsafe { GetLastError() }
        )));
    }
    let output = unsafe { output.assume_init() };
    let encrypted = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe { LocalFree(output.pbData as _) };
    Ok(encrypted)
}

#[cfg(windows)]
fn dpapi_unprotect(data: &[u8]) -> AppResult<Vec<u8>> {
    use std::{mem::MaybeUninit, ptr};
    use winapi::shared::minwindef::DWORD;
    use winapi::um::dpapi::{CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN};
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::winbase::LocalFree;
    use winapi::um::wincrypt::DATA_BLOB;

    let mut input = DATA_BLOB {
        cbData: data.len() as DWORD,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = MaybeUninit::<DATA_BLOB>::zeroed();
    if unsafe {
        CryptUnprotectData(
            &mut input,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            output.as_mut_ptr(),
        )
    } == 0
    {
        return Err(AppError::Credential(format!(
            "CryptUnprotectData failed with Windows error {}",
            unsafe { GetLastError() }
        )));
    }
    let output = unsafe { output.assume_init() };
    let decrypted = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe { LocalFree(output.pbData as _) };
    Ok(decrypted)
}

#[cfg(test)]
mod tests {
    use super::profile_key;

    #[test]
    fn profile_keys_do_not_collide_for_punctuation() {
        assert_ne!(profile_key("prod/eu"), profile_key("prod:eu"));
        assert_ne!(profile_key("prod/eu"), profile_key("prod_eu"));
    }
}
