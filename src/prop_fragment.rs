use std::{collections::HashSet, path::PathBuf};

use async_trait::async_trait;
use macroquad::{camera::{Camera2D, set_camera}, color::WHITE, math::{Rect, Vec2}, prelude::{Material, MaterialParams, gl_use_default_material, gl_use_material, load_material}, texture::{DrawTextureParams, RenderTarget, Texture2D, draw_texture_ex, render_target}};
use rapier2d::prelude::{ColliderHandle, RigidBodyHandle};

use crate::{drawable::{DrawContext, Drawable}, prop::{DESTRUCTION_MASK_FRAGMENT_SHADER, DESTRUCTION_MASK_VERTEXT_SHADER}, rapier_to_macroquad};

pub struct PropFragment {
    sprite_path: PathBuf,
    collider: ColliderHandle,
    body: RigidBodyHandle,
    mask: Option<RenderTarget>,
    shader_material: Option<Material>,
    scale: f32,
    despawn: bool,
    layer: u32
}

impl PropFragment {

    fn draw_mask(
        &mut self,
        draw_context: &DrawContext,
        texture: &Texture2D
    ) {

        if self.shader_material.is_none() {
            self.shader_material = Some(
                load_material(
                    macroquad::prelude::ShaderSource::Glsl 
                    { 
                        vertex: DESTRUCTION_MASK_VERTEXT_SHADER, 
                        fragment: DESTRUCTION_MASK_FRAGMENT_SHADER 
                    }, 
                    MaterialParams { 
                        uniforms: vec![], 
                        textures: vec!["Mask".to_string()],
                        ..Default::default()
                    }
                ).unwrap()
            )
        }

        if self.mask.is_none() {
            self.mask = Some(
                render_target(texture.width() as u32, texture.height() as u32)
            )
        }

        let mask = self.mask.as_mut().unwrap();

        let mut camera = Camera2D::from_display_rect(
            Rect::new(0., 0., mask.texture.width(), mask.texture.height())
        );  

        camera.render_target = Some(mask.clone());
        camera.zoom.y = -camera.zoom.y;

        set_camera(&camera);

        // ACTUALLY DRAW THE INVERSE MASK HERE!
        //draw_rectangle(0., 0., 10., 10., BLACK);

        set_camera(draw_context.default_camera);

    }
}

#[async_trait]
impl Drawable for PropFragment {
    async fn draw(&mut self, draw_context: &DrawContext) {
        if self.despawn {
            return;
        }

        let texture = draw_context.textures.get(&self.sprite_path);

        self.draw_mask(draw_context, texture);

        let mask = self.mask.as_ref().unwrap();
        let material = self.shader_material.as_ref().unwrap();
        material.set_texture("Mask", mask.texture.clone());

        let body = draw_context.space.rigid_body_set.get(self.body).unwrap();

        // this is probably going to be the wrong position
        // we need to make it so that we draw it exactly where it was when it broke off
        // so maybe we need to store some offset?
        // calculate the difference between the center of masses?
        let macroquad_pos = rapier_to_macroquad(body.center_of_mass());

        let size = Vec2::new(texture.width() * self.scale, texture.height() * self.scale);

        gl_use_material(material);
        
        draw_texture_ex(
            texture, 
            macroquad_pos.x - (size.x / 2.), 
            macroquad_pos.y - (size.y / 2.),
            WHITE,
            DrawTextureParams { 
                dest_size: Some(size), 
                rotation: body.rotation().angle() * -1., 
                ..Default::default()
            }
        
        );
        gl_use_default_material();


    }

    fn draw_layer(&self) -> u32 {
        self.layer
    }
}