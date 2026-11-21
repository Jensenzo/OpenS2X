//! Skeletal animation (`specs/GFX-animations.md` GFX-ANIM-2, 8): Granny
//! animation curves sampled into bone poses, the per-mesh bone palette
//! with four weights per vertex, and the pool of poses that settlers of one
//! model playing one clip share. Plain math on the decoded files; the Bevy
//! side uploads the matrices.
//!
//! Matrices are column-vector (`p' = M p`). Everything here is in the
//! file's own axes; [`conjugate`] carries a composite matrix into the
//! frame the meshes were converted to (GFX-VIEW-1.2).

use std::collections::HashMap;

use bevy::math::{Mat3, Mat4, Quat, Vec3};
use s2_assets::gr2;

/// GFX-ANIM-2.1: animation seconds per real second (`graphics.xml`
/// Settings/Animations `globalSpeed`).
pub const GLOBAL_SPEED: f32 = 1.8;

/// GFX-ANIM-8.3: matrices in the shader's palette (MAX_BONES).
pub const MAX_BONES: usize = 50;

/// A bone's transform relative to its parent: position, orientation and
/// scale/shear (GFX-GR2-3.2 type 9).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Local {
    pub position: Vec3,
    pub orientation: Quat,
    pub scale_shear: Mat3,
}

impl Local {
    #[cfg(test)]
    pub const IDENTITY: Self = Self {
        position: Vec3::ZERO,
        orientation: Quat::IDENTITY,
        scale_shear: Mat3::IDENTITY,
    };

    fn from_gr2(t: &gr2::Transform) -> Self {
        let [x, y, z, w] = t.orientation;
        Self {
            position: Vec3::from_array(t.position),
            orientation: normalized(Quat::from_xyzw(x, y, z, w)),
            scale_shear: row_major_3(&t.scale_shear),
        }
    }

    /// Position, then orientation, then scale/shear: `T R S`.
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_translation(self.position)
            * Mat4::from_quat(self.orientation)
            * Mat4::from_mat3(self.scale_shear)
    }
}

fn normalized(q: Quat) -> Quat {
    let len = q.length();
    if len > 1e-12 {
        q / len
    } else {
        Quat::IDENTITY
    }
}

/// A 3 x 3 matrix stored by rows.
fn row_major_3(m: &[f32; 9]) -> Mat3 {
    Mat3::from_cols_array(m).transpose()
}

/// A skeleton ready for posing: parents come before their children in
/// Granny's bone order.
#[derive(Debug, Clone, PartialEq)]
pub struct Skeleton {
    /// The skeleton's name: for the game's building models the model's
    /// name, `root` (GFX-ANIM-6.1a).
    pub name: String,
    pub names: Vec<String>,
    pub parents: Vec<Option<usize>>,
    pub rest: Vec<Local>,
    /// The inverse of each bone's rest world matrix (GFX-GR2-4.3
    /// `InverseWorldTransform`, stored for row vectors, which read by
    /// columns is the column-vector matrix).
    pub inverse_world: Vec<Mat4>,
}

impl Skeleton {
    pub fn from_gr2(s: &gr2::Skeleton) -> Self {
        Self {
            name: s.name.clone(),
            names: s.bones.iter().map(|b| b.name.clone()).collect(),
            parents: s.bones.iter().map(|b| b.parent).collect(),
            rest: s
                .bones
                .iter()
                .map(|b| Local::from_gr2(&b.transform))
                .collect(),
            inverse_world: s
                .bones
                .iter()
                .map(|b| Mat4::from_cols_array(&b.inverse_world))
                .collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn find(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }

    /// World matrices from the bones' local transforms.
    pub fn world(&self, locals: &[Local]) -> Vec<Mat4> {
        let mut world: Vec<Mat4> = Vec::with_capacity(self.len());
        for (i, local) in locals.iter().enumerate() {
            let m = local.matrix();
            let w = match self.parents[i] {
                Some(p) if p < i => world[p] * m,
                _ => m,
            };
            world.push(w);
        }
        world
    }

    /// GFX-ANIM-8.2: the composite matrices (world times inverse rest
    /// world) that move a bind-pose vertex with its bone.
    pub fn composites(&self, world: &[Mat4]) -> Vec<Mat4> {
        world
            .iter()
            .zip(&self.inverse_world)
            .map(|(w, inv)| *w * *inv)
            .collect()
    }
}

pub use s2_assets::gr2::curve::sample;

/// The three curves of one bone.
#[derive(Debug, Clone, PartialEq)]
struct Track {
    position: gr2::Curve,
    orientation: gr2::Curve,
    scale_shear: gr2::Curve,
}

/// An animation bound to a skeleton: each bone's track, found by name.
/// Bones without a track keep their rest transform.
#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    /// Animation seconds.
    pub duration: f32,
    tracks: Vec<Option<Track>>,
}

impl Clip {
    /// The first animation of a file bound to `skeleton`: its track group
    /// named as the skeleton, else its first (GFX-ANIM-6.1a: the Roman
    /// huts' door clips hold a `GROUND` group before their `root` one).
    pub fn bind(file: &gr2::Gr2, skeleton: &Skeleton) -> Option<Self> {
        let animation = file.animations.first()?;
        let group = animation
            .track_groups
            .iter()
            .find(|g| g.name == skeleton.name)
            .or(animation.track_groups.first())?;
        let mut tracks = vec![None; skeleton.len()];
        for t in &group.transform_tracks {
            if let Some(bone) = skeleton.find(&t.name) {
                tracks[bone] = Some(Track {
                    position: t.position.clone(),
                    orientation: t.orientation.clone(),
                    scale_shear: t.scale_shear.clone(),
                });
            }
        }
        Some(Self {
            duration: animation.duration,
            tracks,
        })
    }

