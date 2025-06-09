use bevy::{asset::Handle, audio::AudioSource, prelude::Resource};
use bevy_asset_loader::asset_collection::AssetCollection;

#[derive(AssetCollection, Resource)]
#[allow(dead_code)]
pub struct AudioAssets {
    #[asset(path = "audio/background.ogg")]
    background: Handle<AudioSource>,
}
