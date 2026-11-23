//! Hook W30d: a project's own figure, a binary glTF file (`.glb`) of rigid
//! parts that turn at their joints, moved by clips of its own, drawn by the
//! engine where and as it draws the settler kind it stands for. Not part of
//! the original game; nothing here runs unless a project names a figure
//! (`extend::FigureLook`).
//!
//! How it is drawn (`docs/extending.md`, "A figure's own model"):
//! - As a project's building (`project`): every node a bone, every mesh
//!   rigid on its node; flat faces, the palette x the vertex colour, in the
//!   map objects' facet family (the game's light, shadows, night).
//! - The file faces glTF +Z, the engine's settlers face the other way: the
//!   figure is turned half round about the vertical once, at load; it is
//!   made in game units (the settlers' scale 1.2 is not applied).
//! - The owner's colour: the faces on the palette's `team` swatch.
//! - The far version's parts are posed by the model's own nodes (the same
//!   names), so both are one model in one pose; the view shows one of them
//!   by the figure's height on the screen.
//! - Its clips are chosen by the motion the game asks for ([`motion`]);
//!   they are written in real seconds. Walk and WalkCarry are played by the
//!   ground the figure covers, one cycle per [`STRIDE`], never by the clock.

use super::project::{self, FacetKind, PartData};
use super::*;
use crate::animation::settler_anim::{PICKUP, PUT_DOWN, STAND, SWIM_FWD, SWIM_IDLE, WALK};
use s2_assets::glb::Glb;

/// The point the carried ware hangs at (GFX-OBJ-7.2: its pose in the
/// playing clip, nothing added).
pub const HAND_POINT: &str = "attach_hand";
/// The figure's clips, by the motion the game asks for.
pub const STAND_CLIP: &str = "Stand";
pub const WALK_CLIP: &str = "Walk";
pub const STAND_CARRY_CLIP: &str = "StandCarry";
pub const WALK_CARRY_CLIP: &str = "WalkCarry";
pub const PICKUP_CLIP: &str = "Pickup";
pub const PUT_DOWN_CLIP: &str = "PutDown";
/// The clips a figure file must hold: the fallbacks of every motion.
pub const REQUIRED_CLIPS: [&str; 2] = [STAND_CLIP, WALK_CLIP];

/// The ground one cycle of a walking clip covers, in game units.
pub const STRIDE: f32 = 1.5;

/// The phases a walking cycle is drawn in: one shared pose each, so that
/// figures in step share their parts' materials and are drawn in batches.
/// At the full walk (2.7 units a second) a phase lasts 17 ms, a frame.
pub const WALK_PHASES: u32 = 32;

/// The palette's `team` swatch (palette version 2, column 4, row 4) as the
/// file's texture coordinates hold it (glTF: v down): a face whose corners'
/// middle lies in this cell takes the owner's colour.
pub const TEAM_CELL: [[f32; 2]; 2] = [[0.5, 0.625], [0.5, 0.625]];

/// The half turn about the vertical from the file's frame (facing +Z) to
/// the settlers' (GFX-VIEW-4.3: a settler of heading 0 faces the game's +z,
/// Bevy's -Z).
pub fn turn() -> Mat4 {
    Mat4::from_rotation_y(std::f32::consts::PI)
}

/// The figure's clip for the base index the game's choice gave
/// (GFX-ANIM-3.1), and whether it loops: standing and walking with a ware
/// at the hand `StandCarry` and `WalkCarry`, without `Stand` and `Walk`;
/// taking up and putting down `Pickup` and `PutDown`; another motion by the
/// base's name (`WorkLoop0`, `Hit`), played once.
pub fn motion(base: usize, carrying: bool) -> (&'static str, bool) {
    match base {
        PICKUP => (PICKUP_CLIP, false),
        PUT_DOWN => (PUT_DOWN_CLIP, false),
        WALK | SWIM_FWD if carrying => (WALK_CARRY_CLIP, true),
        WALK | SWIM_FWD => (WALK_CLIP, true),
        STAND | SWIM_IDLE if carrying => (STAND_CARRY_CLIP, true),
        STAND | SWIM_IDLE => (STAND_CLIP, true),
        other => (
            crate::world::scene3d::SETTLER_BASES
                .get(other)
                .copied()
                .unwrap_or(STAND_CLIP),
            false,
        ),
    }
}