    /// Hook W30: a clip of a project's model (`models3d::project`) from
    /// each bone's position, orientation and scale/shear curves (laid out
    /// as Granny's; an empty curve keeps the rest value), `None` for a bone
    /// it does not move.
    pub fn from_curves(duration: f32, tracks: Vec<Option<[gr2::Curve; 3]>>) -> Self {
        Self {
            duration,
            tracks: tracks
                .into_iter()
                .map(|t| {
                    t.map(|[position, orientation, scale_shear]| Track {
                        position,
                        orientation,
                        scale_shear,
                    })
                })
                .collect(),
        }
    }

    /// The bones' local transforms at animation time `t`; `looping`: the
    /// clip can still wrap (GFX-GR2-7.6), so its curves' windows wrap.
    pub fn locals(&self, skeleton: &Skeleton, t: f32, looping: bool) -> Vec<Local> {
        skeleton
            .rest
            .iter()
            .zip(&self.tracks)
            .map(|(rest, track)| {
                let Some(track) = track else { return *rest };
                let p = sample(&track.position, t, rest.position.to_array(), looping);
                let q = sample(&track.orientation, t, rest.orientation.to_array(), looping);
                let s = sample(
                    &track.scale_shear,
                    t,
                    rest.scale_shear.transpose().to_cols_array(),
                    looping,
                );
                Local {
                    position: Vec3::from_array(p),
                    orientation: normalized(Quat::from_array(q)),
                    scale_shear: row_major_3(&s),
                }
            })
            .collect()
    }

    /// The clip's tracks laid over `locals` at animation time `t`: bones
    /// it has a track for take the track's transform, the others keep
    /// theirs (several clips of one building play at once, its door and its
    /// working machinery).
    pub fn apply(&self, skeleton: &Skeleton, locals: &mut [Local], t: f32, looping: bool) {
        let sampled = self.locals(skeleton, t, looping);
        for ((local, track), s) in locals.iter_mut().zip(&self.tracks).zip(sampled) {
            if track.is_some() {
                *local = s;
            }
        }
    }

    /// GFX-ANIM-2.2: the time a clip shows after `elapsed` animation
    /// seconds: wrapped when it loops, held at the end when it plays once.
    pub fn time_at(&self, elapsed: f32, looping: bool) -> f32 {
        if self.duration <= 0.0 {
            return 0.0;
        }
        if looping {
            elapsed.rem_euclid(self.duration)
        } else {
            elapsed.clamp(0.0, self.duration)
        }
    }

    /// The composite matrices after `elapsed` animation seconds.
    #[cfg(test)]
    pub fn composites(&self, skeleton: &Skeleton, elapsed: f32, looping: bool) -> Vec<Mat4> {
        let locals = self.locals(skeleton, self.time_at(elapsed, looping), looping);
        skeleton.composites(&skeleton.world(&locals))
    }
}

/// The composites of the rest pose. They are not the identity when a
/// mesh was bound in another pose (the settlers', GFX-OBJ-1.8).
pub fn rest_composites(skeleton: &Skeleton) -> Vec<Mat4> {
    skeleton.composites(&skeleton.world(&skeleton.rest))
}

/// GFX-ANIM-8.1, 8.2: a mesh's bone palette (the skeleton bones its
/// vertices use, in skeleton order) and each vertex's four palette indices
/// and weights.
#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    pub bones: Vec<usize>,
    pub indices: Vec<[u32; 4]>,
    pub weights: Vec<[f32; 4]>,
}

impl Palette {
    /// GFX-ANIM-8.3: the palette fits the shader's matrix array.
    pub fn fits(&self) -> bool {
        self.bones.len() <= MAX_BONES
    }
}

