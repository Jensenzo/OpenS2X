//! The Bevy side of skeletal animation (`specs/GFX-animations.md`
//! GFX-ANIM-2, 8): the pool of shared poses, a material per pose and model
//! part holding that part's bone palette, and the system that samples
//! every pose once a frame and writes the palettes.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;

use crate::animation::anim::{self, Clip, PoseKey, PosePool, PoseWant, Skeleton};
use crate::render::models3d::{Bones, LoadedModel, Look, ModelLibrary, ObjectMaterial};

/// A model part drawn in a pose: the part's mesh and its look.
type PartKey = (AssetId<Mesh>, String);

struct PosePart {
    material: Handle<ObjectMaterial>,
    /// The part's slot in the palette buffer.
    offset: u32,
    palette: Vec<usize>,
    basis: Mat4,
    basis_inverse: Mat4,
}

struct BoundPose {
    skeleton: Arc<Skeleton>,
    clip: Option<Arc<Clip>>,
    parts: HashMap<PartKey, PosePart>,
    /// The bones' world matrices (file axes) of the last sample.
    world: Vec<Mat4>,
}

/// The shared poses and the materials of the parts drawn in them.
#[derive(Resource, Default)]
pub struct Animator {
    pub pool: PosePool,
    poses: HashMap<PoseKey, BoundPose>,
    /// The palette slots of the poses let go, for [`animate`] to give back.
    freed: Vec<u32>,
}

/// The mesh and material of each part of a model in a pose.
pub type PoseParts = Vec<(Handle<Mesh>, Handle<ObjectMaterial>)>;

impl Animator {
    /// Take a pose of the pool for one user drawing `model` (GFX-ANIM-2.4:
    /// a looping clip joins its pose in phase, a play-once clip starts a
    /// pose of its own at time 0) and give the materials of its parts:
    /// rigid parts share the library's, skinned parts get the pose's own.
    pub fn attach(
        &mut self,
        want: &PoseWant,
        model: &LoadedModel,
        look: &Look,
        library: &mut ModelLibrary,
        images: &mut Assets<Image>,
        materials: &mut Assets<ObjectMaterial>,
    ) -> (PoseKey, PoseParts) {
        let pose = self.pool.acquire(want);
        let parts = self.parts(&pose, model, look, library, images, materials);
        (pose, parts)
    }

    fn parts(
        &mut self,
        pose: &PoseKey,
        model: &LoadedModel,
        look: &Look,
        library: &mut ModelLibrary,
        images: &mut Assets<Image>,
        materials: &mut Assets<ObjectMaterial>,
    ) -> PoseParts {
        let mut out = Vec::new();
        let Some(skeleton) = model.skeleton.clone() else {
            for p in &model.parts {
                if let Some(m) = library.material(p, look, images, materials) {
                    out.push((p.mesh.clone(), m));
                }
            }
            return out;
        };
        if !self.poses.contains_key(pose) {
            let clip = library.clip(&pose.clip, &model.skeleton_file);
            self.poses.insert(
                pose.clone(),
                BoundPose {
                    world: Vec::new(),
                    skeleton: skeleton.clone(),
                    clip,
                    parts: HashMap::new(),
                },
            );
        }
        let elapsed = self.pool.elapsed(pose).unwrap_or(0.0);
        let bound = self.poses.get_mut(pose).expect("bound above");
        bound.world = world(bound, elapsed, pose.looping);
        let composites = bound.skeleton.composites(&bound.world);
        for p in &model.parts {
            if p.palette.is_empty() {
                if let Some(m) = library.material(p, look, images, materials) {
                    out.push((p.mesh.clone(), m));
                }
                continue;
            }
            let key = (p.mesh.id(), format!("{:?}", look.key()));
            if let Some(found) = bound.parts.get(&key) {
                out.push((p.mesh.clone(), found.material.clone()));
                continue;
            }
            let bones = Bones::from_palette(&p.palette, &model.converted(&composites));
            let Some(material) = library.unshared_material(p, look, Some(bones), images, materials)
            else {
                continue;
            };
            let offset = materials
                .get(&material)
                .map_or(0, |m| crate::render::models3d::palette_offset(&m.params));
            bound.parts.insert(
                key,
                PosePart {
                    material: material.clone(),
                    offset,
                    palette: p.palette.clone(),
                    basis: model.basis,
                    basis_inverse: model.basis_inverse,
                },
            );
            out.push((p.mesh.clone(), material));
        }
        out
    }

    pub fn detach(&mut self, pose: &PoseKey) {
        self.pool.release(pose);
        if self.pool.elapsed(pose).is_none() {
            if let Some(bound) = self.poses.remove(pose) {
                self.freed.extend(bound.parts.values().map(|p| p.offset));
            }
        }
    }

