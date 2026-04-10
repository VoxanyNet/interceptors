use derive_more::From;
use rapier2d::prelude::{ColliderHandle, ImpulseJointHandle, RigidBodyHandle};

use crate::{TickContext, area::AreaContext, enemy::EnemyContext, items::Item, player::PlayerContext, space::Space};

pub mod weapon;
pub mod shotgun;
pub mod lmg;
pub mod smg;
pub mod sledge;
pub mod bullet_impact_data;
pub mod weapon_fire_context;
pub mod weapon_type;
pub mod weapon_type_save;

#[derive(From)]
pub enum ItemOwnerContext<'a> {
    Player(PlayerContext<'a>),
    Enemy(EnemyContext<'a>)
}
pub trait Weapon {
    fn collider_handle(&self) -> Option<ColliderHandle>;
    fn fire(&mut self, ctx: &mut TickContext, area_context: &mut AreaContext, weapon_owner_context: &mut ItemOwnerContext);
    fn player_joint_handle(&self) -> Option<ImpulseJointHandle>;
    fn rigid_body_handle(&self) -> Option<RigidBodyHandle>;
}






