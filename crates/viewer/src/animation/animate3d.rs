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
    /// F55 (the long play): the pose's users by look. A look's parts and
    /// their palette slots go when its last user leaves: a settler figure's
    /// look holds its own atlas block (GFX-OBJ-3.2 (3)), and a shared pose
    /// that never empties (the walk of a busy map) kept one slot more for
    /// every figure that ever walked in it.
    looks: HashMap<String, u32>,
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
    /// Hook W30d: the shared poses held for good (a figure's looping clips
    /// and walking phases, by model, clip and look), so that figures moving
    /// between them only change materials, never make new ones.
    pinned: std::collections::HashSet<(String, String, String)>,
}

/// F55: the palette slots the poses hold (`--bench`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoseSlots {
    pub poses: usize,
    pub slots: usize,
    /// The most slots one pose holds.
    pub most: usize,
    /// Slots of poses let go, given back at the next [`animate`].
    pub pending: usize,
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
                    looks: HashMap::new(),
                },
            );
        }
        let look_key = format!("{:?}", look.key());
        let elapsed = self.pool.elapsed(pose).unwrap_or(0.0);
        let bound = self.poses.get_mut(pose).expect("bound above");
        *bound.looks.entry(look_key.clone()).or_default() += 1;
        bound.world = world(bound, elapsed, pose.looping);
        let composites = bound.skeleton.composites(&bound.world);
        for p in &model.parts {
            if p.palette.is_empty() {
                if let Some(m) = library.material(p, look, images, materials) {
                    out.push((p.mesh.clone(), m));
                }
                continue;
            }
            let key = (p.mesh.id(), look_key.clone());
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

    /// Hook W30d: hold the shared pose `want` asks for, drawn in `look`,
    /// for good: one user that never leaves (the first time only). A pose
    /// whose last figure leaves then keeps its parts' materials for the
    /// next. At most one pose per figure clip and walking phase and look.
    pub fn pin(
        &mut self,
        want: &PoseWant,
        model: &LoadedModel,
        look: &Look,
        library: &mut ModelLibrary,
        images: &mut Assets<Image>,
        materials: &mut Assets<ObjectMaterial>,
    ) {
        if !want.looping || want.private {
            return;
        }
        let key = (
            want.model.clone(),
            want.clip.clone(),
            format!("{:?}", look.key()),
        );
        if self.pinned.insert(key) {
            let _ = self.attach(want, model, look, library, images, materials);
        }
    }

    /// One user drawing `look` leaves the pose (GFX-ANIM-2.4 c). The last
    /// user of the pose gives all its parts' palette slots back; the last
    /// user of a look its look's (F55).
    pub fn detach(&mut self, pose: &PoseKey, look: &Look) {
        self.pool.release(pose);
        if self.pool.elapsed(pose).is_none() {
            if let Some(bound) = self.poses.remove(pose) {
                self.freed.extend(bound.parts.values().map(|p| p.offset));
            }
            return;
        }
        let Some(bound) = self.poses.get_mut(pose) else {
            return;
        };
        let key = format!("{:?}", look.key());
        let Some(users) = bound.looks.get_mut(&key) else {
            return;
        };
        *users -= 1;
        if *users > 0 {
            return;
        }
        bound.looks.remove(&key);
        let freed = &mut self.freed;
        bound.parts.retain(|(_, part_look), part| {
            let keep = *part_look != key;
            if !keep {
                freed.push(part.offset);
            }
            keep
        });
    }

    /// The palette slots of the poses and looks let go since the last call,
    /// given back to the buffer (each frame, before the poses are sampled).
    pub fn give_back(&mut self, palettes: &mut crate::render::models3d::Palettes) {
        for offset in self.freed.drain(..) {
            palettes.release(offset);
        }
    }

    /// F55: the palette slots the poses hold.
    pub fn slot_counts(&self) -> PoseSlots {
        let held = |b: &BoundPose| b.parts.values().filter(|p| p.offset != 0).count();
        PoseSlots {
            poses: self.poses.len(),
            slots: self.poses.values().map(held).sum(),
            most: self.poses.values().map(held).max().unwrap_or(0),
            pending: self.freed.len(),
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
    animator.give_back(&mut library.palettes);
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
            looks: HashMap::from([(format!("{:?}", Look::plain([1.0; 4]).key()), 2)]),
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
        let look = Look::plain([1.0; 4]);
        animator.detach(&first, &look);
        assert!(animator.freed.is_empty(), "one still draws with it");
        assert!(animator.poses.contains_key(&second));
        animator.detach(&second, &look);
        assert_eq!(animator.freed, [50]);
        assert!(animator.poses.is_empty());
    }

    /// A walker: one bone, one skinned part; and a library set up for a map.
    fn walker() -> (
        ModelLibrary,
        LoadedModel,
        Assets<Image>,
        Assets<ObjectMaterial>,
        Assets<bevy::render::storage::ShaderBuffer>,
    ) {
        use crate::render::models3d::{scene_light, LoadedPart, ObjectParams};
        let mut buffers = Assets::default();
        let mut library = ModelLibrary::new(None, HashMap::new());
        let light = buffers.add(scene_light(Vec4::ZERO, Vec4::ONE, Vec4::ONE));
        let params = ObjectParams {
            tint: Vec4::ONE,
            world_rect: Vec4::ZERO,
            mode: Vec4::ZERO,
            extra: Vec4::ZERO,
            player: Vec4::ZERO,
        };
        library.set_map(Handle::default(), params, light, &mut buffers);
        let model = LoadedModel {
            parts: vec![LoadedPart {
                mesh: Handle::default(),
                texture: None,
                light: None,
                palette: vec![0],
                ground: false,
                decal: Vec::new(),
                piece: 0,
                group: 0,
                facet: None,
            }],
            skeleton: Some(Arc::new(Skeleton {
                name: "root".into(),
                names: vec!["root".into()],
                parents: vec![None],
                rest: vec![anim::Local::IDENTITY],
                inverse_world: vec![Mat4::IDENTITY],
            })),
            skeleton_file: "walker.gr2".into(),
            basis: Mat4::IDENTITY,
            basis_inverse: Mat4::IDENTITY,
            extent: [0.0; 3],
            bounds: [[0.0; 3]; 2],
            house: None,
        };
        (
            library,
            model,
            Assets::default(),
            Assets::default(),
            buffers,
        )
    }

    /// Hook W30d: a figure's pinned pose keeps its materials when its last
    /// figure leaves, so a walker moving from phase to phase makes no new
    /// material; a play-once clip is never pinned.
    // spec: none (hook W30d: the figures' shared poses are held for good)
    #[test]
    fn a_pinned_pose_keeps_its_materials() {
        let (mut library, model, mut images, mut materials, _buffers) = walker();
        let mut animator = Animator::default();
        let look = Look::plain([1.0; 4]);
        let want = PoseWant::new("walker.gr2", "c.glb#figure:Walk@3", true);
        animator.pin(
            &want,
            &model,
            &look,
            &mut library,
            &mut images,
            &mut materials,
        );
        let made = materials.len();
        for _ in 0..10 {
            let (pose, _) = animator.attach(
                &want,
                &model,
                &look,
                &mut library,
                &mut images,
                &mut materials,
            );
            animator.detach(&pose, &look);
        }
        assert_eq!(materials.len(), made, "no new material per visit");
        assert!(animator.freed.is_empty());
        // Pinned once: a second pin adds no user.
        animator.pin(
            &want,
            &model,
            &look,
            &mut library,
            &mut images,
            &mut materials,
        );
        let (pose, _) = animator.attach(
            &want,
            &model,
            &look,
            &mut library,
            &mut images,
            &mut materials,
        );
        assert_eq!(animator.pool.users(&pose), 2);
        // A play-once clip: nothing held.
        let once = PoseWant::new("walker.gr2", "c.glb#figure:Pickup", false);
        animator.pin(
            &once,
            &model,
            &look,
            &mut library,
            &mut images,
            &mut materials,
        );
        assert_eq!(animator.pinned.len(), 1);
    }

    /// A settler figure's look: its own atlas block (GFX-OBJ-3.2 (3)).
    fn figure(block: u32) -> Look {
        let mut look = Look::family(crate::render::models3d::Family::Settler, [1.0, 0.0, 0.0]);
        look.extra = [block as f32, 0.0, 0.0, 0.0];
        look
    }

    /// F55 (the long play): the walk of a busy map is one shared looping pose
    /// that never empties while figures, each with a look of its own, come
    /// and go. A look's part and palette slot go with its last user, so the
    /// slots held are those of the figures walking and the buffer stops
    /// growing (it kept one slot more for every figure that ever walked).
    // spec: GFX-ANIM-2.4, GFX-ANIM-8.2, GFX-OBJ-3.2
    #[test]
    fn a_pose_that_never_empties_gives_a_leaving_looks_slot_back() {
        let (mut library, model, mut images, mut materials, mut buffers) = walker();
        let mut animator = Animator::default();
        let want = PoseWant::new("walker.gr2", "walk.gr2", true);
        let mut walking = std::collections::VecDeque::new();
        let mut steady = None;
        for block in 0..3_000u32 {
            let (pose, _) = animator.attach(
                &want,
                &model,
                &figure(block),
                &mut library,
                &mut images,
                &mut materials,
            );
            walking.push_back((pose, block));
            if walking.len() > 100 {
                let (pose, block) = walking.pop_front().expect("walking");
                animator.detach(&pose, &figure(block));
            }
            // A frame every few figures.
            if block % 5 == 4 {
                animator.give_back(&mut library.palettes);
                library.palettes.flush(&mut buffers);
            }
            if block == 1_000 {
                steady = Some(library.palettes.capacity());
            }
        }
        let held = animator.slot_counts();
        assert_eq!(held.poses, 1, "one pose, never empty");
        assert_eq!(held.slots, 100, "a slot per figure walking");
        let (taken, _) = library.palettes.slots();
        assert!(taken <= 101 + 5, "{taken} slots taken");
        assert_eq!(
            Some(library.palettes.capacity()),
            steady,
            "the buffer stops growing"
        );
    }

    /// F55: two figures drawn with the same look share its part: the slot
    /// stays while one still walks and goes with the second.
    // spec: GFX-ANIM-2.4, GFX-ANIM-8.2
    #[test]
    fn a_shared_looks_slot_stays_while_one_draws_with_it() {
        let (mut library, model, mut images, mut materials, _buffers) = walker();
        let mut animator = Animator::default();
        let want = PoseWant::new("walker.gr2", "walk.gr2", true);
        let (anchor, _) = animator.attach(
            &want,
            &model,
            &figure(0),
            &mut library,
            &mut images,
            &mut materials,
        );
        let mut attach = |animator: &mut Animator, look: &Look| {
            animator
                .attach(
                    &want,
                    &model,
                    look,
                    &mut library,
                    &mut images,
                    &mut materials,
                )
                .0
        };
        let a = attach(&mut animator, &figure(7));
        let b = attach(&mut animator, &figure(7));
        assert_eq!((&a, &b), (&anchor, &anchor), "all in the one pose");
        assert_eq!(animator.slot_counts().slots, 2, "figure 0's and figure 7's");
        animator.detach(&a, &figure(7));
        assert!(animator.freed.is_empty(), "figure 7's look still walks");
        animator.detach(&b, &figure(7));
        assert_eq!(animator.freed.len(), 1);
        assert_eq!(animator.slot_counts().slots, 1);
        animator.detach(&anchor, &figure(0));
        assert_eq!(animator.freed.len(), 2);
        assert_eq!(animator.slot_counts().poses, 0);
    }
}
