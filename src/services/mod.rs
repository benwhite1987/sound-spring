pub mod audio_meta;
pub mod pipewire;
pub mod player;
pub mod shortcuts;
pub mod tabs;

pub use pipewire::PipewireManager;
pub use player::{Player, PlayerCommand, PlayerEvent, VolumeState};
pub use shortcuts::{ShortcutDef, ShortcutEvent, ShortcutsManager};
pub use tabs::{Tab, TabsRepository};
