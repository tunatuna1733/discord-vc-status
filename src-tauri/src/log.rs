// TODO: use official tauri logging plugin

pub fn log_error(name: String, contents: String) {
    println!("Error: {name}\n{contents}");
}
