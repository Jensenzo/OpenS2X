//! What a settler holds and how (`specs/GFX-objects.md` GFX-OBJ-7,
//! `specs/GFX-animations.md` GFX-ANIM-1.3, 3.4, 3.5): the ware or the tool
//! at the attach bone, which tool a job has, when it shows, the good its
//! clips are looked up by, and a rowing carrier's boat. Plain logic; the
//! view draws what it says.

use bevy::math::{Mat4, Vec3};

/// GFX-OBJ-7.4: the settler kinds, for which job's tool they hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Holder {
    /// A worker or specialist: its own job.
    Worker,
    Builder,
    Leveller,
    Soldier,
    Carrier,
    Donkey,
}

/// What a settler holds this tick and what picks its clips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Look {
    /// GFX-ANIM-1.3: the lookup good.
    pub good: Option<String>,
    /// GFX-ANIM-1.3: the profession list's name.
    pub profession: Option<String>,
    /// The good whose mesh is drawn at the attach bone.
    pub held: Option<String>,
    /// GFX-ANIM-3.4: the base index the kind overrides, if it does.
    pub base: Option<usize>,
}

/// GFX-OBJ-7.4: the job whose cost list gives a kind's tool: workers and
/// specialists their own, builders `Constructor`, levellers `Bulldozer`,
/// soldiers `Private` (every rank holds the Sword); carriers and donkeys
/// have none.
pub fn tool_job(holder: Holder, job: &str) -> Option<&str> {
    match holder {
        Holder::Worker => Some(job),
        Holder::Builder => Some("Constructor"),
        Holder::Leveller => Some("Bulldozer"),
        Holder::Soldier => Some("Private"),
        Holder::Carrier | Holder::Donkey => None,
    }
}

/// GFX-OBJ-7.4: a job's tool: the first good of its cost list, in file
/// order, that has an `items.xml` entry.
pub fn tool_of<'a>(
    costs: impl IntoIterator<Item = &'a str>,
    has_item: impl Fn(&str) -> bool,
) -> Option<&'a str> {
    costs.into_iter().find(|g| has_item(g))
}

/// GFX-OBJ-7.5: the tool shows while the settler is drawn (the caller
/// draws it), carries no ware and stands outside the fog; the clip does not
/// matter.
pub fn tool_shown(carries_ware: bool, fogged: bool) -> bool {
    !carries_ware && !fogged
}

/// What a settler of kind `holder` with the job `job` holds and plays
/// this tick: GFX-ANIM-1.3's lookup good (the ware, else the tool, also
/// while fog hides it; Boat while it rows; none for an empty carrier) and
/// profession (the job's name; `Carrier`, `Donkey`), GFX-OBJ-7.5's drawn
/// item, and 3.4's override: a rowing carrier plays WorkLoop0 standing and
/// WorkLoop1 moving, its ware in the boat (7.6) instead of at the bone.
#[allow(clippy::too_many_arguments)]
pub fn look(
    holder: Holder,
    job: &str,
    ware: Option<&str>,
    tool_of: &dyn Fn(&str) -> Option<String>,
    moving: bool,
    fogged: bool,
    rowing: bool,
) -> Look {
    let tool = tool_job(holder, job).and_then(tool_of);
    let profession = Some(match holder {
        Holder::Carrier => "Carrier".to_owned(),
        Holder::Donkey => "Donkey".to_owned(),
        _ => job.to_owned(),
    });
    if rowing && holder == Holder::Carrier {
        return Look {
            good: Some("Boat".to_owned()),
            profession,
            held: None,
            base: Some(if moving {
                crate::animation::settler_anim::WORK_LOOP0 + 1
            } else {
                crate::animation::settler_anim::WORK_LOOP0
            }),
        };
    }
    let held = match ware {
        Some(w) => Some(w.to_owned()),
        None => tool.clone().filter(|_| tool_shown(false, fogged)),
    };
    Look {
        good: ware.map(str::to_owned).or(tool),
        profession,
        held,
        base: None,
    }
}

