mod adapters;
mod commands;
mod error;
mod fsops;
mod lockfile;
mod model;
mod scanner;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_agents,
            commands::scan_skills,
            commands::scan_all_skills,
            commands::get_skill_detail,
            commands::search_skills,
            commands::mount_skill,
            commands::unmount_skill,
            commands::update_frontmatter,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
