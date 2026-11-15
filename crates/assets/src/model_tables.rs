//! The model tables of `data/settings` and `data/character`: which `.gr2`
//! file draws a building, its construction site, a ware, an animal, a ship
//! and a settler, and the scales of GFX-VIEW-1.3. The files are XML; the
//! elements and attributes read here are the ones `specs/GFX-world-view.md`
//! (GFX-VIEW-1.3, 4.4, 4.5) and `specs/GFX-animations.md` name. No FMT spec
//! describes the files as a whole (Q-174).

use crate::anim_tables::{self, AnimEntry, AnimEvent};
use crate::xml;

/// A building's animation (`DoorOpen`, `DoorClose`, `Working`) with its
/// events (GFX-OBJ-1.1, GFX-ANIM-6).
#[derive(Debug, Clone, PartialEq)]
pub struct BuildingClip {
    pub file: String,
    pub events: Vec<AnimEvent>,
}

/// A building's `Production` block: animals shown while it produces
/// (GFX-ANIM-6.3): their mesh, skeleton, texture, clips and the bones of
/// the building they stand at.
#[derive(Debug, Clone, PartialEq)]
pub struct ProductionModel {
    pub mesh: String,
    pub model: Option<String>,
    pub texture: Option<String>,
    pub anims: Vec<String>,
    pub bone_links: Vec<String>,
}

/// A `Building` entry of `buildings.xml`: its `name` (the tribe and the
/// building's graphics name, as `Roman_Sawmill`), the finished building's
/// `mesh` and the construction site's `frame` (GFX-VIEW-4.4), its door and
/// working animations and production animals (GFX-OBJ-1.1).
#[derive(Debug, Clone, PartialEq)]
pub struct BuildingModel {
    pub name: String,
    pub mesh: Option<String>,
    pub frame: Option<String>,
    pub door_open: Option<BuildingClip>,
    pub door_close: Option<BuildingClip>,
    pub working: Option<BuildingClip>,
    pub production: Option<ProductionModel>,
    /// GFX-ANIM-6.4, GFX-VIEW-4.3a: the `heading` attribute in degrees;
    /// no shipped entry sets it, so every building takes
    /// [`DEFAULT_BUILDING_HEADING`].
    pub heading: i32,
}

/// GFX-ANIM-6.4: the type loader's default building heading (degrees).
pub const DEFAULT_BUILDING_HEADING: i32 = -30;

/// GFX-OBJ-5.2: a stack layout of `items.xml`: the six ware offsets (in
/// units of the ware's bounding box) and the goods that use it.
#[derive(Debug, Clone, PartialEq)]
pub struct StackLayout {
    pub offsets: [[f32; 3]; 6],
    pub goods: Vec<String>,
}

/// An `Item` of `items.xml`'s item definitions: a good's mesh (GFX-VIEW-4.5).
#[derive(Debug, Clone, PartialEq)]
pub struct ItemModel {
    pub name: String,
    pub mesh: String,
}

/// An animal kind of `animals.xml` (the element's own name, as `Deer`), its
/// `mesh`, `model` (skeleton), `scale` (GFX-VIEW-1.3) and its Stand, Walk
/// and Dying animations (GFX-ANIM-1.1).
#[derive(Debug, Clone, PartialEq)]
pub struct AnimalModel {
    pub kind: String,
    pub mesh: String,
    pub model: Option<String>,
    pub scale: f32,
    pub animations: Vec<AnimEntry>,
}

/// A `Ship` of `ships.xml`: the tribe it belongs to, its `mesh`, the
/// construction `frame`, its `scale`, skeleton `model` and the `idle` and
/// `move` animations (GFX-OBJ-1.6, GFX-ANIM-7).
#[derive(Debug, Clone, PartialEq)]
pub struct ShipModel {
    pub tribe: String,
    pub mesh: String,
    pub frame: Option<String>,
    pub scale: f32,
    pub model: Option<String>,
    pub idle: Option<String>,
    pub moving: Option<String>,
}

