// GFX-OBJ-2.6a: pieces in numeric name order, else file order.
#[test]
fn piece_order_by_name() {
    assert_eq!(super::piece_order(&["13", "2", "0", "1"]), [3, 2, 0, 1]);
    assert_eq!(super::piece_order(&["b", "a"]), [0, 1]);
    assert_eq!(super::piece_order(&["1", "GROUND"]), [0, 1]);
}

use super::*;

// GFX-VIEW-2.7: streets, then decals, then the rest, whatever the
// view depth (0 at the camera to -FAR).
#[test]
fn ground_layers_in_fixed_order() {
    let key = |f: Family, depth: f32| depth + ground_layer_bias(f as u32 as f32);
    let far = -crate::render::world3d::FAR;
    assert!(key(Family::Road, 0.0) < key(Family::Decal, far));
    assert!(key(Family::Decal, 0.0) < far);
    for f in [
        Family::Plain,
        Family::House,
        Family::Border,
        Family::Settler,
    ] {
        assert_eq!(ground_layer_bias(f as u32 as f32), 0.0);
    }
}

/// F55 (the long play): slots given back are used again from the next frame, so
/// the buffer stops growing however many poses come and go; slot 0 stays.
// spec: GFX-ANIM-8.2
#[test]
fn palette_slots_are_used_again() {
    let mut buffers = Assets::<ShaderBuffer>::default();
    let mut palettes = Palettes::default();
    palettes.reset(&mut buffers);
    let start = palettes.capacity();
    let a = palettes.alloc(&Bones::IDENTITY);
    assert_ne!(a, 0, "slot 0 is the identity's");
    palettes.release(a);
    let b = palettes.alloc(&Bones::IDENTITY);
    assert_ne!(b, a, "not in the frame it was given back");
    palettes.flush(&mut buffers);
    assert_eq!(palettes.alloc(&Bones::IDENTITY), a, "from the next");
    palettes.release(0);
    palettes.flush(&mut buffers);
    assert_ne!(palettes.alloc(&Bones::IDENTITY), 0);
    // A long game: more poses come and go than the buffer holds.
    for _ in 0..10_000 {
        let slot = palettes.alloc(&Bones::IDENTITY);
        palettes.release(slot);
        palettes.flush(&mut buffers);
    }
    assert_eq!(palettes.capacity(), start);
}

/// F55 (the long play): a house's look holds a site's progress only while it is
/// built (its plane or its frame's pieces); the variation alone is a finished
/// house's, and any other family's look is kept as ever.
// spec: GFX-OBJ-2.2, GFX-OBJ-2.6a, GFX-OBJ-2.6b
#[test]
fn only_a_site_at_work_has_a_look_of_its_own() {
    let mut house = Look::family(Family::House, [0.0; 3]);
    house.extra = [0.25, 0.0, 0.0, 0.0];
    assert!(!house.of_a_site_at_work());
    house.extra = [0.25, 12.5, 1.0, 0.0];
    assert!(house.of_a_site_at_work());
    house.extra = [0.25, 0.0, 0.0, 1.5];
    assert!(house.of_a_site_at_work());
    let mut flag = Look::family(Family::Flag, [0.0; 3]);
    flag.extra[1] = 1.0 / 3.0;
    assert!(!flag.of_a_site_at_work());
}

// GFX-OBJ-6.2 (F55): the street pass's parts keep their order from any camera,
// and all of them stay under the decals.
// spec: GFX-OBJ-6.2, GFX-VIEW-2.7
#[test]
fn street_parts_in_fixed_order() {
    let road = Family::Road as u32 as f32;
    let far = -crate::render::world3d::FAR;
    let key =
        |part: f32, depth: f32| depth + ground_layer_bias(road) + street_part_bias(road, part);
    for part in 0..3 {
        let part = part as f32;
        assert!(key(part, 0.0) < key(part + 1.0, far));
    }
    assert!(key(3.0, 0.0) < far + ground_layer_bias(Family::Decal as u32 as f32));
    assert_eq!(street_part_bias(Family::House as u32 as f32, 3.0), 0.0);
}

// spec: GFX-OBJ-1.1, GFX-OBJ-1.3
#[test]
fn model_lookups() {
    assert_eq!(building_entry(0, "Lumberjack"), "Roman_Lumberjack");
    assert_eq!(building_entry(2, "Castle"), "Chinese_Castle");
    assert_eq!(
        deposit_model("tree_07").as_deref(),
        Some("data/map/objects/RS_TreeFir01.GR2")
    );
    assert_eq!(
        deposit_model("medstoneressource_03").as_deref(),
        Some("data/map/objects/RS_MedStoneRessource03.GR2")
    );
    assert_eq!(
        deposit_model("stoneressource_06").as_deref(),
        Some("data/map/objects/RS_StoneRessource06.GR2")
    );
    // GFX-OBJ-1.3 (Q-174): the shipped table's test deposits.
    assert_eq!(
        deposit_model("do_not_use_01").as_deref(),
        Some("data/map/objects/RS_TreeBirch01.GR2")
    );
}