/// The clip a figure plays for the motion `wanted` (with its loop flag)
/// when the file holds `available`: the motion's own; else a carrying
/// motion's plain one (`WalkCarry` to `Walk`); else `Walk` while it moves,
/// `Stand` at rest (looping). The second value names the missing clip.
pub fn choose<'a>(
    wanted: (&'a str, bool),
    moving: bool,
    available: &[String],
) -> ((&'a str, bool), Option<&'a str>) {
    let has = |name: &str| available.iter().any(|a| a == name);
    let name = wanted.0;
    if has(name) {
        return (wanted, None);
    }
    let plain = match name {
        WALK_CARRY_CLIP => Some(WALK_CLIP),
        STAND_CARRY_CLIP => Some(STAND_CLIP),
        _ => None,
    };
    if let Some(plain) = plain.filter(|p| has(p)) {
        return ((plain, true), Some(name));
    }
    let fallback = if moving { WALK_CLIP } else { STAND_CLIP };
    ((fallback, true), Some(name))
}

/// A clip played by the ground covered ([`STRIDE`] a cycle), not by time.
pub fn walks(clip: &str) -> bool {
    clip == WALK_CLIP || clip == WALK_CARRY_CLIP
}

/// The walking phase (0 .. 1 of a cycle) after `moved` game units.
pub fn advance(phase: f32, moved: f32) -> f32 {
    (phase + moved / STRIDE).rem_euclid(1.0)
}

/// The drawn phase of a walking cycle at `phase` (0 .. 1).
pub fn phase_index(phase: f32) -> u32 {
    ((phase.rem_euclid(1.0) * WALK_PHASES as f32) as u32).min(WALK_PHASES - 1)
}

/// The clip file of a figure's clip (`<model>#figure:<clip>`, a walking
/// phase `...@<phase>`), as the model library loads clips.
pub fn clip_file(model: &str, clip: &str, phase: Option<u32>) -> String {
    match phase {
        Some(p) => project::clip_file(model, &format!("figure:{clip}@{p}")),
        None => project::clip_file(model, &format!("figure:{clip}")),
    }
}

/// A figure clip's name and walking phase from the clip part of its file
/// (`figure:<clip>[@<phase>]`); none for a building's clip.
pub fn parse_clip(name: &str) -> Option<(&str, Option<u32>)> {
    let rest = name.strip_prefix("figure:")?;
    match rest.split_once('@') {
        Some((clip, phase)) => Some((clip, Some(phase.parse().ok()?))),
        None => Some((rest, None)),
    }
}

/// A figure's clip on its nodes: written in real seconds, so its keys are
/// stretched by the animation clock's speed (GFX-ANIM-2.1: the clock runs
/// 1.8 animation seconds a second) to play at their own pace; at a walking
/// phase the clip's pose there, held.
pub fn clip(
    glb: &Glb,
    name: &str,
    phase: Option<u32>,
    skeleton: &anim::Skeleton,
    bone_of: &[usize],
) -> Option<anim::Clip> {
    // The clip files' keys are lower case (the pose pool's).
    let name = &glb
        .animations
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(name))?
        .name;
    let clip = project::clip_scaled(glb, name, skeleton, bone_of, anim::GLOBAL_SPEED)?;
    let Some(phase) = phase else {
        return Some(clip);
    };
    let t = clip.duration * (phase.min(WALK_PHASES - 1) as f32 + 0.5) / WALK_PHASES as f32;
    let locals = clip.locals(skeleton, t, true);
    let held = |values: Vec<f32>| gr2::Curve {
        degree: 1,
        knots: vec![0.0],
        controls: values,
    };
    let tracks = locals
        .iter()
        .map(|l| {
            Some([
                held(l.position.to_array().to_vec()),
                held(l.orientation.to_array().to_vec()),
                held(l.scale_shear.transpose().to_cols_array().to_vec()),
            ])
        })
        .collect();
    Some(anim::Clip::from_curves(1.0, tracks))
}

/// GFX-OBJ-7.2 for a figure: the carried item's matrix in the figure's
/// frame: the hand point's pose `point` (the file's frame, the view's own,
/// as every project model's) turned with the figure. The item as the view
/// holds it (up along its +Y, a plank or log along its Z, its origin its
/// grip) takes the point's frame: its origin on the point, its Y along the
/// point's Y (the fingers), its length along the point's Z; nothing added.
pub fn ware_matrix(point: Mat4) -> Mat4 {
    turn() * point
}

/// Whether a face of corners at `uvs` lies on the `team` swatch: the
/// middle of its corners in [`TEAM_CELL`].
pub fn on_team_cell(uvs: [[f32; 2]; 3]) -> bool {
    let u = (uvs[0][0] + uvs[1][0] + uvs[2][0]) / 3.0;
    let v = (uvs[0][1] + uvs[1][1] + uvs[2][1]) / 3.0;
    let [us, vs] = TEAM_CELL;
    (us[0]..us[1]).contains(&u) && (vs[0]..vs[1]).contains(&v)
}

