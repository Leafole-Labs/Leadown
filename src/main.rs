#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod buffer;
mod code;
mod i18n;
mod links;
mod md;
mod render;
mod search;
mod spaces;
mod store;
mod theme;
mod vault;
mod watch;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_note,
            save_note,
            list_tree,
            create_note,
            create_folder,
            rename_item,
            delete_item,
            search_notes,
            get_settings,
            save_settings,
            load_spaces,
            save_spaces,
            markdown_to_html,
            get_theme,
        ])
        .run(tauri::generate_context!())
        .expect("error while running leadown");
}

#[tauri::command]
fn load_note(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_note(path: String, content: String) -> Result<(), String> {
    store::write_atomic(std::path::Path::new(&path), content.as_bytes()).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_tree(root: String) -> Result<Vec<vault::Node>, String> {
    vault::scan(std::path::Path::new(&root)).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_note(path: String, content: String) -> Result<(), String> {
    store::write_atomic(std::path::Path::new(&path), content.as_bytes()).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_folder(path: String) -> Result<(), String> {
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn rename_item(from: String, to: String) -> Result<(), String> {
    std::fs::rename(&from, &to).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_item(path: String) -> Result<(), String> {
    trash::delete(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn search_notes(root: String, query: String) -> Vec<search::Hit> {
    search::search(std::path::Path::new(&root), &query)
}

#[tauri::command]
fn get_settings() -> Result<store::Settings, String> {
    Ok(store::Settings::load())
}

#[tauri::command]
fn save_settings(settings: store::Settings) -> Result<(), String> {
    settings.save();
    Ok(())
}

#[tauri::command]
fn load_spaces() -> Result<spaces::Spaces, String> {
    Ok(spaces::load())
}

#[tauri::command]
fn save_spaces(spaces: spaces::Spaces) -> Result<(), String> {
    spaces::save(&spaces);
    Ok(())
}

#[tauri::command]
fn markdown_to_html(text: String) -> Result<String, String> {
    Ok(render::markdown_to_html(&text))
}

#[tauri::command]
fn get_theme() -> Result<theme::ThemePref, String> {
    Ok(theme::ThemePref::System)
}
