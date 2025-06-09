use bevy::{asset::Handle, gltf::Gltf, prelude::Resource};
use bevy_asset_loader::asset_collection::AssetCollection;

#[derive(AssetCollection, Resource)]
pub struct ModelAssets {
    #[asset(path = "models/bg.glb")]
    pub background: Handle<Gltf>,
    #[asset(path = "models/bird.glb")]
    pub bird: Handle<Gltf>,
    #[asset(path = "models/log.glb")]
    pub log: Handle<Gltf>,
}
