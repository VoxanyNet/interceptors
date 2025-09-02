use macroquad::{color::WHITE, shapes::{draw_circle, draw_rectangle}};
use nalgebra::{point, vector, Isometry2};
use rapier2d::{math::UnitVector, prelude::{ColliderBuilder, ColliderHandle, ImpulseJointHandle, PrismaticJointBuilder, RevoluteJointBuilder, RigidBodyBuilder, RigidBodyHandle}};

use crate::{draw_hitbox, rapier_to_macroquad, space::Space};

pub struct Car {
    platform_body: RigidBodyHandle,
    platform_collider: ColliderHandle,

    left_wheel_body: RigidBodyHandle,
    left_wheel_collider: ColliderHandle,
    left_wheel_revolute_joint: ImpulseJointHandle,

    left_axle_body: RigidBodyHandle,
    left_axle_collider: ColliderHandle,
    left_axle_prismatic_joint: ImpulseJointHandle,


    right_axle_body: RigidBodyHandle,
    right_axle_collider: ColliderHandle,
    right_axle_prismatic_joint: ImpulseJointHandle,
    

    right_wheel_body: RigidBodyHandle,
    right_wheel_collider: ColliderHandle,
    right_wheel_revolute_joint: ImpulseJointHandle,


}

impl Car {
    pub fn new(space: &mut Space, pos: Isometry2<f32>) -> Self {
        
        // platform
        let platform_body = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .position(pos)
        );
        let platform_collider = space.collider_set.insert_with_parent(
            ColliderBuilder::cuboid(50., 10.)
                .mass(500.), 
            platform_body, 
            &mut space.rigid_body_set
        );

        // left wheel
        let left_wheel_body = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .angular_damping(0.)
                .position(vector![pos.translation.x - 50., pos.translation.y - 50.].into())
        );
        let left_wheel_collider = space.collider_set.insert_with_parent(
            ColliderBuilder::ball(20.).mass(100.),
            left_wheel_body,
            &mut space.rigid_body_set
        );

        // right wheel
        let right_wheel_body = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
                .angular_damping(0.)
                .position(vector![pos.translation.x + 50., pos.translation.y - 50.].into())
        );
        let right_wheel_collider = space.collider_set.insert_with_parent(
            ColliderBuilder::ball(20.).mass(100.),
            right_wheel_body,
            &mut space.rigid_body_set
        );

        // left axle
        let left_axle_body = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
        );
        
        let left_axle_collider = space.collider_set.insert_with_parent(
            ColliderBuilder::ball(0.05).sensor(true), 
            left_axle_body, 
            &mut space.rigid_body_set
        );

        // right axle
        let right_axle_body = space.rigid_body_set.insert(
            RigidBodyBuilder::dynamic()
        );
        
        let right_axle_collider = space.collider_set.insert_with_parent(
            ColliderBuilder::ball(0.05).sensor(true), 
            right_axle_body, 
            &mut space.rigid_body_set
        );

        // left suspension
        let susp_axis = UnitVector::new_normalize(vector![0.0, 1.0]);
        let left_axle_prismatic_joint = PrismaticJointBuilder::new(susp_axis)
            .local_anchor1(vector![-50., -50.].into())   // mount point on chassis
            .local_anchor2(point![0.0, 0.0])    // axle center
            .limits([-5., 5.])              // bump/rebound stops (meters)
            .contacts_enabled(false)
            .build();

        let left_axle_prismatic_joint_handle = space.impulse_joint_set.insert(platform_body, left_axle_body, left_axle_prismatic_joint, true);
        

        // right suspension
        let susp_axis = UnitVector::new_normalize(vector![0.0, 1.0]);
        let right_axle_prismatic_joint = PrismaticJointBuilder::new(susp_axis)
            .local_anchor1(vector![50., -50.].into())   // mount point on chassis
            .local_anchor2(point![0.0, 0.0])    // axle center
            .limits([-5., 5.])              // bump/rebound stops (meters)
            .contacts_enabled(false)
            .build();

        let right_axle_prismatic_joint_handle = space.impulse_joint_set.insert(platform_body, right_axle_body, right_axle_prismatic_joint, true);
        
        let left_wheel_revolute_joint = RevoluteJointBuilder::new()
            .local_anchor1(point![0.0, 0.0])    // axle center
            .local_anchor2(point![0.0, 0.0])    // wheel center
            .contacts_enabled(false)
            .build();
        
        let right_wheel_revolute_joint = RevoluteJointBuilder::new()
            .local_anchor1(point![0.0, 0.0])    // axle center
            .local_anchor2(point![0.0, 0.0])    // wheel center
            .contacts_enabled(false)
            .build();

        let left_wheel_revolute_joint_handle = space.impulse_joint_set.insert(left_axle_body, left_wheel_body, left_wheel_revolute_joint, true);
        let right_wheel_revolute_joint_handle = space.impulse_joint_set.insert(right_axle_body, right_wheel_body, right_wheel_revolute_joint, true);

        space.impulse_joint_set.get_mut(left_axle_prismatic_joint_handle, true).unwrap().data.as_prismatic_mut().unwrap().set_motor_position(0., 12000., 80.);
        space.impulse_joint_set.get_mut(right_axle_prismatic_joint_handle, true).unwrap().data.as_prismatic_mut().unwrap().set_motor_position(0., 12000., 80.);
        
        Self {
            platform_body,
            platform_collider,
            left_wheel_body,
            left_wheel_collider,
            left_wheel_revolute_joint: left_wheel_revolute_joint_handle,
            left_axle_body,
            left_axle_collider,
            left_axle_prismatic_joint: left_axle_prismatic_joint_handle,
            right_axle_body,
            right_axle_collider,
            right_axle_prismatic_joint: right_axle_prismatic_joint_handle,
            right_wheel_body,
            right_wheel_collider,
            right_wheel_revolute_joint: right_wheel_revolute_joint_handle,
        }

        

    }

    pub fn draw(&self, space: &Space) {
        let platform_pos = space.rigid_body_set.get(self.platform_body).unwrap().position().translation.vector;

        let left_wheel_pos = rapier_to_macroquad(space.collider_set.get(self.left_wheel_collider).unwrap().position().translation.vector);
        let right_wheel_pos = rapier_to_macroquad(space.collider_set.get(self.right_wheel_collider).unwrap().position().translation.vector);

        let left_wheel_size = space.collider_set.get(self.left_wheel_collider).unwrap().shape().as_ball().unwrap().radius;
        let right_wheel_size = space.collider_set.get(self.right_wheel_collider).unwrap().shape().as_ball().unwrap().radius;


        draw_hitbox(space, self.platform_body, self.platform_collider, WHITE);

        draw_circle(left_wheel_pos.x, left_wheel_pos.y, left_wheel_size, WHITE);
        draw_circle(right_wheel_pos.x, right_wheel_pos.y, right_wheel_size, WHITE);        


        //draw_rectangle(macroquad_platform_pos.x - platform_size.x, macroquad_platform_pos.y + , w, h, color);

    }
}