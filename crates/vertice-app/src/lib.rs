//! `vertice-app`: the Tauri 2 desktop shell. Owns the Tauri runtime, IPC
//! commands, and the bundled Svelte 5 + Vite + Tailwind frontend. Depends on
//! `vertice-core` (path dependency) for all domain logic — this crate must
//! stay the only place the `tauri` crate is imported in the workspace.

mod commands;
mod freshness;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::scan,
            commands::rescan,
            commands::freshness,
            commands::freshness_settings,
            commands::set_freshness_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Vertice application");
}