    /// GFX-ANIM-2.2, 3.3: a pose playing once has run its clip to the end
    /// (`true` also for a pose without a clip).
    pub fn finished(&self, pose: &PoseKey) -> bool {
        let Some(elapsed) = self.pool.elapsed(pose) else {
            return true;
        };
        match self.poses.get(pose).and_then(|b| b.clip.as_ref()) {
            Some(clip) => !pose.looping && elapsed >= clip.duration,
            None => true,
        }
    }

    /// A clip's length in animation seconds, if it is loaded.
    pub fn duration(&self, pose: &PoseKey) -> Option<f32> {
        self.poses
            .get(pose)
            .and_then(|b| b.clip.as_ref())
            .map(|c| c.duration)
    }

    /// GFX-ANIM-3.5: a bone's world matrix (file axes) in a pose, as last
    /// sampled.
    pub fn bone_world(&self, pose: &PoseKey, bone: usize) -> Option<Mat4> {
        self.poses.get(pose)?.world.get(bone).copied()
    }

    /// Forget every pose (a new map).
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// The bones' world matrices of a pose after `elapsed` animation seconds
/// (the rest pose without a clip).
fn world(bound: &BoundPose, elapsed: f32, looping: bool) -> Vec<Mat4> {
    let locals = match &bound.clip {
        Some(clip) => clip.locals(&bound.skeleton, clip.time_at(elapsed, looping), looping),
        None => bound.skeleton.rest.clone(),
    };
    bound.skeleton.world(&locals)
}

/// GFX-ANIM-2.5, TOOL-1.2: advance the poses and the pool's sweep by the
/// game time step (TOOL-trace TRACE-5.3), sample each pose once and write
/// the palettes of the parts drawn in it.
pub fn animate(
    clock: Res<crate::game::GameClock>,
    mut animator: ResMut<Animator>,
    mut library: ResMut<crate::render::models3d::ModelLibrary>,
    materials: Res<Assets<ObjectMaterial>>,
) {
    let animator = &mut *animator;
    for offset in animator.freed.drain(..) {
        library.palettes.release(offset);
    }
    animator.pool.advance_poses(clock.step);
    animator.pool.advance_sweep(clock.step);
    let pool = &animator.pool;
    for (key, bound) in animator.poses.iter_mut() {
        if pool.users(key) == 0 {
            continue;
        }
        let elapsed = pool.elapsed(key).unwrap_or(0.0);
        bound.world = world(bound, elapsed, key.looping);
        let composites = bound.skeleton.composites(&bound.world);
        for part in bound.parts.values() {
            let converted: Vec<Mat4> = part
                .palette
                .iter()
                .map(|&b| {
                    composites.get(b).map_or(Mat4::IDENTITY, |&c| {
                        anim::conjugate(part.basis, part.basis_inverse, c)
                    })
                })
                .collect();
            // F51: into the shared palette buffer, the material unchanged.
            if let Some(m) = materials.get(&part.material) {
                let offset = crate::render::models3d::palette_offset(&m.params);
                for (slot, c) in converted.into_iter().enumerate().take(anim::MAX_BONES) {
                    library.palettes.write_slot(offset, slot, c);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bound(offset: u32) -> BoundPose {
        let mut parts = HashMap::new();
        parts.insert(
            (AssetId::<Mesh>::default(), String::new()),
            PosePart {
                material: Handle::default(),
                offset,
                palette: Vec::new(),
                basis: Mat4::IDENTITY,
                basis_inverse: Mat4::IDENTITY,
            },
        );
        BoundPose {
            skeleton: Arc::new(Skeleton {
                name: String::new(),
                names: Vec::new(),
                parents: Vec::new(),
                rest: Vec::new(),
                inverse_world: Vec::new(),
            }),
            clip: None,
            parts,
            world: Vec::new(),
        }
    }

    /// F55 (the long play): a pose's palette slots are given back when its last
    /// user leaves, not while another still draws with them.
    // spec: GFX-ANIM-2.4, GFX-ANIM-8.2
    #[test]
    fn the_last_user_gives_a_poses_slots_back() {
        let mut animator = Animator::default();
        let want = PoseWant {
            model: "m".into(),
            clip: "c".into(),
            looping: true,
            private: false,
        };
        let first = animator.pool.acquire(&want);
        let second = animator.pool.acquire(&want);
        assert_eq!(first, second, "a looping clip joins its pose");
        animator.poses.insert(first.clone(), bound(50));
        animator.detach(&first);
        assert!(animator.freed.is_empty(), "one still draws with it");
        assert!(animator.poses.contains_key(&second));
        animator.detach(&second);
        assert_eq!(animator.freed, [50]);
        assert!(animator.poses.is_empty());
    }
}