/// A part split into its faces on the `team` swatch (a team part, the
/// owner's colour) and the others (as it was); a part with only one kind
/// stays whole.
pub fn split_team(part: PartData) -> Vec<PartData> {
    if part.kind != FacetKind::Solid || part.uvs.len() != part.positions.len() {
        return vec![part];
    }
    let (mut team, mut rest) = (Vec::new(), Vec::new());
    for t in part.indices.as_chunks::<3>().0.iter() {
        let uvs = [t[0], t[1], t[2]].map(|i| part.uvs[i as usize]);
        if on_team_cell(uvs) {
            team.extend_from_slice(t);
        } else {
            rest.extend_from_slice(t);
        }
    }
    if team.is_empty() {
        return vec![part];
    }
    if rest.is_empty() {
        return vec![PartData {
            kind: FacetKind::Team,
            ..part
        }];
    }
    let team_part = PartData {
        kind: FacetKind::Team,
        indices: team,
        ..part.clone()
    };
    vec![
        PartData {
            indices: rest,
            ..part
        },
        team_part,
    ]
}

/// A part turned by `m` (positions and normals).
fn transformed(mut part: PartData, m: Mat4) -> PartData {
    let normals = Mat3::from_mat4(m).inverse().transpose();
    for p in &mut part.positions {
        *p = m.transform_point3(Vec3::from_array(*p)).to_array();
    }
    for n in &mut part.normals {
        *n = (normals * Vec3::from_array(*n))
            .normalize_or_zero()
            .to_array();
    }
    part
}

/// The parts of a far version posed by the model's nodes: each part's bone
/// the model's node of the same name, its vertices where the far file has
/// them at rest (so the two versions stand alike; a node placed otherwise
/// in the far file turns about the model's). The error names the node the
/// model lacks.
pub fn far_on_model(far: &Glb, model: &anim::Skeleton) -> Result<Vec<PartData>, String> {
    let (skeleton, bone_of) = project::skeleton(far);
    project::parts(far, &skeleton, &bone_of)
        .into_iter()
        .map(|part| {
            let name = &skeleton.names[part.bone];
            let bone = model
                .find(name)
                .ok_or_else(|| format!("the far version's node {name} is not in the model"))?;
            Ok(PartData { bone, ..part })
        })
        .collect()
}

/// What a figure file lacks to be drawn, if anything: a mesh, at most
/// [`anim::MAX_BONES`] nodes, the clips [`REQUIRED_CLIPS`] and the point
/// [`HAND_POINT`].
pub fn lacks(glb: &Glb) -> Option<String> {
    if glb.meshes.is_empty() {
        return Some("the file holds no mesh".into());
    }
    if glb.nodes.len() > anim::MAX_BONES {
        return Some(format!(
            "{} nodes (more than {})",
            glb.nodes.len(),
            anim::MAX_BONES
        ));
    }
    if let Some(clip) = REQUIRED_CLIPS.iter().find(|c| glb.animation(c).is_none()) {
        return Some(format!("no clip {clip}"));
    }
    if glb.node(HAND_POINT).is_none() {
        return Some(format!("no point {HAND_POINT}"));
    }
    None
}

/// A figure's parts of one version (model or far) and one kind (plain or
/// the owner's colour) in one mesh: each vertex bound with weight 1 to its
/// part's node through the mesh's palette (`palette[bones[v]]`), so a figure
/// is two drawn meshes, not one per node.
pub struct MergedPart {
    pub far: bool,
    pub kind: FacetKind,
    pub palette: Vec<usize>,
    pub bones: Vec<u32>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub colours: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    pub texture: Option<Handle<Image>>,
    pub light: Option<Handle<Image>>,
}

