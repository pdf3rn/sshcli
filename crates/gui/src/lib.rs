mod commands;
mod session;

pub fn run() {
    tauri::Builder::default()
        .manage(session::init_state())
        .invoke_handler(tauri::generate_handler![
            commands::list_profiles,
            commands::list_identity_keys,
            commands::create_profile,
            commands::update_profile,
            commands::delete_profile,
            session::ssh_connect,
            session::ssh_write,
            session::ssh_resize,
            session::ssh_list,
            session::ssh_close,
        ])
        .run(tauri::generate_context!())
        .expect("error while running sshcli");
}
