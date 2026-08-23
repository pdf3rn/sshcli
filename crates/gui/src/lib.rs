mod commands;
mod session;
mod sftp_session;
mod tunnel;

pub fn run() {
    tauri::Builder::default()
        .manage(session::init_state())
        .manage(sftp_session::init_state())
        .manage(tunnel::init_state())
        .invoke_handler(tauri::generate_handler![
            commands::list_profiles,
            commands::list_identity_keys,
            commands::create_profile,
            commands::update_profile,
            commands::delete_profile,
            commands::touch_last_used,
            commands::import_profiles,
            session::ssh_connect,
            session::ssh_write,
            session::ssh_resize,
            session::ssh_list,
            session::ssh_close,
            sftp_session::sftp_connect,
            sftp_session::sftp_close,
            sftp_session::sftp_list_dir,
            sftp_session::sftp_pwd,
            sftp_session::sftp_download,
            sftp_session::sftp_upload,
            sftp_session::sftp_mkdir,
            sftp_session::sftp_rm_file,
            sftp_session::sftp_rm_dir,
            sftp_session::list_local_dir,
            sftp_session::local_home,
            tunnel::tunnel_start,
            tunnel::tunnel_list,
            tunnel::tunnel_stop,
        ])
        .run(tauri::generate_context!())
        .expect("error while running sshcli");
}