/// The palette of `mesh` on `skeleton`. A vertex's stored bone indices
/// point into the mesh's bone bindings, which name skeleton bones. Weights
/// are the three stored bytes / 255 and their rest to 1 (GFX-ANIM-8.1);
/// an index whose weight is not above 0 becomes 0. A mesh without weights
/// but with bindings follows its first binding rigidly. `None` when the
/// mesh has no bindings or names a bone the skeleton lacks.
pub fn palette(mesh: &gr2::Mesh, skeleton: &Skeleton) -> Option<Palette> {
    if mesh.bone_bindings.is_empty() {
        return None;
    }
    let bound: Vec<usize> = mesh
        .bone_bindings
        .iter()
        .map(|b| skeleton.find(&b.bone))
        .collect::<Option<_>>()?;
    let v = &mesh.vertices;
    let (stored_indices, weights): (Vec<[u8; 4]>, Vec<[f32; 4]>) =
        if v.bone_weights.len() == v.count && v.bone_indices.len() == v.count {
            (v.bone_indices.clone(), v.skin_weights())
        } else {
            (vec![[0; 4]; v.count], vec![[1.0, 0.0, 0.0, 0.0]; v.count])
        };
    // Skeleton bone of each vertex slot, 0 where the weight is not above 0.
    let skeleton_bones: Vec<[Option<usize>; 4]> = stored_indices
        .iter()
        .zip(&weights)
        .map(|(idx, w)| {
            std::array::from_fn(|k| {
                let binding = if w[k] > 0.0 { usize::from(idx[k]) } else { 0 };
                bound.get(binding).copied()
            })
        })
        .collect();
    let mut used = vec![false; skeleton.len()];
    for (bones, w) in skeleton_bones.iter().zip(&weights) {
        for k in 0..4 {
            if let (Some(b), true) = (bones[k], w[k] > 0.0) {
                used[b] = true;
            }
        }
    }
    let bones: Vec<usize> = (0..skeleton.len()).filter(|&b| used[b]).collect();
    let slot = |b: Option<usize>| -> u32 {
        b.and_then(|b| bones.iter().position(|&p| p == b))
            .map_or(0, |p| p as u32)
    };
    let indices = skeleton_bones
        .iter()
        .zip(&weights)
        .map(|(b, w)| std::array::from_fn(|k| if w[k] > 0.0 { slot(b[k]) } else { 0 }))
        .collect();
    Some(Palette {
        bones,
        indices,
        weights,
    })
}

/// A composite matrix in the file's axes carried into the frame the
/// meshes were converted to by `basis` (`p_new = basis p_file`).
pub fn conjugate(basis: Mat4, basis_inverse: Mat4, composite: Mat4) -> Mat4 {
    basis * composite * basis_inverse
}

/// What a mover asks the pool for: a clip of a model file's skeleton,
/// whether it loops (GFX-ANIM-2.2), and whether its pose must be its own
/// (a unique clip; play-once clips always are, GFX-ANIM-2.4 b).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PoseWant {
    pub model: String,
    pub clip: String,
    pub looping: bool,
    pub private: bool,
}

impl PoseWant {
    pub fn new(model: &str, clip: &str, looping: bool) -> Self {
        Self {
            model: model.into(),
            clip: clip.into(),
            looping,
            private: false,
        }
    }

    pub fn private(mut self) -> Self {
        self.private = true;
        self
    }
}

/// A pose of the pool as one user holds it: the model file and clip, the
/// pool's pose (`instance`) and which start of it (`run`: a restart at
/// time 0 is a new run, a join keeps the run).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PoseKey {
    pub model: String,
    pub clip: String,
    pub looping: bool,
    pub private: bool,
    pub instance: u32,
    pub run: u32,
}

impl PoseKey {
    /// The key holds what `want` asks for (GFX-ANIM-3.2: no change of pose).
    pub fn holds(&self, want: &PoseWant) -> bool {
        self.model == want.model
            && self.clip == want.clip
            && self.looping == want.looping
            && self.private == want.private
    }
}

#[derive(Debug, Clone, PartialEq)]
struct PoseState {
    /// The clip playing, while the pose has users.
    clip: Option<String>,
    looping: bool,
    private: bool,
    run: u32,
    /// Animation-clock seconds since the pose started (TOOL-1.2: game time).
    clock: f32,
    users: u32,
}

/// GFX-ANIM-2.4 (d): the sweep's period and the pool size it keeps.
pub const SWEEP_SECONDS: f32 = 10.0;
pub const SWEEP_KEEP: usize = 5;

/// GFX-ANIM-2.4, 2.5: one pool of poses per model file, in the order they
/// were made. A looping clip joins the first pose playing it (or restarts
/// the first pose without a user, whichever comes first); a play-once clip
/// always gets a pose of its own at time 0. A pose whose last user leaves
/// stops and stays for reuse; every 10 s, while a pool holds more than 5
/// poses, poses without a user are freed.
#[derive(Debug, Clone, Default)]
pub struct PosePool {
    poses: HashMap<u32, PoseState>,
    order: HashMap<String, Vec<u32>>,
    next_instance: u32,
    next_run: u32,
    sweep_clock: f32,
}