// GFX-OBJ-1.1a: a building's levels of detail are the files named by the
// model's name up to the extension, LOD<n>, the extension; a level's own
// file has none.
// spec: GFX-OBJ-1.1a
#[test]
fn levels_of_detail_are_files() {
    use super::lod_file;
    assert_eq!(
        lod_file("data/buildings/PB_R_Sawmill01.GR2", 0).as_deref(),
        Some("data/buildings/PB_R_Sawmill01LOD0.GR2")
    );
    assert_eq!(lod_file("a/B.gr2", 3).as_deref(), Some("a/BLOD3.gr2"));
    assert_eq!(lod_file("a/BLOD0.GR2", 1), None);
    assert_eq!(lod_file("a/Blod12.GR2", 0), None);
    assert_eq!(lod_file("noextension", 0), None);
}

// GFX-VIEW-2.6 a: a model texture keeps the mip levels its file stores
// when they make the whole chain; otherwise the chain is made from the top
// level.
// spec: GFX-VIEW-2.6
#[test]
fn model_textures_keep_their_files_mips() {
    let texture = s2_assets::dds::Texture {
        width: 4,
        height: 4,
        format: s2_assets::dds::Format::Dxt1,
        mip_levels: 3,
        cube_map: false,
        rgba: [10u8, 20, 30, 255].repeat(16),
    };
    let levels = vec![
        (2, 2, [200u8, 0, 0, 0].repeat(4)),
        (1, 1, vec![0, 200, 0, 255]),
    ];
    let image = library::model_image_with_levels(&texture, &levels);
    assert_eq!(image.texture_descriptor.mip_level_count, 3);
    let data = image.data.unwrap();
    assert_eq!(data.len(), 64 + 16 + 4);
    assert_eq!(&data[64..68], &[200, 0, 0, 0]);
    assert_eq!(&data[80..84], &[0, 200, 0, 255]);
    // Without the 1 x 1 level: the chain made from the top level.
    let made = library::model_image_with_levels(&texture, &levels[..1]);
    assert_eq!(made.texture_descriptor.mip_level_count, 3);
    assert_eq!(&made.data.unwrap()[64..68], &[10, 20, 30, 255]);
}

// GFX-OBJ-2.8a, T48: a ship's player colour weight is its part's number:
// the hull (part 0) untinted, the sails and banner (part 1) tinted; a ship
// look of weight 0 (the construction frame) tints no part; other families
// keep their extra.
// spec: GFX-OBJ-2.8, GFX-OBJ-2.8a
#[test]
fn a_ships_weight_is_its_parts_number() {
    let mut ship = Look::family(Family::Ship, [0.01, 0.10, 0.60]);
    ship.extra[0] = 1.0;
    assert_eq!(ship.part_extra(0)[0], 0.0);
    assert_eq!(ship.part_extra(1)[0], 1.0);
    let frame = Look::family(Family::Ship, [0.01, 0.10, 0.60]);
    assert_eq!(frame.extra[0], 0.0);
    assert_eq!(frame.part_extra(0)[0], 0.0);
    assert_eq!(frame.part_extra(1)[0], 0.0);
    let mut plain = Look::family(Family::Plain, [0.6, 0.0, 0.0]);
    plain.extra = [1.0, 2.0, 3.0, 4.0];
    assert_eq!(plain.part_extra(1), plain.extra);
}

// GFX-OBJ-2.8a (a), T48, with the install: each tribe's ship has the hull
// with its `PV_<t>_Ship01` texture as part 0 and the sails with the
// `PV_<t>_Ship01PC` texture as part 1; the construction frame is one part
// (part 0, so it is never tinted). Skipped without `S2_GAME_DIR`.
// spec: GFX-OBJ-2.8a
#[test]
fn the_ship_models_parts_with_the_install() {
    let Ok(dir) = std::env::var("S2_GAME_DIR") else {
        eprintln!("skipped: S2_GAME_DIR is not set");
        return;
    };
    let parts_of = |file: &str| {
        let rel = format!("data/ships/{file}");
        let path = s2_rules::resolve(std::path::Path::new(&dir), &rel).expect(&rel);
        let data = std::fs::read(&path).expect(&rel);
        let gr2 = s2_assets::gr2::load(&path.to_string_lossy(), &data).expect(&rel);
        crate::tools::model::parts(&gr2)
    };
    for t in ["R", "N", "C"] {
        let parts = parts_of(&format!("PV_{t}_Ship01.GR2"));
        let textures: Vec<(usize, String)> = parts
            .iter()
            .map(|p| {
                let t = p.texture.clone().unwrap_or_default().to_ascii_lowercase();
                (p.group, t.trim_end_matches(".dds").to_owned())
            })
            .collect();
        let hull = format!("pv_{}_ship01", t.to_ascii_lowercase());
        let sails = format!("{hull}pc");
        assert_eq!(
            textures,
            [(0, hull), (1, sails)],
            "tribe {t}: parts and textures"
        );
    }
    let frame = parts_of("PV_R_Ship01ConstructionSite.GR2");
    assert!(!frame.is_empty());
    assert!(frame.iter().all(|p| p.group == 0), "the frame's parts");
}
