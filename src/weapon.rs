use std::{collections::HashSet, path::{Path, PathBuf}, time::Instant};

use macroquad::{audio::{load_sound, load_sound_from_bytes, play_sound, play_sound_once, PlaySoundParams}, color::Color, input::{is_key_released, KeyCode}, math::Vec2, rand::RandomRange};
use nalgebra::{point, vector, Isometry2, Vector2};
use rapier2d::{math::{Translation, Vector}, parry::query::Ray, prelude::{ColliderHandle, ImpulseJointHandle, InteractionGroups, QueryFilter, RevoluteJointBuilder, RigidBodyBuilder, RigidBodyHandle}};
use serde::{Deserialize, Serialize};

use crate::{area::AreaId, bullet_trail::{self, BulletTrail, SpawnBulletTrail}, collider_from_texture_size, draw_preview, draw_texture_onto_physics_body, dropped_item::{DroppedItem, NewDroppedItemUpdate}, enemy::{Enemy, EnemyId}, get_preview_resolution, inventory::Inventory, player::{ActiveWeaponUpdate, Facing, Player, PlayerId}, prop::{DissolvedPixel, Prop, PropVelocityUpdate}, space::Space, texture_loader::TextureLoader, ClientId, ClientTickContext, Prefabs};

