use rapier2d::{
    na::Vector2,
    prelude::{
        BroadPhase, CCDSolver, ColliderSet, EventHandler, ImpulseJointSet, IntegrationParameters,
        IslandManager, MultibodyJointSet, NarrowPhase, PhysicsHooks, PhysicsPipeline, RigidBody,
        RigidBodyHandle, RigidBodySet,
    },
};
// use salva2d::{
//     integrations::rapier::{ColliderSampling, FluidsPipeline},
//     object::Boundary,
// };

pub const PHYSICS_SCALE: f32 = 10.0;

// const PARTICLE_RADIUS: f32 = 0.19;
// const SMOOTHING_FACTOR: f32 = 2.0;

pub struct Physics {
    // pub fluid_pipeline: FluidsPipeline,
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
    pub gravity: Vector2<f32>,
    pub integration_parameters: IntegrationParameters,
    pub physics_pipeline: PhysicsPipeline,
    pub islands: IslandManager,
    pub broad_phase: BroadPhase,
    pub narrow_phase: NarrowPhase,
    pub ccd_solver: CCDSolver,
    pub impulse_joints: ImpulseJointSet,
    pub multibody_joints: MultibodyJointSet,
    pub hooks: Box<dyn PhysicsHooks>,
    pub event_handler: Box<dyn EventHandler>,
}

impl Physics {
    pub fn new() -> Self {
        let bodies = RigidBodySet::new();
        let colliders = ColliderSet::new();
        // let mut fluid_pipeline = FluidsPipeline::new(PARTICLE_RADIUS, SMOOTHING_FACTOR);

        // let mut points1: Vec<Point2<f32>> = Vec::new();
        // let mut points2 = Vec::new();
        // let ni = 25;
        // let nj = 15;
        // for i in 0..ni / 2 {
        //     for j in 0..nj {
        //         let x = (i as f32) * PARTICLE_RADIUS * 2.0 - ni as f32 * PARTICLE_RADIUS;
        //         let y = (j as f32 + 1.0) * PARTICLE_RADIUS * 2.0 - 10.0;
        //         points1.push(Point2::new(x, y));
        //         points2.push(Point2::new(x + ni as f32 * PARTICLE_RADIUS, y));
        //     }
        // }

        // for i in 0..100 {
        //     for j in -10..nj {
        //         let x = (i as f32) * PARTICLE_RADIUS * 4.0 - 25.0 - ni as f32 * PARTICLE_RADIUS;
        //         let y = (j as f32 + 1.0) * PARTICLE_RADIUS * 2.0 - 20.0;
        //         points2.push(Point2::new(x + ni as f32 * PARTICLE_RADIUS, y));
        //     }
        // }

        // let elasticity: Becker2009Elasticity = Becker2009Elasticity::new(1_000.0, 0.3, true);
        // let viscosity = XSPHViscosity::new(0.5, 1.0);
        // let mut fluid = Fluid::new(points1, PARTICLE_RADIUS, 1.0);
        // fluid.nonpressure_forces.push(Box::new(elasticity));
        // fluid.nonpressure_forces.push(Box::new(viscosity.clone()));
        // let fluid_handle = fluid_pipeline.liquid_world.add_fluid(fluid);

        // // let viscosity = XSPHViscosity::new(0.5, 1.0);
        // let mut fluid = Fluid::new(points2, PARTICLE_RADIUS, 1.0);
        // // fluid.nonpressure_forces.push(Box::new(viscosity.clone()));
        // let fluid_handle = fluid_pipeline.liquid_world.add_fluid(fluid);

        // let rigid_body = RigidBodyBuilder::fixed()
        //     .position(Isometry2::new(Vector2::new(0.0, 20.0), 0.0))
        //     .build();
        // let handle = bodies.insert(rigid_body);
        // let collider = ColliderBuilder::cuboid(10.0, 1.0).build();
        // let co_handle = colliders.insert_with_parent(collider, handle, &mut bodies);
        // let bo_handle = fluid_pipeline
        //     .liquid_world
        //     .add_boundary(Boundary::new(Vec::new()));
        // fluid_pipeline.coupling.register_coupling(
        //     bo_handle,
        //     co_handle,
        //     ColliderSampling::DynamicContactSampling,
        // );

        #[allow(clippy::default_trait_access)]
        Self {
            // fluid_pipeline,
            bodies,
            colliders,
            gravity: Vector2::y() * 3.0,
            hooks: Box::new(()),
            event_handler: Box::new(()),
            integration_parameters: Default::default(),
            physics_pipeline: Default::default(),
            islands: Default::default(),
            broad_phase: Default::default(),
            narrow_phase: Default::default(),
            ccd_solver: Default::default(),
            impulse_joints: Default::default(),
            multibody_joints: Default::default(),
        }
    }

    pub fn step(&mut self, _time_step: f32) {
        // self.fluid_pipeline
        //     .step(&self.gravity, time_step, &self.colliders, &mut self.bodies);

        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            None,
            &*self.hooks,
            &*self.event_handler,
        );
    }

    pub fn remove_rigidbody(&mut self, handle: RigidBodyHandle) -> Option<RigidBody> {
        self.bodies.remove(
            handle,
            &mut self.islands,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            true,
        )
    }
}

impl Default for Physics {
    fn default() -> Self {
        Self::new()
    }
}