impl PosePool {
    /// Take a pose for one user (GFX-ANIM-2.4 a, b).
    pub fn acquire(&mut self, want: &PoseWant) -> PoseKey {
        let order = self.order.entry(want.model.clone()).or_default();
        let shared = want.looping && !want.private;
        let found = order.iter().copied().find(|id| {
            let p = &self.poses[id];
            p.users == 0
                || (shared
                    && p.looping
                    && !p.private
                    && p.clip.as_deref() == Some(want.clip.as_str()))
        });
        let id = found.unwrap_or_else(|| {
            self.next_instance += 1;
            let id = self.next_instance;
            order.push(id);
            self.poses.insert(
                id,
                PoseState {
                    clip: None,
                    looping: false,
                    private: false,
                    run: 0,
                    clock: 0.0,
                    users: 0,
                },
            );
            id
        });
        let pose = self.poses.get_mut(&id).expect("in the pool");
        if pose.users == 0 {
            self.next_run += 1;
            *pose = PoseState {
                clip: Some(want.clip.clone()),
                looping: want.looping,
                private: want.private || !want.looping,
                run: self.next_run,
                clock: 0.0,
                users: 0,
            };
        }
        pose.users += 1;
        PoseKey {
            model: want.model.clone(),
            clip: want.clip.clone(),
            looping: want.looping,
            private: want.private,
            instance: id,
            run: pose.run,
        }
    }

    fn get(&self, key: &PoseKey) -> Option<&PoseState> {
        self.poses
            .get(&key.instance)
            .filter(|p| p.users > 0 && p.run == key.run)
    }

    /// GFX-ANIM-2.4 (c): one user leaves; the last one stops the pose,
    /// which stays in the pool without a clip.
    pub fn release(&mut self, key: &PoseKey) {
        if self.get(key).is_none() {
            return;
        }
        let p = self.poses.get_mut(&key.instance).expect("checked");
        p.users -= 1;
        if p.users == 0 {
            p.clip = None;
        }
    }

    /// Advance every pose and the sweep by `dt` seconds.
    #[cfg(test)]
    pub fn advance(&mut self, dt: f32) {
        self.advance_poses(dt);
        self.advance_sweep(dt);
    }

    /// Advance every pose by `dt` seconds of the animation clock
    /// (TOOL-1.2: the game time step; GFX-ANIM-5.1: an animal's run clip
    /// too, nothing adapts it to the walk, Q-200 answered).
    pub fn advance_poses(&mut self, dt: f32) {
        for p in self.poses.values_mut().filter(|p| p.users > 0) {
            p.clock += dt;
        }
    }

    /// GFX-ANIM-2.4 d: sweep every 10 s (`dt` game seconds, the game step:
    /// TOOL-trace TRACE-5.3).
    pub fn advance_sweep(&mut self, dt: f32) {
        self.sweep_clock += dt;
        while self.sweep_clock >= SWEEP_SECONDS {
            self.sweep_clock -= SWEEP_SECONDS;
            self.sweep();
        }
    }

    /// GFX-ANIM-2.4 (d): free poses without a user while the pool holds
    /// more than 5; freeing one moves the last pose into its place.
    fn sweep(&mut self) {
        for order in self.order.values_mut() {
            let mut i = 0;
            while i < order.len() && order.len() > SWEEP_KEEP {
                if self.poses[&order[i]].users == 0 {
                    self.poses.remove(&order[i]);
                    order.swap_remove(i);
                } else {
                    i += 1;
                }
            }
        }
    }

    /// GFX-ANIM-2.1: the pose's animation time (game seconds x global
    /// speed), `None` for a key no longer held.
    pub fn elapsed(&self, key: &PoseKey) -> Option<f32> {
        self.get(key).map(|p| p.clock * GLOBAL_SPEED)
    }

    pub fn users(&self, key: &PoseKey) -> u32 {
        self.get(key).map_or(0, |p| p.users)
    }