/// GFX-OBJ-7.6: a rowing carrier's ware relative to his own matrix W:
/// turned by `rotation` degrees about the vertical, moved by
/// (offset.x, 0, offset.y) in his frame (game frame, left-handed).
pub fn boat_ware_matrix(offset: [f32; 2], rotation: f32) -> Mat4 {
    Mat4::from_translation(Vec3::new(offset[0], 0.0, offset[1]))
        * Mat4::from_rotation_y(rotation.to_radians())
}

/// GFX-OBJ-7.6: the boat relative to the rowing carrier's W, moved by
/// `boatOffset` (x, 0, y) the same way.
pub fn boat_matrix(offset: [f32; 2]) -> Mat4 {
    Mat4::from_translation(Vec3::new(offset[0], 0.0, offset[1]))
}

/// GFX-OBJ-7.2: B(t), the ware's or tool's matrix in the settler's model
/// frame: the attach bone's model-space matrix `bone_world` (settler file
/// axes, posed) carried into the frame of the settler's converted meshes
/// by `basis`. Its full orientation is kept and nothing is added: no
/// offset, no turn, no scale (the conversion's scale moves the position
/// only, so the item keeps its own size).
pub fn attach_matrix(basis: Mat4, basis_inverse: Mat4, bone_world: Mat4) -> Mat4 {
    crate::animation::anim::conjugate(basis, basis_inverse, bone_world)
}

#[cfg(test)]
// The test values are measured (GFX-OBJ-7.8), not approximations of constants.
#[allow(clippy::approx_constant)]
mod tests {
    use super::*;
    use crate::animation::settler_anim::{resolve, STAND, WALK, WORK_LOOP0};
    use bevy::math::Quat;
    use std::path::{Path, PathBuf};

    fn game_dir() -> Option<PathBuf> {
        match std::env::var("S2_GAME_DIR") {
            Ok(d) => Some(PathBuf::from(d)),
            Err(_) => {
                eprintln!("skipped: S2_GAME_DIR is not set");
                None
            }
        }
    }

    fn gr2(dir: &Path, rel: &str) -> s2_assets::gr2::Gr2 {
        let p = s2_rules::resolve(dir, rel).expect(rel);
        let d = std::fs::read(&p).expect(rel);
        s2_assets::gr2::load(&p.to_string_lossy(), &d).expect(rel)
    }

    /// An item mesh's bounding box after the conversion (game frame, 1.0
    /// units per metre, GFX-OBJ-7.2).
    fn item_box(dir: &Path, mesh: &str) -> ([f32; 3], [f32; 3]) {
        let model = gr2(dir, &format!("data/items/{mesh}.gr2"));
        let basis = crate::render::models3d::basis_conversion(model.art_tool.as_ref(), 1.0);
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for m in &model.meshes {
            for p in &m.vertices.positions {
                let q = basis.apply(*p);
                for k in 0..3 {
                    lo[k] = lo[k].min(q[k]);
                    hi[k] = hi[k].max(q[k]);
                }
            }
        }
        (lo, hi)
    }

    type Case = (
        &'static str,
        &'static str,
        &'static str,
        bool,
        f32,
        [f32; 3],
        [f32; 4],
        [f32; 3],
        [f32; 3],
        [f32; 3],
    );

