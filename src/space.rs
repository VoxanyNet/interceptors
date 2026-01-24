
use rapier2d::prelude::{CCDSolver, ColliderSet, DefaultBroadPhase, ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet, NarrowPhase, PhysicsHooks, PhysicsPipeline, RigidBodySet, SolverFlags};

pub struct MyPhysicsHooks;

impl PhysicsHooks for MyPhysicsHooks {
    fn filter_contact_pair(&self, context: &rapier2d::prelude::PairFilterContext) -> Option<rapier2d::prelude::SolverFlags> {
        
        
        let user_data = context.colliders.get(context.collider1).unwrap().user_data;
        let y_vel= context.bodies.get(context.rigid_body2.unwrap()).unwrap().vels().linvel.y;

        if y_vel > 0. && user_data == 1 {
            return None
        }

        //log::debug!("{}", y);

        Some(SolverFlags::COMPUTE_IMPULSES)
    }
}
pub struct Space {
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub integration_parameters: IntegrationParameters,
    pub physics_pipeline: PhysicsPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: DefaultBroadPhase,
    pub narrow_phase: NarrowPhase,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub ccd_solver: CCDSolver
}

impl Space {
    pub fn step(&mut self, dt: web_time::Duration) {

        self.integration_parameters.dt = dt.as_secs_f32();
        

        self.physics_pipeline.step(
            glamx::vec2(0., -998.), 
            &self.integration_parameters, 
            &mut self.island_manager, 
            &mut self.broad_phase, 
            &mut self.narrow_phase, 
            &mut self.rigid_body_set, 
            &mut self.collider_set, 
            &mut self.impulse_joint_set, 
            &mut self.multibody_joint_set, 
            &mut self.ccd_solver, 
            &(),
            &()

        );
    }

    pub fn new() -> Self {
        Self {
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            integration_parameters: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new()
        }
    }
}