/// The parts merged by version and kind, in the order the first of each
/// comes (the model's before the far version's).
pub fn merge_parts(data: Vec<project::TexturedPart>) -> Vec<MergedPart> {
    let mut out: Vec<MergedPart> = Vec::new();
    for t in data {
        let part = t.part;
        let at = match out
            .iter()
            .position(|m| m.far == t.far && m.kind == part.kind)
        {
            Some(i) => i,
            None => {
                out.push(MergedPart {
                    far: t.far,
                    kind: part.kind,
                    palette: Vec::new(),
                    bones: Vec::new(),
                    positions: Vec::new(),
                    normals: Vec::new(),
                    uvs: Vec::new(),
                    colours: Vec::new(),
                    indices: Vec::new(),
                    texture: t.texture.clone(),
                    light: t.light.clone(),
                });
                out.len() - 1
            }
        };
        let m = &mut out[at];
        let slot = match m.palette.iter().position(|&b| b == part.bone) {
            Some(slot) => slot,
            None => {
                m.palette.push(part.bone);
                m.palette.len() - 1
            }
        } as u32;
        let base = m.positions.len() as u32;
        let count = part.positions.len();
        m.bones.extend(std::iter::repeat_n(slot, count));
        m.positions.extend(part.positions);
        m.normals.extend(part.normals);
        let uvs = if part.uvs.len() == count {
            part.uvs
        } else {
            vec![[0.0; 2]; count]
        };
        m.uvs.extend(uvs);
        let colours = if part.colours.len() == count {
            part.colours
        } else {
            vec![[1.0; 4]; count]
        };
        m.colours.extend(colours);
        m.indices.extend(part.indices.iter().map(|i| i + base));
    }
    out
}

/// The figure's model from its merged parts: one mesh each, skinned rigidly
/// through its palette.
fn build_merged(
    parts: Vec<MergedPart>,
    skeleton: anim::Skeleton,
    file: &str,
    meshes: &mut Assets<Mesh>,
) -> LoadedModel {
    let near = || parts.iter().filter(|p| !p.far).flat_map(|p| &p.positions);
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for p in near() {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let extent = [0, 1, 2].map(|k| (hi[k] - lo[k]).max(0.0));
    let bounds = game_box(near());
    let out = parts
        .into_iter()
        .enumerate()
        .map(|(group, m)| {
            let bone_index: Vec<[u32; 4]> = m.bones.iter().map(|&b| [b, 0, 0, 0]).collect();
            let count = m.positions.len();
            let mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::RENDER_WORLD,
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, m.positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, m.normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, m.uvs.clone())
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, m.uvs.clone())
            .with_inserted_attribute(ATTRIBUTE_UV_2, m.uvs)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, m.colours)
            .with_inserted_attribute(ATTRIBUTE_BONE_INDEX, bone_index)
            .with_inserted_attribute(ATTRIBUTE_BONE_WEIGHT, vec![[1.0f32, 0.0, 0.0, 0.0]; count])
            .with_inserted_indices(Indices::U32(m.indices));
            LoadedPart {
                mesh: meshes.add(mesh),
                texture: m.texture,
                light: m.light,
                palette: m.palette,
                ground: false,
                decal: Vec::new(),
                piece: group,
                group,
                facet: Some(project::Facet {
                    kind: m.kind,
                    decal_colours: Vec::new(),
                    far: m.far,
                }),
            }
        })
        .collect();
    LoadedModel {
        parts: out,
        skeleton: Some(Arc::new(skeleton)),
        skeleton_file: file.to_owned(),
        basis: turn(),
        basis_inverse: turn().inverse(),
        extent,
        bounds,
        house: Some(bounds),
    }
}

impl ModelLibrary {
    /// Hook W30d: a figure's model (with its far version's parts, flagged
    /// `far`), turned into the settlers' frame, its faces on the `team`
    /// swatch tinted by the owner; loaded once.
    pub(crate) fn figure_model(
        &mut self,
        look: &crate::extend::FigureLook,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
    ) -> Option<LoadedModel> {
        let key = (
            file_key(&look.model),
            format!(
                "figure|{}",
                look.far.as_deref().map(file_key).unwrap_or_default()
            ),
            0,
        );
        if let Some(found) = self.models.get(&key) {
            return found.clone();
        }
        let loaded = self.load_figure(look, meshes, images);
        self.models.insert(key, loaded.clone());
        self.loads.push(format!("model {}", look.model));
        loaded
    }

