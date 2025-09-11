use macroquad::{color::{Color, WHITE}, shapes::draw_line};
use nalgebra::Vector2;
use serde::{Deserialize, Serialize};

use crate::{area::AreaId, rapier_to_macroquad, uuid_u64, ClientId, ClientTickContext};

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct BulletTrailId {
    id: u64
}

impl BulletTrailId {
    pub fn new() -> Self {
        Self {
            id: uuid_u64(),
        }
    }
}

pub struct BulletTrail {
    start: Vector2<f32>,
    end: Vector2<f32>,
    color: Color,
    pub owner: ClientId,
    id: BulletTrailId
}

impl BulletTrail {
    pub fn draw(&self) {

        let start_pos = rapier_to_macroquad(self.start);
        let end_pos = rapier_to_macroquad(self.end);

        draw_line(start_pos.x, start_pos.y, end_pos.x, end_pos.y, 5., self.color);
    }

    pub fn client_tick(&mut self, ctx: &ClientTickContext) {
        self.color.a -= 0.3 * ctx.last_tick_duration.as_secs_f32();
    }

    pub fn save(&self) -> BulletTrailSave {
        BulletTrailSave {
            start: self.start,
            end: self.end,
            owner: self.owner,
            id: self.id,
        }
    }

    pub fn from_save(save: BulletTrailSave) -> Self {
        Self::new(save.start, save.end, None, save.owner)
    }

    pub fn new(
        start: Vector2<f32>,
        end: Vector2<f32>,
        color: Option<Color>,
        owner: ClientId
    ) -> Self {

        let color = match color {
            Some(color) => color,
            None => {
                let mut color = WHITE.clone();
                color.a = 0.2;

                color
            },
        };
        Self {
            start,
            end,
            color,
            owner: owner.clone(),
            id: BulletTrailId::new()
        }
    }

    
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct BulletTrailSave {
    start: Vector2<f32>,
    end: Vector2<f32>,
    pub owner: ClientId,
    id: BulletTrailId
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct SpawnBulletTrail {
    pub area_id: AreaId,
    pub save: BulletTrailSave
}