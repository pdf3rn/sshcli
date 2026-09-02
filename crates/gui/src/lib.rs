mod commands;
mod local_shell;
mod session;
mod sftp_session;
mod telemetry;
mod tunnel;

pub fn run() {
    tauri::Builder::default()
        .manage(session::init_state())
        .manage(sftp_session::init_state())
        .manage(local_shell::init_state())
        .manage(tunnel::init_state())
        .manage(telemetry::init_state())
        .invoke_handler(tauri::generate_handler![
            commands::list_profiles,
            commands::list_identity_keys,
            commands::create_profile,
            commands::update_profile,
            commands::delete_profile,
            commands::save_profile_secret,
            commands::ssh_trust_host_key,
            commands::toggle_favorite,
            commands::touch_last_used,
            commands::import_profiles,
            commands::export_profiles,
            session::ssh_connect,
            session::ssh_connect_adhoc,
            session::ssh_write,
            session::ssh_resize,
            session::ssh_exec,
            session::ssh_list,
            session::ssh_close,
            local_shell::local_shell_detect,
            local_shell::local_shell_start,
            local_shell::local_write,
            local_shell::local_resize,
            local_shell::local_close,
            sftp_session::sftp_connect,
            sftp_session::sftp_close,
            sftp_session::sftp_list_dir,
            sftp_session::sftp_pwd,
            sftp_session::sftp_download,
            sftp_session::sftp_upload,
            sftp_session::sftp_mkdir,
            sftp_session::sftp_rm_file,
            sftp_session::sftp_rm_dir,
            sftp_session::sftp_file_exists,
            sftp_session::list_local_dir,
            sftp_session::local_home,
            sftp_session::local_file_exists,
            tunnel::tunnel_start,
            tunnel::tunnel_list,
            tunnel::tunnel_stop,
            telemetry::telemetry_sample,
            telemetry::telemetry_disconnect,
        ])
        .run(tauri::generate_context!())
        .expect("error while running sshcli");
}