    fn load_figure(
        &mut self,
        look: &crate::extend::FigureLook,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
    ) -> Option<LoadedModel> {
        let glb = self.read_project_model(&look.model)?;
        if let Some(lack) = lacks(&glb) {
            eprintln!("figure {}: {lack}, not drawn", look.model);
            return None;
        }
        let (skeleton, bone_of) = project::skeleton(&glb);
        let turn = turn();
        let prepared = |data: Vec<PartData>| -> Vec<PartData> {
            data.into_iter()
                .flat_map(split_team)
                .map(|p| transformed(p, turn))
                .collect()
        };
        let own_images = project::material_images(&glb, images);
        let mut data = project::textured_parts(
            prepared(project::parts(&glb, &skeleton, &bone_of)),
            &own_images,
            false,
        );
        if let Some(far_file) = &look.far {
            let far = self.read_project_model(far_file)?;
            match far_on_model(&far, &skeleton) {
                Ok(parts) => {
                    let far_images = project::material_images(&far, images);
                    data.extend(project::textured_parts(prepared(parts), &far_images, true));
                }
                Err(e) => eprintln!("figure {far_file}: {e}, the model at every size"),
            }
        }
        // Its clips, each walking one at every phase, bound once here.
        let skeleton_key = file_key(&look.model);
        for a in &glb.animations {
            let phases: Vec<Option<u32>> = if walks(&a.name) {
                (0..WALK_PHASES).map(Some).collect()
            } else {
                vec![None]
            };
            for phase in phases {
                let bound = clip(&glb, &a.name, phase, &skeleton, &bone_of).map(Arc::new);
                let file = file_key(&clip_file(&look.model, &a.name, phase));
                self.clips.insert((file, skeleton_key.clone()), bound);
            }
        }
        self.figure_clips.insert(
            file_key(&look.model),
            glb.animations.iter().map(|a| a.name.clone()).collect(),
        );
        Some(build_merged(
            merge_parts(data),
            skeleton,
            &look.model,
            meshes,
        ))
    }