/// A `Character` of `settler_config.xml`: its `name` (tribe and kind, as
/// `Roman_Carrier`), its `mesh` and LOD meshes `mesh1..3`, the colour-index
/// map `color` and the colour map name `slotMap` (GFX-OBJ-1.7).
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterModel {
    pub name: String,
    pub mesh: String,
    pub lods: Vec<String>,
    pub color: Option<String>,
    pub slot_map: String,
}

/// The `Donkey` of `settler_config.xml` (GFX-OBJ-1.7).
#[derive(Debug, Clone, PartialEq)]
pub struct DonkeyModel {
    pub scale: f32,
    pub mesh: String,
    pub model: Option<String>,
    pub link: Option<String>,
    pub animations: Vec<AnimEntry>,
}

/// The `Characters` element's settings (GFX-ANIM-3.5, 4.3, 4.4,
/// GFX-OBJ-3.4).
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterSettings {
    pub evade_radius_scale: f32,
    pub evade_start: f32,
    pub boat_offset: [f32; 2],
    pub good_offset: [f32; 2],
    pub good_rotation: f32,
    pub lod_ranges: [f32; 3],
}

impl Default for CharacterSettings {
    fn default() -> Self {
        Self {
            evade_radius_scale: 1.0,
            evade_start: 0.0,
            boat_offset: [0.0; 2],
            good_offset: [0.0; 2],
            good_rotation: 0.0,
            lod_ranges: [f32::MAX; 3],
        }
    }
}

/// The model tables, read from the decoded file texts.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModelTables {
    pub buildings: Vec<BuildingModel>,
    /// `buildings.xml` root: `colorMap` and `numVariations` (GFX-OBJ-2.2).
    pub color_map: Option<String>,
    pub num_variations: u32,
    pub items: Vec<ItemModel>,
    /// GFX-OBJ-5: the stack layouts and the stacks' distance from the flag.
    pub stack_layouts: Vec<StackLayout>,
    pub stack_distance: f32,
    pub animals: Vec<AnimalModel>,
    pub ships: Vec<ShipModel>,
    pub characters: Vec<CharacterModel>,
    /// `Characters scale` of `settler_config.xml` (GFX-VIEW-1.3), 1 unless
    /// given.
    pub character_scale: f32,
    pub character_settings: CharacterSettings,
    /// `Models/Model file`: the settlers' shared skeleton.
    pub settler_model: Option<String>,
    pub donkey: Option<DonkeyModel>,
}

fn scale_of(element: &xml::Element) -> f32 {
    element
        .get("scale")
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(1.0)
}

fn number(e: &xml::Element, key: &str) -> Option<f32> {
    e.get(key)?.trim().parse().ok()
}

/// Two numbers separated by spaces.
fn pair(e: &xml::Element, key: &str) -> Option<[f32; 2]> {
    let mut it = e.get(key)?.split_whitespace().map(|v| v.parse().ok());
    Some([it.next()??, it.next()??])
}

/// The elements from `start` up to its end tag.
fn block(elements: &[xml::Element], start: usize) -> &[xml::Element] {
    let close = format!("/{}", elements[start].tag);
    let end = elements[start + 1..]
        .iter()
        .position(|e| e.tag == close)
        .map_or(elements.len(), |i| start + 1 + i);
    &elements[start + 1..end]
}

