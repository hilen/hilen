use std::collections::HashSet;

use rapier3d::{
    geometry::{ColliderHandle, Ray as RapierRay},
    parry::query::ShapeCastOptions,
    pipeline::{QueryFilter, QueryPipeline},
    prelude::Pose3,
};

use crate::{
    deps::refs::{Own, Weak},
    gm::volume::{Quat, Ray, Vec3},
    scene::{ColliderShape, Node, Player, SceneBase},
};

/// Where a query met a node's collider.
#[derive(Clone, Copy)]
pub struct QueryHit {
    pub node:     Weak<dyn Node>,
    /// How far the ray or the swept shape travelled to the hit.
    pub distance: f32,
    /// The point on the node's collider that was met.
    pub point:    Vec3,
    /// The collider's outward surface normal at `point`.
    pub normal:   Vec3,
}

/// Questions about the colliders of a scene with physics, the way a game
/// asks what the camera would pass through or what a swing hits. Nodes in
/// `skip` are left out, and so is the scene's own player, which is no
/// node and holds the camera. A scene without physics has no colliders
/// and every query finds nothing.
impl SceneBase {
    /// The nearest collider along `ray` within `max_distance`.
    pub fn cast_ray(&self, ray: Ray, max_distance: f32, skip: &[Weak<dyn Node>]) -> Option<QueryHit> {
        let direction = ray.direction.normalize();
        let rapier_ray = RapierRay::new(ray.origin, direction);
        self.query(skip, |queries| {
            let (handle, hit) = queries.cast_ray_and_get_normal(&rapier_ray, max_distance, true)?;
            Some((
                handle,
                hit.time_of_impact,
                ray.origin + direction * hit.time_of_impact,
                hit.normal,
            ))
        })
    }

    /// The first collider `shape` runs into when it is swept from `from`,
    /// turned by `rotation`, along `direction` for up to `max_distance`.
    /// A shape that already overlaps something at `from` hits it at 0.
    pub fn cast_shape(
        &self,
        shape: &ColliderShape,
        from: Vec3,
        rotation: Quat,
        direction: Vec3,
        max_distance: f32,
        skip: &[Weak<dyn Node>],
    ) -> Option<QueryHit> {
        let solid = shape.builder(1.0).shape;
        let options = ShapeCastOptions {
            compute_impact_geometry_on_penetration: true,
            ..ShapeCastOptions::with_max_time_of_impact(max_distance)
        };
        self.query(skip, |queries| {
            let pose = Pose3::from_parts(from, rotation);
            let (handle, hit) = queries.cast_shape(&pose, direction.normalize(), &*solid, options)?;
            // The scene's colliders are the first shape of the cast, their
            // side comes back in the world already. The second side is
            // the swept shape in its own space.
            Some((handle, hit.time_of_impact, hit.witness1, hit.normal1))
        })
    }

    /// Every node whose collider overlaps `shape` placed at `at`, turned
    /// by `rotation`, the way a swing's hitbox finds what it strikes.
    pub fn overlapping(
        &self,
        shape: &ColliderShape,
        at: Vec3,
        rotation: Quat,
        skip: &[Weak<dyn Node>],
    ) -> Vec<Weak<dyn Node>> {
        let Some(physics) = &self.physics else {
            return vec![];
        };
        let solid = shape.builder(1.0).shape;
        let excluded = self.excluded(skip);
        let keep = |handle: ColliderHandle, _: &_| !excluded.contains(&handle);
        let queries = physics.broad_phase.as_query_pipeline(
            physics.narrow_phase.query_dispatcher(),
            &physics.sets.rigid_bodies,
            &physics.sets.colliders,
            QueryFilter::default().predicate(&keep),
        );
        let handles: Vec<ColliderHandle> = queries
            .intersect_shape(Pose3::from_parts(at, rotation), &*solid)
            .map(|(handle, _)| handle)
            .collect();
        handles.into_iter().filter_map(|handle| self.node_with(handle)).collect()
    }

    /// Runs `ask` on a query pipeline that leaves out `skip` and the
    /// player, and turns the collider it answers with into its node.
    fn query(
        &self,
        skip: &[Weak<dyn Node>],
        ask: impl FnOnce(&QueryPipeline) -> Option<(ColliderHandle, f32, Vec3, Vec3)>,
    ) -> Option<QueryHit> {
        let physics = self.physics.as_ref()?;
        let excluded = self.excluded(skip);
        let keep = |handle: ColliderHandle, _: &_| !excluded.contains(&handle);
        let queries = physics.broad_phase.as_query_pipeline(
            physics.narrow_phase.query_dispatcher(),
            &physics.sets.rigid_bodies,
            &physics.sets.colliders,
            QueryFilter::default().predicate(&keep),
        );
        let (handle, distance, point, normal) = ask(&queries)?;
        Some(QueryHit {
            node: self.node_with(handle)?,
            distance,
            point,
            normal: normal.normalize_or_zero(),
        })
    }

    fn excluded(&self, skip: &[Weak<dyn Node>]) -> HashSet<ColliderHandle> {
        skip.iter()
            .filter(|node| node.is_ok())
            .filter_map(|node| node.collider_handle())
            .chain(self.player.as_ref().map(Player::collider_handle))
            .collect()
    }

    fn node_with(&self, handle: ColliderHandle) -> Option<Weak<dyn Node>> {
        self.nodes
            .iter()
            .find(|node| node.collider_handle() == Some(handle))
            .map(Own::weak)
    }
}
