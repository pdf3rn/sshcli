const SERVICE: &str = "sshcli";

fn entry(profile_name: &str) -> Result<keyring::Entry, keyring::Error> {
    keyring::Entry::new(SERVICE, &format!("profile:{profile_name}"))
}

pub fn set(profile_name: &str, secret: &str) -> Result<(), keyring::Error> {
    entry(profile_name)?.set_password(secret)
}

pub fn get(profile_name: &str) -> Result<String, keyring::Error> {
    entry(profile_name)?.get_password()
}

pub fn delete(profile_name: &str) -> Result<(), keyring::Error> {
    entry(profile_name)?.delete_credential()
}
