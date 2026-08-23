mod commands;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::list_profiles,
            commands::list_identity_keys,
        ])
        .run(tauri::generate_context!())
        .expect("error while running sshcli");
}
