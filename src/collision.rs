use bevy::ecs::component::Component;
use bevy::math::Vec3;

pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn from_center_half_extents(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            min: center - half_extents,
            max: center + half_extents,
        }
    }

    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
            && self.min.z < other.max.z
            && self.max.z > other.min.z
    }
}

#[derive(Component)]
pub struct Collider {
    pub half_extents: Vec3,
}

pub fn resolve_movement(
    current_pos: Vec3,
    displacement: Vec3,
    mover_half_extents: Vec3,
    colliders: &[(Vec3, Vec3)],
) -> Vec3 {
    let mut result = current_pos;

    let candidate_x = Vec3::new(result.x + displacement.x, result.y, result.z);
    let mover_aabb = Aabb::from_center_half_extents(candidate_x, mover_half_extents);
    let mut blocked = false;
    for (pos, half_ext) in colliders {
        let collider_aabb = Aabb::from_center_half_extents(*pos, *half_ext);
        if mover_aabb.intersects(&collider_aabb) {
            blocked = true;
            break;
        }
    }
    if !blocked {
        result.x = candidate_x.x;
    }

    let candidate_z = Vec3::new(result.x, result.y, result.z + displacement.z);
    let mover_aabb = Aabb::from_center_half_extents(candidate_z, mover_half_extents);
    let mut blocked = false;
    for (pos, half_ext) in colliders {
        let collider_aabb = Aabb::from_center_half_extents(*pos, *half_ext);
        if mover_aabb.intersects(&collider_aabb) {
            blocked = true;
            break;
        }
    }
    if !blocked {
        result.z = candidate_z.z;
    }

    result
}