    /// Hook W30d: the clip names of a figure's file (none before
    /// [`Self::figure_model`] loaded it).
    pub(crate) fn figure_clips(&self, model: &str) -> &[String] {
        self.figure_clips
            .get(&file_key(model))
            .map_or(&[][..], Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::settler_anim::{HIT, WORK_LOOP0};

    fn fixture(name: &str) -> Glb {
        let path = format!(
            "{}/../../tests/models/{name}.glb",
            env!("CARGO_MANIFEST_DIR")
        );
        s2_assets::glb::load(&std::fs::read(&path).expect("tests/models")).expect("loads")
    }

    fn names(glb: &Glb) -> Vec<String> {
        glb.animations.iter().map(|a| a.name.clone()).collect()
    }

    // spec: none (hook W30d: the motion the game asks for picks the figure's clip)
    #[test]
    fn the_motion_picks_the_clip() {
        assert_eq!(motion(STAND, false), (STAND_CLIP, true));
        assert_eq!(motion(WALK, false), (WALK_CLIP, true));
        assert_eq!(motion(STAND, true), (STAND_CARRY_CLIP, true));
        assert_eq!(motion(WALK, true), (WALK_CARRY_CLIP, true));
        assert_eq!(motion(PICKUP, true), (PICKUP_CLIP, false));
        assert_eq!(motion(PUT_DOWN, false), (PUT_DOWN_CLIP, false));
        // In the water as on land (the engine's lowering is not the figure's).
        assert_eq!(motion(SWIM_FWD, true), (WALK_CARRY_CLIP, true));
        assert_eq!(motion(SWIM_IDLE, false), (STAND_CLIP, true));
        // Another motion by its base's name, once.
        assert_eq!(motion(WORK_LOOP0, false), ("WorkLoop0", false));
        assert_eq!(motion(HIT, false), ("Hit", false));
    }

    // spec: none (hook W30d: a motion the file has no clip for falls back to Stand or Walk, naming the clip)
    #[test]
    fn a_missing_clip_falls_back() {
        let available = names(&fixture("figure"));
        assert_eq!(
            choose(motion(WALK, true), true, &available),
            ((WALK_CARRY_CLIP, true), None)
        );
        assert_eq!(
            choose(motion(PICKUP, false), false, &available),
            ((PICKUP_CLIP, false), None)
        );
        // No StandCarry in the file: Stand, and the missing one named.
        assert_eq!(
            choose(motion(STAND, true), false, &available),
            ((STAND_CLIP, true), Some(STAND_CARRY_CLIP))
        );
        // No PutDown: Stand at rest.
        assert_eq!(
            choose(motion(PUT_DOWN, true), false, &available),
            ((STAND_CLIP, true), Some(PUT_DOWN_CLIP))
        );
        // A rowing carrier's WorkLoop1 while moving: Walk.
        assert_eq!(
            choose(("WorkLoop1", true), true, &available),
            ((WALK_CLIP, true), Some("WorkLoop1"))
        );
        // A file without WalkCarry walks with a ware in Walk.
        let plain = vec![STAND_CLIP.to_owned(), WALK_CLIP.to_owned()];
        assert_eq!(
            choose(motion(WALK, true), true, &plain),
            ((WALK_CLIP, true), Some(WALK_CARRY_CLIP))
        );
    }

    // spec: none (hook W30d: the walk is played by the ground covered, a cycle a stride, in 32 shared phases)
    #[test]
    fn the_walk_follows_the_ground() {
        assert!(walks(WALK_CLIP) && walks(WALK_CARRY_CLIP));
        assert!(!walks(STAND_CLIP) && !walks(PICKUP_CLIP));
        // A whole stride: the same phase (the feet land where they did).
        assert!((advance(0.3, STRIDE) - 0.3).abs() < 1e-5);
        assert!((advance(0.0, STRIDE / 2.0) - 0.5).abs() < 1e-6);
        // Slower over the ground (a slope, the free walk): slower through
        // the cycle, by the same rule.
        let mut fast = 0.0;
        let mut slow = 0.0;
        for _ in 0..10 {
            fast = advance(fast, 2.7 / 60.0);
            slow = advance(slow, 1.35 / 60.0);
        }
        assert!((fast - 2.0 * slow).abs() < 1e-5, "{fast} {slow}");
        assert_eq!(phase_index(0.0), 0);
        assert_eq!(phase_index(0.999), WALK_PHASES - 1);
        assert_eq!(phase_index(1.0), 0);
        assert_eq!(phase_index(0.5), WALK_PHASES / 2);
        assert_eq!(
            clip_file("data/c.glb", WALK_CLIP, Some(7)),
            "data/c.glb#figure:Walk@7"
        );
        assert_eq!(parse_clip("figure:Walk@7"), Some((WALK_CLIP, Some(7))));
        assert_eq!(parse_clip("figure:Stand"), Some((STAND_CLIP, None)));
        assert_eq!(parse_clip("DoorOpen"), None);
        assert_eq!(parse_clip("figure:Walk@x"), None);
    }

    // spec: none (hook W30d: the clips in real seconds; a walking phase holds its pose)
    #[test]
    fn the_clips_play_in_real_seconds() {
        let glb = fixture("figure");
        let (skeleton, bone_of) = project::skeleton(&glb);
        let stand = clip(&glb, STAND_CLIP, None, &skeleton, &bone_of).expect("Stand");
        // 2.0 real seconds on the 1.8-times animation clock.
        assert!((stand.duration - 2.0 * anim::GLOBAL_SPEED).abs() < 1e-5);
        let pickup = clip(&glb, "pickup", None, &skeleton, &bone_of).expect("any case");
        assert!((pickup.duration - 0.4 * anim::GLOBAL_SPEED).abs() < 1e-5);
        let walk = clip(&glb, WALK_CLIP, None, &skeleton, &bone_of).expect("Walk");
        let thigh = skeleton.find("thigh_l").expect("thigh");
        for phase in [0, 8, 20] {
            let held = clip(&glb, WALK_CLIP, Some(phase), &skeleton, &bone_of).expect("phase");
            let t = walk.duration * (phase as f32 + 0.5) / WALK_PHASES as f32;
            let want = walk.locals(&skeleton, t, true)[thigh].orientation;
            // The same pose whatever the clock says.
            for elapsed in [0.0, 0.3, 7.0] {
                let got = held.locals(&skeleton, held.time_at(elapsed, true), true)[thigh];
                assert!(got.orientation.abs_diff_eq(want, 1e-5), "{phase} {elapsed}");
            }
        }
        // A quarter cycle on, the thigh has swung forward (x turn > 0).
        let quarter = clip(&glb, WALK_CLIP, Some(8), &skeleton, &bone_of).expect("phase");
        let q = quarter.locals(&skeleton, 0.0, true)[thigh].orientation;
        assert!(q.x > 0.15, "{q}");
        assert!(clip(&glb, "StandCarry", None, &skeleton, &bone_of).is_none());
    }

    // spec: none (hook W30d: the faces on the team swatch take the owner's colour, the others keep theirs)
    #[test]
    fn the_tunic_takes_the_owners_colour() {
        assert!(on_team_cell([[0.5625, 0.5625]; 3]));
        assert!(!on_team_cell([[0.0625, 0.0625]; 3]));
        // The swatch beside it (column 4, row 3) is not the team's.
        assert!(!on_team_cell([[0.5625, 0.4375]; 3]));
        let glb = fixture("figure");
        let (skeleton, bone_of) = project::skeleton(&glb);
        let parts: Vec<PartData> = project::parts(&glb, &skeleton, &bone_of)
            .into_iter()
            .flat_map(split_team)
            .collect();
        let kinds = |node: &str| -> Vec<FacetKind> {
            parts
                .iter()
                .filter(|p| p.node == node)
                .map(|p| p.kind)
                .collect()
        };
        // The hips: the belt as it was, the tunic in the owner's colour.
        assert_eq!(kinds("hips"), [FacetKind::Solid, FacetKind::Team]);
        assert_eq!(kinds("chest"), [FacetKind::Team]);
        assert_eq!(kinds("head"), [FacetKind::Solid]);
        assert_eq!(kinds("thigh_l"), [FacetKind::Solid]);
        // Every face in one of them, none twice.
        let hips: usize = parts
            .iter()
            .filter(|p| p.node == "hips")
            .map(|p| p.indices.len())
            .sum();
        assert_eq!(hips, 72);
    }

    // spec: none (hook W30d: a figure is drawn as one mesh per version and kind, each vertex rigid on its node)
    #[test]
    fn a_figure_is_few_meshes() {
        let glb = fixture("figure");
        let (skeleton, bone_of) = project::skeleton(&glb);
        let near: Vec<PartData> = project::parts(&glb, &skeleton, &bone_of)
            .into_iter()
            .flat_map(split_team)
            .collect();
        let far: Vec<PartData> = far_on_model(&fixture("figure_far"), &skeleton)
            .expect("far")
            .into_iter()
            .flat_map(split_team)
            .collect();
        let triangles: usize = near.iter().chain(&far).map(|p| p.indices.len()).sum();
        let hips = skeleton.find("hips").expect("hips");
        let head = skeleton.find("head").expect("head");
        let mut data = project::textured_parts(near, &[], false);
        data.extend(project::textured_parts(far, &[], true));
        let merged = merge_parts(data);
        let kinds: Vec<(bool, FacetKind)> = merged.iter().map(|m| (m.far, m.kind)).collect();
        assert_eq!(
            kinds,
            [
                (false, FacetKind::Solid),
                (false, FacetKind::Team),
                (true, FacetKind::Team),
                (true, FacetKind::Solid)
            ]
        );
        assert_eq!(
            merged.iter().map(|m| m.indices.len()).sum::<usize>(),
            triangles
        );
        // Every vertex on its own node: the head's vertices on the head.
        let solid = &merged[0];
        assert!(solid.palette.contains(&head) && solid.palette.contains(&hips));
        assert_eq!(solid.bones.len(), solid.positions.len());
        let top = (0..solid.positions.len())
            .max_by(|&a, &b| solid.positions[a][1].total_cmp(&solid.positions[b][1]))
            .expect("vertices");
        assert_eq!(solid.palette[solid.bones[top] as usize], head);
        assert!(solid
            .indices
            .iter()
            .all(|&i| (i as usize) < solid.positions.len()));
    }

    // spec: none (hook W30d: the figure turned to the settlers' facing, in game units; its far version on the model's nodes)
    #[test]
    fn the_figure_faces_its_heading() {
        let glb = fixture("figure");
        let (skeleton, bone_of) = project::skeleton(&glb);
        let parts = project::parts(&glb, &skeleton, &bone_of);
        let nose = parts
            .iter()
            .find(|p| p.node == "head")
            .expect("head")
            .positions
            .iter()
            .map(|&p| turn().transform_point3(Vec3::from_array(p)))
            .fold(f32::MAX, |z, p| z.min(p.z));
        // The nose (the file's +Z) leads towards Bevy's -Z, where a settler
        // of heading 0 faces.
        assert!((nose + 0.27).abs() < 1e-5, "{nose}");
        // 2.15 high, the 1.2 scale not applied.
        let top = parts
            .iter()
            .flat_map(|p| &p.positions)
            .map(|p| p[1])
            .fold(f32::MIN, f32::max);
        assert!((top - 2.15).abs() < 1e-5, "{top}");
        // The far version's thigh_l (its node 0.1 lower, its mesh 0.1
        // higher) stands where the model's does, on the model's node.
        let far = far_on_model(&fixture("figure_far"), &skeleton).expect("same nodes");
        let span = |p: &PartData| {
            let ys = p.positions.iter().map(|q| q[1]);
            (
                ys.clone().fold(f32::MAX, f32::min),
                ys.fold(f32::MIN, f32::max),
            )
        };
        let near_thigh = parts.iter().find(|p| p.node == "thigh_l").expect("thigh");
        let far_thigh = far.iter().find(|p| p.node == "thigh_l").expect("far thigh");
        assert_eq!(far_thigh.bone, near_thigh.bone);
        let (a, b) = (span(near_thigh), span(far_thigh));
        assert!(
            (a.0 - b.0).abs() < 1e-5 && (a.1 - b.1).abs() < 1e-5,
            "{a:?} {b:?}"
        );
        assert!(far.iter().all(|p| p.bone < skeleton.len()));
        let lighter: usize = far.iter().map(|p| p.indices.len()).sum();
        let full: usize = parts.iter().map(|p| p.indices.len()).sum();
        assert!(lighter < full);
        // A far file whose nodes the model lacks is not taken.
        let (building, _) = project::skeleton(
            &s2_assets::glb::load(
                &std::fs::read(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../tests/models/facets.glb"
                ))
                .expect("facets"),
            )
            .expect("loads"),
        );
        let err = far_on_model(&fixture("figure_far"), &building).expect_err("other nodes");
        assert!(err.contains("hips"), "{err}");
    }

    // spec: GFX-OBJ-7.2 (hook W30d: the ware at the hand point, its pose in the playing clip, nothing added)
    #[test]
    fn the_ware_hangs_at_the_hand() {
        let glb = fixture("figure");
        let (skeleton, bone_of) = project::skeleton(&glb);
        let hand = skeleton.find(HAND_POINT).expect("the point");
        let world_at = |name: &str| {
            let c = clip(&glb, name, Some(4), &skeleton, &bone_of).expect("clip");
            skeleton.world(&c.locals(&skeleton, 0.0, true))[hand]
        };
        // WalkCarry raises the right arm: the hand is above the shoulder
        // (1.6); in Walk it hangs at the hip.
        let carry = world_at(WALK_CARRY_CLIP);
        let walk = world_at(WALK_CLIP);
        assert!(carry.w_axis.y > 1.6, "{}", carry.w_axis);
        assert!(walk.w_axis.y < 1.0, "{}", walk.w_axis);
        // The item's origin on the point, its axes the point's (turned with
        // the figure).
        let m = ware_matrix(carry);
        let origin = m.transform_point3(Vec3::ZERO);
        assert!(origin.abs_diff_eq(turn().transform_point3(carry.w_axis.truncate()), 1e-5));
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            let want = (turn() * carry).transform_vector3(axis);
            assert!(m.transform_vector3(axis).abs_diff_eq(want, 1e-5));
        }
    }