    /// GFX-OBJ-7.8: id, clip, item, looping, clip time, origin, quaternion
    /// (x, y, z, w), item centre, end -, end +.
    #[rustfmt::skip]
    const CASES: [Case; 18] = [
        ("C1", "settler_walk_plank", "GO_Plank01", true, 0.00, [0.330, 1.334, 0.096], [0.0580, -0.0386, 0.6356, 0.7689], [0.277, 1.353, 0.070], [0.262, 1.497, -0.959], [0.292, 1.209, 1.099]),
        ("C2", "settler_walk_plank", "GO_Plank01", true, 0.25, [0.284, 1.417, 0.201], [0.1004, -0.1391, 0.6605, 0.7310], [0.232, 1.437, 0.173], [0.306, 1.781, -0.805], [0.158, 1.094, 1.151]),
        ("C3", "settler_walk_plank", "GO_Plank01", true, 0.50, [0.202, 1.368, 0.272], [-0.0229, -0.1856, 0.6520, 0.7348], [0.159, 1.387, 0.231], [0.474, 1.603, -0.736], [-0.155, 1.171, 1.197]),
        ("C4", "settler_walk_pickaxe", "GO_Axe01", true, 0.00, [0.278, 1.305, 0.180], [0.1353, 0.0879, 0.6804, 0.7149], [0.239, 1.418, 0.063], [0.100, 1.451, -0.363], [0.378, 1.384, 0.488]),
        ("C5", "settler_walk_pickaxe", "GO_Axe01", true, 0.25, [0.215, 1.426, 0.224], [0.0240, 0.1109, 0.7658, 0.6329], [0.167, 1.507, 0.087], [0.087, 1.445, -0.351], [0.246, 1.570, 0.524]),
        ("C6", "settler_walk_pickaxe", "GO_Axe01", true, 0.50, [0.174, 1.349, 0.360], [0.0053, -0.0211, 0.7289, 0.6842], [0.166, 1.456, 0.233], [0.175, 1.473, -0.215], [0.156, 1.439, 0.681]),
        ("C7", "woodcutter_chop", "GO_Axe01", false, 1.24, [0.062, 0.849, 0.964], [0.5998, -0.7924, 0.0680, 0.0880], [0.038, 0.781, 1.115], [0.064, 0.877, 1.552], [0.012, 0.686, 0.677]),
        ("C8", "woodcutter_chop", "GO_Axe01", false, 2.88, [0.244, 0.899, 0.907], [-0.1401, -0.9649, 0.1331, 0.1778], [0.202, 0.963, 1.055], [0.373, 1.056, 1.460], [0.032, 0.870, 0.651]),
        ("C9", "woodcutter_chop", "GO_Axe01", false, 4.56, [0.243, 0.901, 0.907], [-0.1399, -0.9651, 0.1328, 0.1771], [0.201, 0.965, 1.056], [0.371, 1.058, 1.461], [0.031, 0.872, 0.651]),
        ("C10", "settler_walk_pickaxe", "GO_PickAxe01", true, 0.00, [0.278, 1.305, 0.180], [0.1353, 0.0879, 0.6804, 0.7149], [0.250, 1.354, 0.106], [0.111, 1.387, -0.318], [0.388, 1.321, 0.529]),
        ("C11", "settler_walk_pickaxe", "GO_PickAxe01", true, 0.25, [0.215, 1.426, 0.224], [0.0240, 0.1109, 0.7658, 0.6329], [0.186, 1.455, 0.140], [0.107, 1.393, -0.295], [0.265, 1.517, 0.575]),
        ("C12", "settler_walk_pickaxe", "GO_PickAxe01", true, 0.50, [0.174, 1.349, 0.360], [0.0053, -0.0211, 0.7289, 0.6842], [0.167, 1.394, 0.279], [0.177, 1.411, -0.167], [0.158, 1.377, 0.725]),
        ("C13", "stonecutter_crush", "GO_PickAxe01", false, 1.56, [-0.259, 0.508, 0.685], [0.1665, 0.9629, 0.0736, 0.1995], [-0.328, 0.522, 0.746], [-0.511, 0.488, 1.153], [-0.146, 0.555, 0.340]),
        ("C14", "stonecutter_crush", "GO_PickAxe01", false, 4.08, [-0.242, 0.544, 0.752], [0.1787, 0.9686, 0.0655, 0.1602], [-0.305, 0.559, 0.819], [-0.455, 0.528, 1.239], [-0.156, 0.591, 0.399]),
        ("C15", "stonecutter_crush", "GO_PickAxe01", false, 6.84, [-0.233, 0.548, 0.651], [0.1584, 0.9669, 0.0429, 0.1956], [-0.301, 0.565, 0.712], [-0.476, 0.556, 1.123], [-0.126, 0.575, 0.301]),
        ("C16", "settler_walk_log", "GO_Log01", true, 0.00, [0.390, 1.399, 0.155], [0.0876, 0.0765, 0.5600, 0.8203], [0.361, 1.410, 0.147], [0.132, 1.469, -0.849], [0.590, 1.350, 1.143]),
        ("C17", "settler_walk_log", "GO_Log01", true, 0.25, [0.347, 1.479, 0.273], [0.1437, -0.0351, 0.5864, 0.7964], [0.318, 1.491, 0.264], [0.203, 1.768, -0.715], [0.434, 1.215, 1.243]),
        ("C18", "settler_walk_log", "GO_Log01", true, 0.50, [0.251, 1.429, 0.355], [0.0238, -0.0774, 0.5897, 0.8036], [0.225, 1.440, 0.339], [0.324, 1.572, -0.672], [0.126, 1.307, 1.349]),
    ];

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|k| (a[k] - b[k]).abs() <= 0.005)
    }

    /// GFX-OBJ-7.8 C1 to C18: B(t) of the attach bone in the settler's
    /// model frame (game frame, settler scale 1.2) and the item it carries,
    /// to 0.005. Skipped without `S2_GAME_DIR`.
    #[test]
    fn c1_to_c18_attach_bone() {
        let Some(dir) = game_dir() else { return };
        let settler = gr2(&dir, "data/character/models/settler_model.GR2");
        let skeleton = crate::animation::anim::Skeleton::from_gr2(&settler.skeletons[0]);
        let bone = skeleton.find("SLOT_HandR").expect("SLOT_HandR");
        let basis = crate::render::models3d::basis_mat4(
            &crate::render::models3d::basis_conversion(settler.art_tool.as_ref(), 1.2),
        );
        let mut failures = Vec::new();
        for (id, clip, item, looping, t, origin, q, centre, lo_end, hi_end) in CASES {
            let file = gr2(&dir, &format!("data/character/animations/{clip}.GR2"));
            let clip = crate::animation::anim::Clip::bind(&file, &skeleton).expect("clip");
            let locals = clip.locals(&skeleton, clip.time_at(t, looping), looping);
            let world = skeleton.world(&locals)[bone];
            let b = attach_matrix(basis, basis.inverse(), world);
            let (lo, hi) = item_box(&dir, item);
            let mid = [0, 1, 2].map(|k| (lo[k] + hi[k]) / 2.0);
            let at = |p: [f32; 3]| b.transform_point3(Vec3::from_array(p)).to_array();
            let (_, rotation, translation) = b.to_scale_rotation_translation();
            let want = Quat::from_array(q);
            let got = if rotation.w < 0.0 {
                -rotation
            } else {
                rotation
            };
            let rot_ok = (got - want).length() <= 0.005 * 2.0;
            let checks = [
                ("origin", close(translation.to_array(), origin)),
                ("rotation", rot_ok),
                ("centre", close(at(mid), centre)),
                ("end -", close(at([mid[0], mid[1], lo[2]]), lo_end)),
                ("end +", close(at([mid[0], mid[1], hi[2]]), hi_end)),
            ];
            for (what, ok) in checks {
                if !ok {
                    failures.push(format!(
                        "{id} {what}: origin {translation:?} rotation {got:?} centre {:?}",
                        at(mid)
                    ));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// GFX-ANIM-3.8d: the forester's and leveller's `work_shovel` clip
    /// with `GO_Shovel01` at SLOT_HandR: clip time, origin, quaternion
    /// (x, y, z, w), the shovel's ends along its long axis (- is the blade).
    type ShovelCase = (f32, [f32; 3], [f32; 4], [f32; 3], [f32; 3]);

    #[rustfmt::skip]
    const SHOVEL: [ShovelCase; 5] = [
        (0.00, [0.697, 0.941, -0.025], [-0.5868, 0.6173, -0.0573, 0.5209], [0.028, 0.447, 0.348], [1.265, 1.389, -0.438]),
        (0.30, [0.088, 0.673, 0.432], [0.1178, 0.7926, 0.5334, 0.2706], [-0.437, -0.014, 0.721], [0.530, 1.348, 0.226]),
        (2.35, [0.328, 0.914, 0.358], [0.2559, 0.9232, 0.1541, 0.2419], [-0.148, 0.810, 1.128], [0.767, 1.090, -0.328]),
        (5.28, [0.079, 0.643, 0.434], [0.1138, 0.7990, 0.5412, 0.2362], [-0.396, -0.071, 0.741], [0.477, 1.342, 0.214]),
        (8.73, [0.002, 0.330, 0.920], [0.2440, 0.9077, 0.1932, 0.2814], [-0.549, 0.179, 1.630], [0.506, 0.550, 0.294]),
    ];

    /// GFX-ANIM-3.8d: the shovel test values, to 0.005. Skipped without
    /// `S2_GAME_DIR`.
    #[test]
    fn shovel_digs() {
        let Some(dir) = game_dir() else { return };
        let settler = gr2(&dir, "data/character/models/settler_model.GR2");
        let skeleton = crate::animation::anim::Skeleton::from_gr2(&settler.skeletons[0]);
        let bone = skeleton.find("SLOT_HandR").expect("SLOT_HandR");
        let basis = crate::render::models3d::basis_mat4(
            &crate::render::models3d::basis_conversion(settler.art_tool.as_ref(), 1.2),
        );
        let file = gr2(&dir, "data/character/animations/work_shovel.GR2");
        let clip = crate::animation::anim::Clip::bind(&file, &skeleton).expect("clip");
        let (lo, hi) = item_box(&dir, "GO_Shovel01");
        let mid = [0, 1, 2].map(|k| (lo[k] + hi[k]) / 2.0);
        let mut failures = Vec::new();
        for (t, origin, q, end_lo, end_hi) in SHOVEL {
            let locals = clip.locals(&skeleton, clip.time_at(t, false), false);
            let b = attach_matrix(basis, basis.inverse(), skeleton.world(&locals)[bone]);
            let at = |p: [f32; 3]| b.transform_point3(Vec3::from_array(p)).to_array();
            let (_, rotation, translation) = b.to_scale_rotation_translation();
            let got = if rotation.w < 0.0 {
                -rotation
            } else {
                rotation
            };
            let checks = [
                ("origin", close(translation.to_array(), origin)),
                ("rotation", (got - Quat::from_array(q)).length() <= 0.01),
                ("end -", close(at([mid[0], mid[1], lo[2]]), end_lo)),
                ("end +", close(at([mid[0], mid[1], hi[2]]), end_hi)),
            ];
            for (what, ok) in checks {
                if !ok {
                    failures.push(format!(
                        "t {t} {what}: origin {translation:?} rotation {got:?} ends {:?} {:?}",
                        at([mid[0], mid[1], lo[2]]),
                        at([mid[0], mid[1], hi[2]])
                    ));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// GFX-OBJ-7.8: the item boxes after the conversion.
    #[test]
    fn item_boxes() {
        let Some(dir) = game_dir() else { return };
        for (mesh, lo, hi) in [
            (
                "GO_Plank01",
                [-0.178, -0.022, -1.068],
                [0.188, 0.131, 1.010],
            ),
            ("GO_Axe01", [-0.123, -0.054, -0.580], [0.330, 0.063, 0.318]),
            (
                "GO_PickAxe01",
                [-0.291, -0.050, -0.530],
                [0.377, 0.062, 0.364],
            ),
            ("GO_Log01", [-0.172, -0.181, -1.039], [0.172, 0.238, 1.010]),
        ] {
            let (l, h) = item_box(&dir, mesh);
            assert!(close(l, lo) && close(h, hi), "{mesh}: {l:?} .. {h:?}");
        }
    }

    /// GFX-OBJ-7.4: the tool of each job, from the install's rules and
    /// `items.xml`.
    #[test]
    fn tool_table() {
        let Some(dir) = game_dir() else { return };
        let game = s2_assets::GameDir::new(&dir).expect("game dir");
        let (rules, _) = s2_rules::load(&game).expect("rules");
        let tables = crate::render::models3d::read_tables(&dir);
        let lookup = crate::world::scene3d::Lookup::new(&rules, &tables);
        let id = |name: &str| {
            rules
                .goods
                .iter()
                .find(|g| g.name.as_deref() == Some(name))
                .map(|g| g.id)
        };
        let tool = |job: &str| lookup.tool_names.get(job).cloned();
        let want = [
            ("Woodcutter", Some("Axe")),
            ("Stonecutter", Some("Pickaxe")),
            ("Miner", Some("Pickaxe")),
            ("Forester", Some("Shovel")),
            ("Fisher", Some("Fishing Rod")),
            ("Hunter", Some("Spear")),
            ("Farmer", Some("Scythe")),
            ("Baker", Some("Rolling Pin")),
            ("Cabinetmaker", Some("Saw")),
            ("Locksmith", Some("Gripper")),
            ("Slaughter", Some("Hatchet")),
            ("Smith", Some("Hammer")),
            ("ShipBuilder", Some("Hammer")),
            ("Scholar", Some("Hammer")),
            ("Coiner", Some("Pot")),
            ("Smelter", Some("Pot")),
            ("Explorer", Some("Spear")),
            ("Constructor", Some("Hammer")),
            ("Bulldozer", Some("Shovel")),
            ("Private", Some("Sword")),
            ("Miller", None),
            ("Brewer", None),
            ("Helper", None),
        ];
        let mut wrong = Vec::new();
        for (job, tool_name) in want {
            assert!(id(job).is_some(), "no job {job}");
            let got = tool(job);
            if got.as_deref() != tool_name {
                wrong.push(format!("{job}: {got:?}, want {tool_name:?}"));
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    fn tools(job: &str) -> Option<String> {
        let t = match job {
            "Woodcutter" => "Axe",
            "Stonecutter" => "Pickaxe",
            "Constructor" => "Hammer",
            "Bulldozer" => "Shovel",
            "Private" => "Sword",
            _ => return None,
        };
        Some(t.to_owned())
    }

    /// The clip a look and a base resolve to in the install's table.
    fn clip_of(
        t: &s2_assets::anim_tables::SettlerAnimations,
        base: usize,
        look: &Look,
    ) -> (String, String) {
        let base = look.base.unwrap_or(base);
        let e = resolve(
            &t.entries,
            base,
            look.good.as_deref(),
            look.profession.as_deref(),
            0,
        )
        .expect("a clip");
        (e.name.clone(), e.file.clone())
    }

    /// GFX-ANIM-1.3's table, T13 to T15, GFX-OBJ-7.5 and T11, T13, T14:
    /// each settler's clip and what it holds.
    #[test]
    fn clips_and_held_items() {
        let Some(dir) = game_dir() else { return };
        let t = crate::render::models3d::read_animations(&dir);
        let l = |holder, job, ware, moving, fogged, rowing| {
            look(holder, job, ware, &tools, moving, fogged, rowing)
        };
        let chop = false;
        let cases: [(&str, Look, usize, &str, Option<&str>); 13] = [
            (
                "woodcutter walking out",
                l(Holder::Worker, "Woodcutter", None, true, false, false),
                WALK,
                "WalkAxe",
                Some("Axe"),
            ),
            (
                "woodcutter standing",
                l(Holder::Worker, "Woodcutter", None, false, false, false),
                STAND,
                "StandAxe",
                Some("Axe"),
            ),
            (
                "woodcutter chopping",
                l(Holder::Worker, "Woodcutter", None, chop, false, false),
                WORK_LOOP0,
                "WorkLoop0Axe",
                Some("Axe"),
            ),
            (
                "woodcutter with the log",
                l(
                    Holder::Worker,
                    "Woodcutter",
                    Some("Wood"),
                    true,
                    false,
                    false,
                ),
                WALK,
                "WalkWood",
                Some("Wood"),
            ),
            (
                "stonemason walking out",
                l(Holder::Worker, "Stonecutter", None, true, false, false),
                WALK,
                "WalkPickaxe",
                Some("Pickaxe"),
            ),
            (
                "stonemason quarrying",
                l(Holder::Worker, "Stonecutter", None, chop, false, false),
                WORK_LOOP0,
                "WorkLoop0Stonecutter",
                Some("Pickaxe"),
            ),
            (
                "stonemason with the stone",
                l(
                    Holder::Worker,
                    "Stonecutter",
                    Some("Stone"),
                    true,
                    false,
                    false,
                ),
                WALK,
                "WalkStone",
                Some("Stone"),
            ),
            (
                "carrier with a plank",
                l(
                    Holder::Carrier,
                    "Carrier",
                    Some("Plank"),
                    true,
                    false,
                    false,
                ),
                WALK,
                "WalkPlank",
                Some("Plank"),
            ),
            (
                "carrier without a ware",
                l(Holder::Carrier, "Carrier", None, true, false, false),
                WALK,
                "Walk",
                None,
            ),
            (
                "soldier walking",
                l(Holder::Soldier, "Private", None, true, false, false),
                WALK,
                "WalkSword",
                Some("Sword"),
            ),
            (
                "builder hammering",
                l(Holder::Builder, "Constructor", None, chop, false, false),
                WORK_LOOP0,
                "WorkLoop0Constructor",
                Some("Hammer"),
            ),
            (
                "rowing carrier, moving",
                l(Holder::Carrier, "Carrier", Some("Plank"), true, false, true),
                WALK,
                "WorkLoop1Boat",
                None,
            ),
            (
                "woodcutter under fog",
                l(Holder::Worker, "Woodcutter", None, true, true, false),
                WALK,
                "WalkAxe",
                None,
            ),
        ];
        let mut wrong = Vec::new();
        for (what, look, base, clip, held) in cases {
            let (name, file) = clip_of(&t, base, &look);
            if name != clip || look.held.as_deref() != held {
                wrong.push(format!(
                    "{what}: {name} ({file}) holding {:?}; want {clip} holding {held:?}",
                    look.held
                ));
            }
        }
        // T15: standing in the boat.
        let still = l(Holder::Carrier, "Carrier", None, false, false, true);
        let (name, _) = clip_of(&t, STAND, &still);
        if name != "WorkLoop0Boat" {
            wrong.push(format!("rowing carrier, still: {name}"));
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    /// GFX-OBJ-7.6, T12: a rowing carrier's ware lies in the boat, turned
    /// 10 degrees about the vertical and moved by (-0.5, 0, 0.4) in the
    /// settler's frame; the boat by (-0.25, 0, 0.2).
    #[test]
    fn t12_ware_in_the_boat() {
        let m = boat_ware_matrix([-0.5, 0.4], 10.0);
        assert!(close(
            m.transform_point3(Vec3::ZERO).to_array(),
            [-0.5, 0.0, 0.4]
        ));
        let turned = m.transform_vector3(Vec3::Z);
        let angle = turned.x.atan2(turned.z).to_degrees();
        assert!((angle - 10.0).abs() < 0.01, "{angle}");
        let boat = boat_matrix([-0.25, 0.2]);
        assert!(close(
            boat.transform_point3(Vec3::ZERO).to_array(),
            [-0.25, 0.0, 0.2]
        ));
    }
}
