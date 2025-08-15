
use rapier2d::{na::vector, prelude::{CCDSolver, ColliderSet, DefaultBroadPhase, ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet, NarrowPhase, PhysicsPipeline, QueryPipeline, RigidBodySet}};

#[derive(Default)]
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
    pub ccd_solver: CCDSolver,
    pub query_pipeline: QueryPipeline,
}

impl Space {
    pub fn step(&mut self, dt: web_time::Duration) {

        self.integration_parameters.dt = dt.as_secs_f32();
        

        self.physics_pipeline.step(
            &vector![0., -998.].into(), 
            &self.integration_parameters, 
            &mut self.island_manager, 
            &mut self.broad_phase, 
            &mut self.narrow_phase, 
            &mut self.rigid_body_set, 
            &mut self.collider_set, 
            &mut self.impulse_joint_set, 
            &mut self.multibody_joint_set, 
            &mut self.ccd_solver, 
            Some(&mut self.query_pipeline), 
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
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
        }
    }
}