impl ModelTables {
    /// `buildings.xml`: the root's colour map and variations, every
    /// `Building` element with a name and its children.
    pub fn read_buildings(&mut self, text: &str) {
        let elements = xml::elements(text);
        if let Some(root) = elements.iter().find(|e| e.tag == "Buildings") {
            self.color_map = root.get("colorMap").map(str::to_owned);
            self.num_variations = root
                .get("numVariations")
                .and_then(|n| n.trim().parse().ok())
                .unwrap_or(1);
        }
        let mut buildings: Vec<BuildingModel> = Vec::new();
        let mut current: Option<String> = None;
        for e in &elements {
            match e.tag.as_str() {
                "Building" => {
                    current = e.get("name").map(str::to_owned);
                    if let Some(name) = &current {
                        buildings.push(BuildingModel {
                            name: name.clone(),
                            mesh: e.get("mesh").map(str::to_owned),
                            frame: e.get("frame").map(str::to_owned),
                            door_open: None,
                            door_close: None,
                            working: None,
                            production: None,
                            heading: e
                                .get("heading")
                                .and_then(|h| h.trim().parse().ok())
                                .unwrap_or(DEFAULT_BUILDING_HEADING),
                        });
                    }
                }
                "/Building" => current = None,
                tag => {
                    let Some(b) = buildings.last_mut().filter(|_| current.is_some()) else {
                        continue;
                    };
                    let clip = || {
                        e.get("file").map(|f| BuildingClip {
                            file: f.to_owned(),
                            events: Vec::new(),
                        })
                    };
                    match tag {
                        "DoorOpen" => b.door_open = clip(),
                        "DoorClose" => b.door_close = clip(),
                        "Working" => b.working = clip(),
                        "Sound" | "Particle" => {
                            if let (Some(ev), Some(w)) = (anim_tables::event(e), b.working.as_mut())
                            {
                                w.events.push(ev);
                            }
                        }
                        "Production" => {
                            let count =
                                |k: &str| e.get(k).and_then(|n| n.trim().parse::<usize>().ok());
                            let list = |prefix: &str, n: usize| {
                                (0..n)
                                    .filter_map(|i| e.get(&format!("{prefix}{i}")))
                                    .map(str::to_owned)
                                    .collect()
                            };
                            if let Some(mesh) = e.get("mesh") {
                                b.production = Some(ProductionModel {
                                    mesh: mesh.to_owned(),
                                    model: e.get("model").map(str::to_owned),
                                    texture: e.get("texture").map(str::to_owned),
                                    anims: list("anim", count("numAnims").unwrap_or(0)),
                                    bone_links: list("boneLink", count("numModels").unwrap_or(0)),
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        self.buildings = buildings;
    }

    /// `items.xml`: the `Item` elements inside `ItemDefinitions`, the
    /// `StackLayouts`.
    pub fn read_items(&mut self, text: &str) {
        let mut inside = false;
        let mut items = Vec::new();
        self.stack_layouts.clear();
        for e in xml::elements(text) {
            match e.tag.as_str() {
                "StackLayouts" => {
                    self.stack_distance = number(&e, "distance").unwrap_or(0.0);
                }
                "Layout" => {
                    let offset = |i: usize| -> [f32; 3] {
                        let v: Vec<f32> = e
                            .get(&format!("offset{i}"))
                            .unwrap_or_default()
                            .split_whitespace()
                            .filter_map(|x| x.parse().ok())
                            .collect();
                        [0, 1, 2].map(|k| v.get(k).copied().unwrap_or(0.0))
                    };
                    self.stack_layouts.push(StackLayout {
                        offsets: std::array::from_fn(offset),
                        goods: (0..)
                            .map_while(|i| e.get(&format!("good{i}")).map(str::to_owned))
                            .collect(),
                    });
                }
                "ItemDefinitions" => inside = true,
                "/ItemDefinitions" => inside = false,
                "Item" if inside => {
                    if let (Some(name), Some(mesh)) = (e.get("name"), e.get("mesh")) {
                        items.push(ItemModel {
                            name: name.to_owned(),
                            mesh: mesh.to_owned(),
                        });
                    }
                }
                _ => {}
            }
        }
        self.items = items;
    }

    /// `animals.xml`: every element with a `mesh` attribute is a kind; the
    /// animations inside it are its clips.
    pub fn read_animals(&mut self, text: &str) {
        let elements = xml::elements(text);
        self.animals = elements
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                Some(AnimalModel {
                    mesh: e.get("mesh")?.to_owned(),
                    model: e.get("model").map(str::to_owned),
                    scale: scale_of(e),
                    kind: e.tag.clone(),
                    animations: anim_tables::entries(block(&elements, i)),
                })
            })
            .collect();
    }

    /// `ships.xml`: the `Ship` elements.
    pub fn read_ships(&mut self, text: &str) {
        self.ships = xml::elements(text)
            .into_iter()
            .filter(|e| e.tag == "Ship")
            .filter_map(|e| {
                Some(ShipModel {
                    tribe: e.get("tribe")?.to_owned(),
                    mesh: e.get("mesh")?.to_owned(),
                    frame: e.get("frame").map(str::to_owned),
                    scale: scale_of(&e),
                    model: e.get("model").map(str::to_owned),
                    idle: e.get("idle").map(str::to_owned),
                    moving: e.get("move").map(str::to_owned),
                })
            })
            .collect();
    }

    /// `settler_config.xml`: the `Characters` settings, every `Character`,
    /// the shared skeleton and the donkey.
    pub fn read_characters(&mut self, text: &str) {
        let elements = xml::elements(text);
        if let Some(c) = elements.iter().find(|e| e.tag == "Characters") {
            self.character_scale = scale_of(c);
            let d = CharacterSettings::default();
            self.character_settings = CharacterSettings {
                evade_radius_scale: number(c, "evadeRadiusScale").unwrap_or(d.evade_radius_scale),
                evade_start: number(c, "evadeStart").unwrap_or(d.evade_start),
                boat_offset: pair(c, "boatOffset").unwrap_or(d.boat_offset),
                good_offset: pair(c, "goodOffset").unwrap_or(d.good_offset),
                good_rotation: number(c, "goodRotation").unwrap_or(d.good_rotation),
                lod_ranges: [
                    number(c, "lod0Range").unwrap_or(f32::MAX),
                    number(c, "lod1Range").unwrap_or(f32::MAX),
                    number(c, "lod2Range").unwrap_or(f32::MAX),
                ],
            };
        } else {
            self.character_scale = 1.0;
        }
        self.characters = elements
            .iter()
            .filter(|e| e.tag == "Character")
            .filter_map(|e| {
                Some(CharacterModel {
                    name: e.get("name")?.to_owned(),
                    mesh: e.get("mesh")?.to_owned(),
                    lods: (1..=3)
                        .map_while(|i| e.get(&format!("mesh{i}")).map(str::to_owned))
                        .collect(),
                    color: e.get("color").map(str::to_owned),
                    slot_map: e.get("slotMap").unwrap_or("default").to_owned(),
                })
            })
            .collect();
        self.settler_model = elements
            .iter()
            .find(|e| e.tag == "Model")
            .and_then(|e| e.get("file"))
            .map(str::to_owned);
        self.donkey = elements
            .iter()
            .position(|e| e.tag == "Donkey")
            .and_then(|i| {
                let e = &elements[i];
                Some(DonkeyModel {
                    scale: scale_of(e),
                    mesh: e.get("mesh")?.to_owned(),
                    model: e.get("model").map(str::to_owned),
                    link: e.get("link").map(str::to_owned),
                    animations: anim_tables::entries(block(&elements, i)),
                })
            });
    }

    pub fn building(&self, name: &str) -> Option<&BuildingModel> {
        self.buildings.iter().find(|b| b.name == name)
    }

    pub fn item(&self, name: &str) -> Option<&ItemModel> {
        self.items.iter().find(|i| i.name == name)
    }

    pub fn character(&self, name: &str) -> Option<&CharacterModel> {
        self.characters.iter().find(|c| c.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUILDINGS: &str = r#"<?xml version="1.0" ?>
<Buildings lodBias="-70.0" colorMap="data\buildings\colors.tga">
<!-- ROMANS -->
	<Building
		name="Roman_Sawmill"
		mesh="data\buildings\PB_R_Sawmill01.gr2"
		frame="data\buildings\PB_R_Sawmill01ConstructionSite.GR2"
	>
		<DoorOpen file="data\buildings\PB_R_Sawmill01\door_open.GR2" />
		<Working file="data\buildings\PB_R_Sawmill01\working.GR2">
			<Particle start="0" name="SawDust" slot="dust"/>
		</Working>
	</Building>
	<!-- <Building name="Roman_Gone" mesh="x.gr2" /> -->
	<Building name="Roman_RuinSmall01" mesh="data\buildings\ruin.gr2"/>
</Buildings>"#;

    // spec: GFX-OBJ-1.1, GFX-VIEW-4.4
    #[test]
    fn building_entries() {
        let mut tables = ModelTables::default();
        tables.read_buildings(BUILDINGS);
        assert_eq!(tables.buildings.len(), 2);
        let sawmill = tables.building("Roman_Sawmill").unwrap();
        assert_eq!(
            sawmill.frame.as_deref(),
            Some(r"data\buildings\PB_R_Sawmill01ConstructionSite.GR2")
        );
        assert_eq!(tables.building("Roman_RuinSmall01").unwrap().frame, None);
        assert_eq!(sawmill.door_close, None);
        assert!(sawmill
            .door_open
            .as_ref()
            .unwrap()
            .file
            .ends_with("door_open.GR2"));
        let working = sawmill.working.as_ref().unwrap();
        assert_eq!(working.events.len(), 1);
        assert_eq!(working.events[0].slot.as_deref(), Some("dust"));
        assert_eq!(
            tables.color_map.as_deref(),
            Some(r"data\buildings\colors.tga")
        );
        assert!(tables.building("Roman_Gone").is_none());
        // GFX-ANIM-6.4: no entry sets `heading`: the loader's -30 degrees.
        assert_eq!(sawmill.heading, -30);
        let mut turned = ModelTables::default();
        turned.read_buildings(
            r#"<Buildings><Building name="X" mesh="x.gr2" heading="15"/></Buildings>"#,
        );
        assert_eq!(turned.building("X").unwrap().heading, 15);
    }

    // spec: GFX-OBJ-1.6, GFX-OBJ-1.7, GFX-VIEW-1.3
    #[test]
    fn items_animals_ships_characters() {
        let mut tables = ModelTables::default();
        tables.read_items(
            r#"<Items><ItemDefinitions>
            <Item name="Wood" mesh="data\items\GO_Log01.gr2"/>
            <Item name="Fishing Rod"	mesh="data\items\GO_FishingRod01.gr2"/>
            </ItemDefinitions><DonkeyItems><Item references="Wood" mesh="dummy.gr2"/></DonkeyItems></Items>"#,
        );
        assert_eq!(tables.items.len(), 2);
        assert_eq!(
            tables.item("Fishing Rod").unwrap().mesh,
            r"data\items\GO_FishingRod01.gr2"
        );
        tables.read_animals(
            r#"<Animals><Deer model="data\animals\deer_model.GR2" mesh="data\animals\deer.GR2" scale="1.5">
            <Animations><Animation name="Stand" filename="x.GR2"/></Animations></Deer>
            <Rabbit mesh="r.GR2" scale="2.0"/></Animals>"#,
        );
        assert_eq!(tables.animals.len(), 2);
        assert_eq!(
            (tables.animals[0].kind.as_str(), tables.animals[0].scale),
            ("Deer", 1.5)
        );
        tables.read_ships(
            r#"<Ships><Freight mesh="f.GR2"/><Ship tribe="Roman" scale="1.0"
            frame="data\ships\site.GR2" mesh="data\ships\PV_R_Ship01.GR2"/></Ships>"#,
        );
        assert_eq!(tables.ships.len(), 1);
        assert_eq!(tables.ships[0].tribe, "Roman");
        tables.read_characters(
            r#"<settler_config><Characters scale = "1.2" goodOffset = "-0.5  0.4">
            <Character name = "Roman_Carrier" mesh = "data\character\meshes\settler_mesh.GR2"/>
            </Characters></settler_config>"#,
        );
        assert_eq!(tables.character_scale, 1.2);
        assert_eq!(tables.characters[0].name, "Roman_Carrier");
        assert_eq!(tables.characters[0].slot_map, "default");
        assert_eq!(tables.character_settings.good_offset, [-0.5, 0.4]);
    }

    /// The install's own tables. Skipped without `S2_GAME_DIR`.
    // spec: GFX-OBJ-1.1, GFX-OBJ-1.6, GFX-OBJ-1.7
    #[test]
    fn install_tables() {
        let Ok(dir) = crate::GameDir::from_env() else {
            eprintln!("skipped: S2_GAME_DIR is not set");
            return;
        };
        let read = |relative: &str| {
            let path = dir.root().join(relative);
            let data = std::fs::read(&path).expect("the file");
            let data = crate::rc00::decode(&path.to_string_lossy(), &data).unwrap_or(data);
            String::from_utf8_lossy(&data).into_owned()
        };
        let mut tables = ModelTables::default();
        tables.read_buildings(&read("data/settings/buildings.xml"));
        tables.read_items(&read("data/settings/items.xml"));
        tables.read_animals(&read("data/settings/animals.xml"));
        tables.read_ships(&read("data/settings/ships.xml"));
        tables.read_characters(&read("data/character/settler_config.xml"));
        // 36 building entries for each of the three tribes.
        assert_eq!(tables.buildings.len(), 108);
        assert!(tables.building("Roman_Lumberjack").is_some());
        assert!(tables.building("Chinese_Castle").is_some());
        // GFX-VIEW-4.3a: every shipped building at -30 degrees.
        assert!(tables.buildings.iter().all(|b| b.heading == -30));
        assert!(tables.item("Wood").is_some());
        // GFX-VIEW-1.3: deer and elk 1.5, rabbit 2.0, ships 1.0, settlers 1.2.
        let scale = |kind: &str| {
            tables
                .animals
                .iter()
                .find(|a| a.kind == kind)
                .map(|a| a.scale)
        };
        assert_eq!(
            (scale("Deer"), scale("Elk"), scale("Rabbit")),
            (Some(1.5), Some(1.5), Some(2.0))
        );
        assert_eq!(tables.ships.len(), 3);
        assert!(tables.ships.iter().all(|s| s.scale == 1.0));
        assert_eq!(tables.character_scale, 1.2);
        // GFX-OBJ-1.1, 1.7, GFX-ANIM-1.1.
        assert_eq!(tables.num_variations, 8);
        let deer = tables.animals.iter().find(|a| a.kind == "Deer").unwrap();
        let names: Vec<&str> = deer.animations.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["Stand", "Walk", "Dying"]);
        let carrier = tables.character("Roman_Carrier").unwrap();
        assert_eq!(carrier.lods.len(), 3);
        assert!(carrier.color.is_some());
        assert_eq!(tables.character("Roman_Baker").unwrap().slot_map, "Baker");
        assert!(tables
            .settler_model
            .as_deref()
            .unwrap()
            .ends_with("settler_model.gr2"));
        let donkey = tables.donkey.as_ref().unwrap();
        assert_eq!(donkey.link.as_deref(), Some("SLOT_Goods"));
        assert!(donkey.animations.iter().any(|a| a.name == "WalkWood"));
        assert_eq!(tables.character_settings.lod_ranges, [40.0, 50.0, 60.0]);
        // GFX-OBJ-5.2: layout A (Wood ...) and layout B (Water ...).
        assert_eq!(tables.stack_layouts.len(), 2);
        assert_eq!(tables.stack_distance, 0.5);
        assert_eq!(tables.stack_layouts[0].offsets[5], [0.0, 2.0, 0.0]);
        assert_eq!(tables.stack_layouts[0].goods[0], "Wood");
        assert_eq!(tables.stack_layouts[1].offsets[0], [-0.5, 0.0, -1.0]);
        assert_eq!(tables.stack_layouts[1].goods.len(), 6);
        assert!(tables.ships[0].idle.is_some() && tables.ships[0].model.is_some());
        let breeding = tables.building("Roman_Donkeybreeding").unwrap();
        assert_eq!(breeding.production.as_ref().unwrap().bone_links.len(), 5);
    }
}