    // spec: none (hook W30d: what a figure file must hold; a file that cannot be drawn is an error starting with its name)
    #[test]
    fn what_a_figure_file_holds() {
        assert_eq!(lacks(&fixture("figure")), None);
        assert_eq!(lacks(&fixture("figure_far")), None);
        assert_eq!(lacks(&fixture("facets")), Some("no clip Stand".into()));
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let folders = [root.join("tests/models")];
        assert_eq!(
            crate::extend::unusable_figure(&folders, &root, ["figure.glb", "figure_far.glb"]),
            Ok(())
        );
        let missing =
            crate::extend::unusable_figure(&folders, &root, ["figure.glb", "data/no/carrier.glb"])
                .expect_err("missing");
        assert!(missing.starts_with("data/no/carrier.glb: "), "{missing}");
        let building = crate::extend::unusable_figure(&folders, &root, ["facets.glb"])
            .expect_err("a building");
        assert_eq!(building, "facets.glb: no clip Stand");
        let not_a_model =
            crate::extend::unusable_figure(&folders, &root, ["Cargo.toml"]).expect_err("no glb");
        assert!(not_a_model.starts_with("Cargo.toml: "), "{not_a_model}");
    }

    // spec: none (hook W30d: --figure-model; no look: no figure)
    #[test]
    fn the_figure_model_option() {
        use crate::app::options::figure_look;
        let look =
            figure_look("Carrier=data/s/carrier.glb,far=data/s/carrier_far.glb,far-below=0.05")
                .expect("parses");
        assert_eq!(look.figure, "Carrier");
        assert_eq!(look.model, "data/s/carrier.glb");
        assert_eq!(look.far.as_deref(), Some("data/s/carrier_far.glb"));
        assert_eq!(look.far_below, 0.05);
        assert_eq!(
            figure_look("Carrier=c.glb").expect("parses"),
            crate::extend::FigureLook::new("Carrier", "c.glb")
        );
        assert_eq!(
            crate::extend::FigureLook::new("Carrier", "c.glb").far_below,
            crate::extend::FIGURE_FAR_BELOW
        );
        assert!(figure_look("Carrier").is_err());
        assert!(figure_look("=c.glb").is_err());
        assert!(figure_look("Carrier=c.glb,far-below=-1").is_err());
        assert!(figure_look("Carrier=c.glb,colour=red").is_err());
        // Without a project's figure every settler is the original's.
        assert!(crate::extend::figure_look_of("Carrier").is_none());
    }
}
