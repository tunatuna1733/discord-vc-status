use discord_rich_presence::DiscordIpcClient;
use tauri::async_runtime::Mutex;

pub struct IPCClientManager {
    pub client: Mutex<DiscordIpcClient>,
}

impl IPCClientManager {
    pub fn new(client: DiscordIpcClient) -> Self {
        Self {
            client: Mutex::from(client),
        }
    }
}
