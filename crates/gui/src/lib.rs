mod commands;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::list_profiles,
            commands::list_identity_keys,
            commands::create_profile,
            commands::update_profile,
            commands::delete_profile,
        ])
        .run(tauri::generate_context!())
        .expect("error while running sshcli");
}