    /// The poses of a model file's pool, in pool order: (instance, clip,
    /// users).
    #[cfg(test)]
    pub fn pool_of(&self, model: &str) -> Vec<(u32, Option<String>, u32)> {
        self.order.get(model).map_or(Vec::new(), |o| {
            o.iter()
                .map(|id| (*id, self.poses[id].clip.clone(), self.poses[id].users))
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn curve(degree: u32, knots: &[f32], controls: &[f32]) -> gr2::Curve {
        gr2::Curve {
            degree,
            knots: knots.to_vec(),
            controls: controls.to_vec(),
        }
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn chain() -> Skeleton {
        // Root at (0, 0, 1); a child 2 along x, turned 90 degrees about z.
        let rest = vec![
            Local {
                position: Vec3::new(0.0, 0.0, 1.0),
                ..Local::IDENTITY
            },
            Local {
                position: Vec3::new(2.0, 0.0, 0.0),
                orientation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                ..Local::IDENTITY
            },
        ];
        let mut s = Skeleton {
            name: "root".into(),
            names: vec!["root".into(), "arm".into()],
            parents: vec![None, Some(0)],
            rest,
            inverse_world: vec![Mat4::IDENTITY; 2],
        };
        let world = s.world(&s.rest);
        s.inverse_world = world.iter().map(|w| w.inverse()).collect();
        s
    }

    // spec: GFX-ANIM-8.1
    #[test]
    fn rest_pose_composites_are_identity() {
        let s = chain();
        for c in rest_composites(&s) {
            assert!(c.abs_diff_eq(Mat4::IDENTITY, 1e-5));
        }
        let world = s.world(&s.rest);
        assert!(world[1]
            .transform_point3(Vec3::ZERO)
            .abs_diff_eq(Vec3::new(2.0, 0.0, 1.0), 1e-5));
    }

    fn file_with_track(name: &str, ori: gr2::Curve, duration: f32) -> gr2::Gr2 {
        gr2::Gr2 {
            animations: vec![gr2::Animation {
                name: "test.max".into(),
                duration,
                time_step: 1.0 / 24.0,
                track_groups: vec![gr2::TrackGroup {
                    name: "g".into(),
                    transform_tracks: vec![gr2::TransformTrack {
                        name: name.into(),
                        position: gr2::Curve::default(),
                        orientation: ori,
                        scale_shear: gr2::Curve::default(),
                    }],
                }],
            }],
            ..gr2::Gr2::default()
        }
    }

    /// A known frame: the root turns from 0 to 90 degrees about z over one
    /// second; at t = 1 the arm's bind-pose tip (2, 0, 1) has swung to
    /// (0, 2, 1); half way it stands at 45 degrees.
    // spec: GFX-ANIM-8.1, GFX-GR2-7.5
    #[test]
    fn clip_moves_a_vertex_with_its_bone() {
        let s = chain();
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let ori = curve(1, &[0.0, 1.0], &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, h, h]);
        let clip = Clip::bind(&file_with_track("root", ori, 1.0), &s).unwrap();
        let tip = Vec3::new(2.0, 0.0, 1.0);
        let at = |t: f32| clip.composites(&s, t, false)[1].transform_point3(tip);
        assert!(at(0.0).abs_diff_eq(tip, 1e-4));
        assert!(at(1.0).abs_diff_eq(Vec3::new(0.0, 2.0, 1.0), 1e-4));
        let half = at(0.5);
        assert!(close(half.x, half.y) && close(half.length_squared(), 5.0));
        // Played once it holds the end; looping, 1.25 is 0.25.
        assert!(at(3.0).abs_diff_eq(Vec3::new(0.0, 2.0, 1.0), 1e-4));
        assert!(close(clip.time_at(1.25, true), 0.25));
        assert!(close(clip.time_at(1.25, false), 1.0));
    }

    /// GFX-ANIM-6.1a, T21: a clip with a `GROUND` group first and the
    /// `root` group second plays the `root` group on a skeleton named
    /// `root` (the Roman huts' doors); a file with one group plays it.
    // spec: GFX-ANIM-6.1a (T21)
    #[test]
    fn the_group_named_as_the_skeleton_plays() {
        let s = chain();
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let ori = || curve(1, &[0.0, 1.0], &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, h, h]);
        let mut file = file_with_track("root", ori(), 1.0);
        file.animations[0].track_groups[0].name = "root".into();
        file.animations[0].track_groups.insert(
            0,
            gr2::TrackGroup {
                name: "GROUND".into(),
                transform_tracks: vec![gr2::TransformTrack {
                    name: "GROUND".into(),
                    position: gr2::Curve::default(),
                    orientation: ori(),
                    scale_shear: gr2::Curve::default(),
                }],
            },
        );
        let tip = Vec3::new(2.0, 0.0, 1.0);
        let clip = Clip::bind(&file, &s).unwrap();
        let end = clip.composites(&s, 1.0, false)[1].transform_point3(tip);
        assert!(end.abs_diff_eq(Vec3::new(0.0, 2.0, 1.0), 1e-4), "{end}");
        // One group of another name: still played.
        let clip = Clip::bind(&file_with_track("root", ori(), 1.0), &s).unwrap();
        let end = clip.composites(&s, 1.0, false)[1].transform_point3(tip);
        assert!(end.abs_diff_eq(Vec3::new(0.0, 2.0, 1.0), 1e-4), "{end}");
    }

    /// GFX-ANIM-6.1a, T21 on the install: the doors of the Roman
    /// woodcutter's hut, forester's hut and stone pit (a `GROUND` group
    /// first) and of the sawmill (one group) turn their `door` bone over
    /// `door_open`. Skipped without `S2_GAME_DIR`.
    // spec: GFX-ANIM-6.1a (T21)
    #[test]
    fn install_hut_doors_open() {
        let Ok(root) = std::env::var("S2_GAME_DIR") else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let read = |file: &str| {
            let path = std::path::Path::new(&root)
                .join("data/buildings")
                .join(file);
            let data = std::fs::read(&path).expect("the file");
            gr2::load(&path.to_string_lossy(), &data).expect("a GR2 file")
        };
        for model in [
            "PB_R_WoodcutterHut01",
            "PB_R_ForesterHut01",
            "PB_R_Stonemason01",
            "PB_R_Sawmill01",
        ] {
            let skeleton = Skeleton::from_gr2(&read(&format!("{model}.GR2")).skeletons[0]);
            let clip = Clip::bind(&read(&format!("{model}/door_open.GR2")), &skeleton).unwrap();
            let door = skeleton.names.iter().position(|n| n == "door").unwrap();
            let open = clip.locals(&skeleton, clip.duration, false)[door];
            let shut = skeleton.rest[door];
            assert!(
                open.orientation.angle_between(shut.orientation) > 0.1
                    || open.position.distance(shut.position) > 0.01,
                "{model}: the door does not move"
            );
        }
    }

    fn mesh(indices: Vec<[u8; 4]>, weights: Vec<[u8; 4]>, bindings: &[&str]) -> gr2::Mesh {
        gr2::Mesh {
            name: "m".into(),
            vertices: gr2::Vertices {
                count: indices.len(),
                positions: vec![[0.0; 3]; indices.len()],
                bone_indices: indices,
                bone_weights: weights,
                ..gr2::Vertices::default()
            },
            indices: Vec::new(),
            groups: Vec::new(),
            materials: Vec::new(),
            bone_bindings: bindings
                .iter()
                .map(|b| gr2::BoneBinding {
                    bone: (*b).into(),
                    min: [0.0; 3],
                    max: [0.0; 3],
                    triangles: Vec::new(),
                })
                .collect(),
        }
    }

    /// GFX-ANIM T9: bones {0, 3, 7} referenced -> 3 palette entries, vertex
    /// indices 0, 1, 2.
    // spec: GFX-ANIM-8.2
    #[test]
    fn palette_of_t9() {
        let names: Vec<String> = (0..8).map(|i| format!("b{i}")).collect();
        let s = Skeleton {
            name: "root".into(),
            parents: vec![None; 8],
            rest: vec![Local::IDENTITY; 8],
            inverse_world: vec![Mat4::IDENTITY; 8],
            names,
        };
        // Bindings in another order than the skeleton's; b5 bound, unused.
        let m = mesh(
            vec![[0, 0, 0, 0], [1, 0, 0, 0], [2, 1, 0, 0]],
            vec![[255, 0, 0, 0], [255, 0, 0, 0], [128, 127, 0, 0]],
            &["b7", "b3", "b0", "b5"],
        );
        let p = palette(&m, &s).unwrap();
        assert_eq!(p.bones, [0, 3, 7]);
        assert_eq!(p.indices, [[2, 0, 0, 0], [1, 0, 0, 0], [0, 1, 0, 0]]);
        assert!(close(p.weights[2][0] + p.weights[2][1], 1.0));
        assert!(p.fits());
    }

    /// GFX-ANIM-8.1: the fourth weight is what the three stored ones leave.
    #[test]
    fn fourth_weight_is_derived() {
        let names: Vec<String> = (0..4).map(|i| format!("b{i}")).collect();
        let s = Skeleton {
            name: "root".into(),
            parents: vec![None; 4],
            rest: vec![Local::IDENTITY; 4],
            inverse_world: vec![Mat4::IDENTITY; 4],
            names,
        };
        let m = mesh(
            vec![[0, 1, 2, 3]],
            vec![[51, 51, 51, 0]],
            &["b0", "b1", "b2", "b3"],
        );
        let p = palette(&m, &s).unwrap();
        assert!(close(p.weights[0][3], 0.4));
        assert_eq!(p.indices[0], [0, 1, 2, 3]);
        assert_eq!(p.bones, [0, 1, 2, 3]);
    }

    /// Two clips over one skeleton: each sets only its own bones.
    // spec: GFX-GR2-7.6
    #[test]
    fn clips_layer_by_bone() {
        let s = chain();
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let turn = curve(0, &[0.0], &[0.0, 0.0, h, h]);
        let root = Clip::bind(&file_with_track("root", turn.clone(), 1.0), &s).unwrap();
        let arm = Clip::bind(&file_with_track("arm", turn, 1.0), &s).unwrap();
        let mut locals = s.rest.clone();
        root.apply(&s, &mut locals, 0.0, false);
        assert_eq!(locals[1], s.rest[1]);
        arm.apply(&s, &mut locals, 0.0, false);
        assert!(locals[0]
            .orientation
            .abs_diff_eq(Quat::from_xyzw(0.0, 0.0, h, h), 1e-6));
        assert!(locals[1]
            .orientation
            .abs_diff_eq(Quat::from_xyzw(0.0, 0.0, h, h), 1e-6));
    }

    /// GFX-ANIM-2.1, 5.1 (Q-200 answered): every pose, an animal's run
    /// clip too, runs at globalSpeed x the game step; a private looping
    /// pose is never joined.
    // spec: GFX-ANIM-2.1, GFX-ANIM-5.1
    #[test]
    fn pose_rate() {
        let mut pool = PosePool::default();
        let own = pool.acquire(&PoseWant::new("deer", "run", true).private());
        let plain = pool.acquire(&PoseWant::new("deer", "idle", true));
        pool.advance(1.0);
        assert!(close(pool.elapsed(&own).unwrap(), 1.8));
        assert!(close(pool.elapsed(&plain).unwrap(), 1.8));
        let other = pool.acquire(&PoseWant::new("deer", "run", true));
        assert_ne!(other.instance, own.instance);
    }

    // TOOL-1.2: a pose's clock takes the game time step: 40 frames of
    // 0.025 s give 1.8 s of clip time at speed 1, 3.6 at 2, 5.4 at 3, and
    // nothing while paused.
    #[test]
    fn poses_run_on_game_time() {
        for (speed, paused, want) in [
            (1, false, 1.8),
            (2, false, 3.6),
            (3, false, 5.4),
            (3, true, 0.0),
        ] {
            let mut pool = PosePool::default();
            let walk = pool.acquire(&PoseWant::new("carrier", "walk", true));
            for _ in 0..40 {
                pool.advance_poses(crate::game::game_step(0.025, speed, paused));
            }
            assert!(
                (pool.elapsed(&walk).unwrap() - want).abs() < 1e-4,
                "{speed} {paused}"
            );
        }
    }

    const HAMMER: &str = "constructor_hammer";

    fn hammer() -> PoseWant {
        PoseWant::new("constructor", HAMMER, false)
    }

    fn stand() -> PoseWant {
        PoseWant::new("constructor", "stand", true)
    }

    /// GFX-ANIM-3.8f, T19a: a builder's next work step leaves the hammer
    /// pose and takes one again, at time 0.
    #[test]
    fn builder_step_restarts_the_hammer_pose() {
        let mut pool = PosePool::default();
        let a = pool.acquire(&hammer());
        pool.advance(5.5);
        pool.release(&a);
        let a2 = pool.acquire(&hammer());
        assert_eq!(pool.elapsed(&a2), Some(0.0));
        // A new run: the old key is no longer held.
        assert_eq!(pool.elapsed(&a), None);
    }

    /// GFX-ANIM-2.4 (b), 3.8f (b), T19b: builders never share a hammer
    /// pose. A at 0, B 20 ticks (2.0 s) later, C when A's step ends: each
    /// starts at clip time 0 and runs in his own phase.
    #[test]
    fn t19b_builders_hammer_in_their_own_phase() {
        let tick = 0.1;
        let mut pool = PosePool::default();
        let a = pool.acquire(&hammer());
        pool.advance(20.0 * tick);
        let b = pool.acquire(&hammer());
        assert_ne!(a.instance, b.instance);
        assert_eq!(pool.elapsed(&b), Some(0.0));
        assert!(close(pool.elapsed(&a).unwrap(), 20.0 * tick * GLOBAL_SPEED));
        pool.advance(34.0 * tick);
        let c = pool.acquire(&hammer());
        assert_eq!(pool.elapsed(&c), Some(0.0));
        assert!(close(pool.elapsed(&b).unwrap(), 34.0 * tick * GLOBAL_SPEED));
        for k in [&a, &b, &c] {
            assert_eq!(pool.users(k), 1);
        }
    }

    /// GFX-ANIM-2.4 T12: A ends his chop, B starts chopping 2 ticks later:
    /// B's clip starts at time 0 in a pose of his own, here A's old one
    /// restarted.
    #[test]
    fn t12_chop_starts_at_zero() {
        let chop = PoseWant::new("settler", "workloop0axe", false);
        let mut pool = PosePool::default();
        let a = pool.acquire(&chop);
        pool.advance(4.0);
        pool.release(&a);
        pool.advance(0.2);
        let b = pool.acquire(&chop);
        assert_eq!(b.instance, a.instance);
        assert_eq!(pool.elapsed(&b), Some(0.0));
        // While B chops, a third chopper gets a pose of his own at 0.
        pool.advance(1.0);
        let c = pool.acquire(&chop);
        assert_ne!(c.instance, b.instance);
        assert_eq!(pool.elapsed(&c), Some(0.0));
    }

    /// T19c: pool P1 (Stand, D), P2 (hammer, A), P3 (hammer, B). A's step
    /// ends and A takes Stand, joining D's P1; then C starts a step and
    /// gets P2 restarted at 0. Handled the other way round, C gets a new P4.
    // spec: GFX-ANIM-2.4 (T19c)
    #[test]
    fn t19c_pool_order() {
        let setup = || {
            let mut pool = PosePool::default();
            let d = pool.acquire(&stand());
            let a = pool.acquire(&hammer());
            let b = pool.acquire(&hammer());
            pool.advance(3.0);
            (pool, d, a, b)
        };
        let (mut pool, d, a, b) = setup();
        pool.release(&a);
        let a_stand = pool.acquire(&stand());
        assert_eq!(a_stand, d);
        assert!(close(pool.elapsed(&a_stand).unwrap(), 3.0 * GLOBAL_SPEED));
        let c = pool.acquire(&hammer());
        assert_eq!(c.instance, a.instance);
        assert_eq!(pool.elapsed(&c), Some(0.0));
        assert_eq!(pool.pool_of("constructor").len(), 3);
        let _ = b;
        // C before A: P2 is still A's, so C makes P4.
        let (mut pool, _, a, _) = setup();
        let c = pool.acquire(&hammer());
        assert_eq!(pool.pool_of("constructor").len(), 4);
        pool.release(&a);
        let a_stand = pool.acquire(&stand());
        assert_eq!(pool.users(&a_stand), 2);
        assert_ne!(c.instance, a.instance);
    }

    /// T19d: Y starts standing while X has stood for 3 s: Y joins X's pose
    /// in phase, unless a pose without a user comes first in the pool, which
    /// Y restarts at 0.
    // spec: GFX-ANIM-2.4 (T19d)
    #[test]
    fn t19d_stand_joins_or_restarts_an_earlier_free_pose() {
        let mut pool = PosePool::default();
        let x = pool.acquire(&stand());
        pool.advance(3.0);
        let y = pool.acquire(&stand());
        assert_eq!(x, y);
        assert_eq!(pool.users(&y), 2);
        assert!(close(pool.elapsed(&y).unwrap(), 3.0 * GLOBAL_SPEED));

        let mut pool = PosePool::default();
        let early = pool.acquire(&hammer());
        let x = pool.acquire(&stand());
        pool.advance(3.0);
        pool.release(&early);
        let y = pool.acquire(&stand());
        assert_eq!(y.instance, early.instance);
        assert_ne!(y, x);
        assert_eq!(pool.elapsed(&y), Some(0.0));
    }

    /// GFX-ANIM-2.4 (c), (d): a stopped pose stays for any clip; the 10 s
    /// sweep frees poses without a user only while more than 5 exist.
    #[test]
    fn stopped_poses_stay_and_the_sweep_keeps_five() {
        let mut pool = PosePool::default();
        let keys: Vec<_> = (0..8).map(|_| pool.acquire(&hammer())).collect();
        for k in &keys[..6] {
            pool.release(k);
        }
        pool.advance(9.0);
        assert_eq!(pool.pool_of("constructor").len(), 8);
        pool.advance(1.0);
        let left = pool.pool_of("constructor");
        assert_eq!(left.len(), 5);
        assert_eq!(left.iter().filter(|(_, _, u)| *u > 0).count(), 2);
        // The two in use still play.
        assert!(pool.elapsed(&keys[6]).is_some() && pool.elapsed(&keys[7]).is_some());
        // A stopped pose is reused by another clip at 0.
        let walk = pool.acquire(&PoseWant::new("constructor", "walk", true));
        assert_eq!(pool.elapsed(&walk), Some(0.0));
        assert_eq!(pool.pool_of("constructor").len(), 5);
        // Other model files have their own pool.
        let other = pool.acquire(&PoseWant::new("settler", "walk", true));
        assert_eq!(pool.pool_of("settler").len(), 1);
        assert_ne!(other.instance, walk.instance);
    }

    /// The install's settler skeleton: the rest pose against the stored
    /// bind pose, the walk clip binds its tracks, a played frame differs
    /// from the rest, the mesh's palette fits. Skipped without `S2_GAME_DIR`.
    // spec: GFX-ANIM-1.1, GFX-GR2-4.4
    #[test]
    fn install_settler_walk() {
        let Ok(dir) = std::env::var("S2_GAME_DIR") else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let load = |rel: &str| {
            let path = std::path::Path::new(&dir).join(rel);
            let data = std::fs::read(&path).expect("file");
            gr2::load(&path.to_string_lossy(), &data).expect("gr2")
        };
        let model = load("data/character/models/settler_model.GR2");
        let s = Skeleton::from_gr2(&model.skeletons[0]);
        assert_eq!(s.len(), 46);
        // GFX-OBJ-1.8: the mesh is bound facing file -y; the rest pose
        // turns every bone by the same quarter turn about z, to face +x.
        let rest = rest_composites(&s);
        for c in &rest {
            assert!(c.abs_diff_eq(rest[0], 1e-3), "{c}");
        }
        let ahead = rest[0].transform_vector3(Vec3::new(0.0, -1.0, 0.0));
        assert!(ahead.abs_diff_eq(Vec3::X, 1e-3), "{ahead}");
        // GFX-OBJ T6: in the game frame the posed settler faces +z, which
        // the heading of direction 0 turns to +x; no extra turn.
        let b = crate::render::models3d::basis_mat4(&crate::render::models3d::basis_conversion(
            model.art_tool.as_ref(),
            1.2,
        ));
        let posed = conjugate(b, b.inverse(), rest[0]);
        let forward = posed
            .transform_vector3(b.transform_vector3(Vec3::new(0.0, -1.0, 0.0)))
            .normalize();
        assert!(forward.abs_diff_eq(Vec3::Z, 1e-3), "{forward}");
        let h = crate::render::world3d::heading(0);
        let turned = Quat::from_rotation_y(h) * forward;
        assert!(turned.abs_diff_eq(Vec3::X, 1e-3), "{turned}");
        let walk = load("data/character/animations/settler_walk.GR2");
        let clip = Clip::bind(&walk, &s).unwrap();
        assert!(close(clip.duration, 1.0));
        assert!(clip.tracks.iter().filter(|t| t.is_some()).count() >= 44);
        let frame = clip.composites(&s, 0.25, true);
        let calf = s.find("Bip01 L Calf").unwrap();
        assert!(!frame[calf].abs_diff_eq(Mat4::IDENTITY, 1e-2));
        // The settler mesh's palette fits the 50 matrices.
        let mesh = load("data/character/meshes/settler_mesh.GR2");
        let p = palette(&mesh.meshes[0], &s).unwrap();
        assert!(p.fits() && !p.bones.is_empty());
    }
}
