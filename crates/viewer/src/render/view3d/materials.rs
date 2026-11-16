//! The terrain and water materials and their shaders' parameters (GFX-VIEW-2.4, GFX-WATER-4).

use super::*;

pub(crate) const TERRAIN_SHADER: Handle<Shader> =
    uuid_handle!("6c0f5a52-35c4-4b4e-9a3c-6f8a1d0e2b71");
pub(crate) const WATER_SHADER: Handle<Shader> =
    uuid_handle!("0d3b8f7e-9a14-4c55-8e21-4b7c9e6a3f02");

/// GFX-WATER-2.2: the depth haze of a terrain vertex.
pub(crate) const ATTRIBUTE_HAZE: MeshVertexAttribute =
    MeshVertexAttribute::new("Haze", 0x5332_4841_5a45, VertexFormat::Float32x4);

/// Layers of the terrain's array textures; GFX-PAT-4 lists 73 patterns, a
/// map uses far fewer.
pub(crate) const MAX_LAYERS: usize = 64;
/// Texels per side of a layer of the colour array.
pub(crate) const COLOR_SIZE: u32 = 512;
/// GFX-VIEW-2.4b: the normal maps lie at their own size: one array per size
/// (the shipped pattern maps are 128, 256, 512 and 1024 square). A map of
/// another size goes to the smallest array it fits (the largest if none),
/// resampled.
pub(crate) const NORMAL_SIZES: [u32; 4] = [128, 256, 512, 1024];

/// GFX-VIEW-2.4b: the normal array a `width` x `height` map goes to.
pub(crate) fn normal_class(width: u32, height: u32) -> usize {
    let side = width.max(height);
    NORMAL_SIZES
        .iter()
        .position(|&s| s >= side)
        .unwrap_or(NORMAL_SIZES.len() - 1)
}

/// GFX-VIEW-2.4 inputs of the terrain shader, laid out as `terrain.wesl`
/// reads them.
#[derive(Debug, Clone, Copy, ShaderType)]
pub struct TerrainParams {
    pub(crate) layers: [Vec4; MAX_LAYERS],
    /// Per layer: render type, priority (GFX-PAT-5.1), mask set (5.4), and
    /// where its normal map lies (GFX-VIEW-2.4b: array << 8 | layer).
    pub(crate) kinds: [UVec4; MAX_LAYERS],
    /// GFX-VIEW-2.4a: per layer the highlight's intensity (x) and power (y).
    pub(crate) specular: [Vec4; MAX_LAYERS],
    pub(crate) lattice: Vec4,
    pub(crate) world_rect: Vec4,
    pub(crate) light_dir: Vec4,
    pub(crate) light_color: Vec4,
    pub(crate) ambient: Vec4,
    /// GFX-WATER-2.5: x 1 while the caustics pass is drawn.
    pub(crate) caustics: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TerrainMaterial {
    #[uniform(0)]
    pub(crate) params: TerrainParams,
    #[texture(1, dimension = "2d_array")]
    #[sampler(2)]
    pub(crate) colors: Handle<Image>,
    /// GFX-VIEW-2.4b: the normal maps, one array per size of
    /// [`NORMAL_SIZES`]; a layer's `kinds.w` names its array (bits 8 and up)
    /// and its layer there (bits 0 to 7). All share the sampler of the first.
    #[texture(3, dimension = "2d_array")]
    #[sampler(4)]
    pub(crate) normals: Handle<Image>,
    #[texture(12, dimension = "2d_array")]
    pub(crate) normals_256: Handle<Image>,
    #[texture(13, dimension = "2d_array")]
    pub(crate) normals_512: Handle<Image>,
    #[texture(14, dimension = "2d_array")]
    pub(crate) normals_1024: Handle<Image>,
    #[texture(5, sample_type = "u_int")]
    pub(crate) fields: Handle<Image>,
    #[texture(6)]
    #[sampler(7)]
    pub(crate) world: Handle<Image>,
    /// GFX-PAT-5.3, 5.4: the transition masks, one layer per set.
    #[texture(8, dimension = "2d_array")]
    #[sampler(9)]
    pub(crate) masks: Handle<Image>,
    /// GFX-WATER-2.5: the caustics strip.
    #[texture(10)]
    #[sampler(11)]
    pub(crate) caustics: Handle<Image>,
}

/// GFX-PAT-5.4: the transition mask sets in the order of `graphics.xml`'s
/// `<Transitions>`.
pub(crate) const TRANSITIONS: [&str; 7] = [
    "data/map/transitions/transition00.dds",
    "data/map/transitions/transition01.dds",
    "data/map/transitions/transition02.dds",
    "data/map/transitions/transitionC.dds",
    "data/map/transitions/transitionB.dds",
    "data/map/transitions/transitionA.dds",
    "data/map/transitions/transition03.dds",
];
/// Texels of a mask layer: two square cells side by side.
pub(crate) const MASK_HEIGHT: u32 = 128;

/// The transition masks as an array of 256 x 128 layers, clamped at the
/// edges; a missing file is an opaque layer of neutral brightness.
pub(crate) fn mask_image(root: Option<&std::path::Path>) -> Image {
    let (w, h) = (MASK_HEIGHT * 2, MASK_HEIGHT);
    let mut data = Vec::new();
    for file in TRANSITIONS {
        let texture = root
            .and_then(|root| crate::extend::find(root, file).ok())
            .and_then(|path| {
                let bytes = crate::files::read(&path).ok()?;
                s2_assets::dds::load(&path.to_string_lossy(), &bytes)
                    .map_err(|e| eprintln!("{file}: {e}"))
                    .ok()
            });
        match texture {
            Some(t) => data.extend(textures::resample_2d(&t, w, h)),
            None => data.extend([128, 128, 128, 255].repeat((w * h) as usize)),
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: TRANSITIONS.len() as u32,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..default()
    });
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

impl Material for TerrainMaterial {
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Handle(TERRAIN_SHADER)
    }
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(TERRAIN_SHADER)
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            ATTRIBUTE_HAZE.at_shader_location(2),
            // GFX-VIEW-2.1a (b): the vertex's decoded tangent bytes.
            Mesh::ATTRIBUTE_TANGENT.at_shader_location(3),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        // GFX-WATER-2.6a 3: the shader culls back faces itself, reversed in
        // the mirror view.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// GFX-WATER-4, 5 inputs of the water shader, as `water.wesl` reads them.
#[derive(Debug, Clone, Copy, ShaderType)]
pub struct WaterParams {
    pub(crate) water_color: Vec4,
    pub(crate) wave_color: Vec4,
    pub(crate) opacity: Vec4,
    pub(crate) rotation: Vec4,
    pub(crate) drift: Vec4,
    pub(crate) world_rect: Vec4,
    pub(crate) light_dir: Vec4,
    pub(crate) light_color: Vec4,
    pub(crate) ambient: Vec4,
    pub(crate) mirror: Vec4,
    /// Hook W18: the shore foam's colour (rgb) and strength (w; 0 none),
    /// and its reach (x).
    pub(crate) foam: Vec4,
    pub(crate) foam_reach: Vec4,
}

/// Hook W18: the water's foam uniforms for a project's option; none gives
/// strength 0, which skips the shader's foam step (the pixels as without).
pub(crate) fn foam_params(foam: Option<crate::extend::ShoreFoam>) -> (Vec4, Vec4) {
    match foam {
        Some(f) if f.strength > 0.0 && f.reach > 0.0 => (
            Vec4::new(f.colour[0], f.colour[1], f.colour[2], f.strength.min(1.0)),
            Vec4::new(f.reach, 0.0, 0.0, 0.0),
        ),
        _ => (Vec4::ZERO, Vec4::ZERO),
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WaterMaterial {
    #[uniform(0)]
    pub(crate) params: WaterParams,
    #[texture(1)]
    #[sampler(2)]
    pub(crate) wave0: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    pub(crate) wave1: Handle<Image>,
    #[texture(5)]
    #[sampler(6)]
    pub(crate) world: Handle<Image>,
    /// GFX-WATER-2.6: the mirror image (`mirror.rs`), or a 1 x 1 stand-in
    /// while the mirror is off.
    #[texture(7)]
    #[sampler(8)]
    pub(crate) mirror: Handle<Image>,
}

impl Material for WaterMaterial {
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Handle(WATER_SHADER)
    }
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(WATER_SHADER)
    }
    // GFX-WATER-4.1: alpha blending, depth tested, not written.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout
            .0
            .get_layout(&[Mesh::ATTRIBUTE_POSITION.at_shader_location(0)])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}

/// GFX-WATER-5.2: the `Default` water style, the only one the release maps
/// use, and the global wave settings of `graphics.xml` (GFX-WATER-4.2).
pub(crate) const WATER_COLOR: [f32; 3] = [0.3, 0.55, 0.7];
pub(crate) const WAVE_COLOR: [f32; 3] = [0.92, 0.97, 1.0];
pub(crate) const OPACITY: [f32; 2] = [0.85, 0.94];
pub(crate) const WAVE_ROTATION: [f32; 2] = [206.0, 145.0];
pub(crate) const WAVE_DRIFT: [f32; 2] = [0.015, 0.017];
/// GFX-WATER-5.2: the `Default` style's reflection factor and noise.
pub(crate) const REFLECTION_FACTOR: f32 = 0.8;
pub(crate) const REFLECTION_NOISE: [f32; 2] = [1.2, 1.2];
pub(crate) const WAVE_MAPS: [&str; 2] = [
    "data/map/textures/WaveBump1.dds",
    "data/map/textures/WaveBump2.dds",
];

#[cfg(test)]
mod tests {
    /// The field names of `struct NAME { ... }` in `source`, in order
    /// (Rust or WGSL; comments and attributes skipped).
    fn fields(source: &str, name: &str) -> Vec<String> {
        let start = source
            .find(&format!("struct {name} {{"))
            .unwrap_or_else(|| panic!("struct {name}"));
        let body = &source[start..];
        let body = &body[body.find('{').unwrap() + 1..body.find('}').unwrap()];
        body.lines()
            .map(str::trim)
            .filter(|l| !l.starts_with("//") && !l.starts_with('#') && l.contains(':'))
            .map(|l| {
                let l = l
                    .trim_start_matches("pub(crate) ")
                    .trim_start_matches("pub ");
                l[..l.find(':').unwrap()].trim().to_string()
            })
            .collect()
    }

    // The terrain material's uniform is laid out by the Rust struct; the
    // shader must list the same fields in the same order, or it reads them
    // from the wrong bytes (the terrain was not drawn at all with the type-3
    // highlight's array placed differently, GFX-VIEW-2.4a).
    // spec: none (the shader's interface)
    #[test]
    fn uniform_fields_match_the_shaders() {
        let rust = fields(include_str!("materials.rs"), "TerrainParams");
        let wgsl = fields(include_str!("../shaders/terrain.wesl"), "TerrainParams");
        assert_eq!(rust, wgsl);
        assert!(rust.contains(&"specular".to_string()));
        let rust = fields(include_str!("materials.rs"), "WaterParams");
        let wgsl = fields(include_str!("../shaders/water.wesl"), "WaterParams");
        assert_eq!(rust, wgsl);
        let rust = fields(include_str!("../models3d/mod.rs"), "ObjectParams");
        for shader in [
            include_str!("../shaders/object.wesl"),
            include_str!("../shaders/object_shadow.wesl"),
        ] {
            assert_eq!(rust, fields(shader, "ObjectParams"));
        }
    }

    // GFX-OBJ-2.1 (T31): the house pass (a house, not its site's frame
    // pass, which marks itself with extra.w = 1 + f) lights the normal
    // turned by its bone matrix only, not by the building's heading; every
    // other family and the frame pass light the world normal.
    // spec: GFX-OBJ-2.1
    #[test]
    fn the_house_pass_lights_the_models_own_normal() {
        let src = include_str!("../shaders/object.wesl");
        let vertex = &src[src.find("fn vertex").unwrap()..src.find("fn shadow_at").unwrap()];
        assert!(vertex.contains("out.light_normal = out.normal;"));
        assert!(vertex.contains("if f == HOUSE && params.extra.w < 1.0 {"));
        assert!(vertex.contains("out.light_normal = normal;"));
        let fragment = &src[src.find("fn fragment").unwrap()..];
        assert!(fragment.contains("let lambert = saturate(dot(ln, to_light));"));
        // The shadow still looks up with the world normal.
        assert!(fragment.contains("shadow_at(in.world_position, n, in.clip)"));
    }

    // GFX-OBJ-2.6a (T37), 2.2a (6): a site's frame piece is (ambient + light
    // x N . L x s) x texel, N . L per vertex from the world normal, the
    // same whatever the slot: its pass samples no colour map and no light
    // map and has no alpha test; the rising house of the same site (the
    // house pass, extra.w < 1) keeps its slot's column and light map.
    // spec: GFX-OBJ-2.6a, GFX-OBJ-2.2a
    #[test]
    fn t37_a_sites_frame_piece_reads_no_colour_column() {
        let src = include_str!("../shaders/object.wesl");
        let vertex = &src[src.find("fn vertex").unwrap()..src.find("fn shadow_at").unwrap()];
        let per_vertex = &vertex[vertex
            .find("if f == HOUSE && params.extra.w >= 1.0 {")
            .unwrap()..];
        assert!(per_vertex
            .contains("let lambert = saturate(dot(normalize(out.normal), -light.light_dir.xyz));"));
        assert!(per_vertex.contains("out.color = vec4<f32>(lambert, 0.0, 0.0, 1.0);"));
        let fragment = &src[src.find("fn fragment").unwrap()..];
        let house = &fragment[fragment.find("if f == HOUSE {").unwrap()..];
        let house = &house[..house.find("} else if f == TREE {").unwrap()];
        let frame = &house[house.find("if params.extra.w >= 1.0 {").unwrap()..];
        let (frame, rest) = frame.split_at(frame.find("} else {").unwrap());
        assert!(frame.contains(
            "let frame = light.light_color.rgb * in.color.x * shadow + light.ambient.rgb;"
        ));
        assert!(frame.contains("rgb = frame * tex.rgb;"));
        assert!(frame.contains("alpha_test = false;"));
        for sampled in ["second", "third", "params.extra.x"] {
            assert!(!frame.contains(sampled), "the frame pass reads {sampled}");
        }
        // The house pass: the column (extra.x) and the light map.
        assert!(rest.contains("vec2<f32>(in.uv2.x + params.extra.x, in.uv2.y) + half"));
        assert!(rest.contains("textureSample(third, third_sampler, in.uv1)"));
    }

    // GFX-WATER-4.2 (T16b): the surface normal is the plain mean of the two
    // wave texels' normals, not normalised: (0.6, 0.5, 1.0) and (0.5, 0.6,
    // 1.0) give (0.1, 1.0, 0.1) in the game frame (x from red, up from blue,
    // z from green), length 1.01; the shader's wave_normal and mean, done
    // here on the CPU, and the shader's own lines.
    // spec: GFX-WATER-4.2
    #[test]
    fn t16b_the_wave_normal_is_the_plain_mean() {
        let src = include_str!("../shaders/water.wesl");
        assert!(src.contains("let n = 0.5 * (n0 + n1);"));
        assert!(!src.contains("normalize(n0 + n1)"));
        assert!(src.contains("return vec3<f32>(n.x, n.z, -n.y);"));
        // wave_normal: Bevy (x, y, z) = (r, b, -g) x 2 - 1.
        let bevy = |[r, g, b]: [f32; 3]| [r * 2.0 - 1.0, b * 2.0 - 1.0, -(g * 2.0 - 1.0)];
        let (n0, n1) = (bevy([0.6, 0.5, 1.0]), bevy([0.5, 0.6, 1.0]));
        let mean: [f32; 3] = std::array::from_fn(|i| 0.5 * (n0[i] + n1[i]));
        let game = [mean[0], mean[1], -mean[2]];
        for (got, want) in game.iter().zip([0.1, 1.0, 0.1]) {
            assert!((got - want).abs() < 1e-6, "{game:?}");
        }
    }

    // GFX-WATER-4.5a (T19): a surface pixel with colour (0.6, 0.7, 0.8) and
    // alpha 0.3 after 4.3 / 4.4, highlight S = 0.1. (i) lit: colour 0.64,
    // 0.73, 0.82, alpha 0.335; (ii) in a shadow at noon (s = 0.1): x 0.9
    // first, colour 0.586, 0.667, 0.748, alpha 0.305; (iii) in a shadow at
    // hour 0 (s = 0.8): as (i); (iv) shadows off (the lookup gives lit): as
    // (i). The map border's material in (ii): as (i). The shader's tail done
    // here on the CPU, and the shader's own lines and their order.
    // spec: GFX-WATER-4.5a
    #[test]
    fn t19_the_water_takes_shadows() {
        let src = include_str!("../shaders/water.wesl");
        let fragment = &src[src.find("fn fragment").unwrap()..];
        assert!(fragment.contains("if params.opacity.z > 0.5 {"));
        assert!(fragment.contains("let shadow = mix(params.light_dir.w, 1.0, shadow_lit(tap));"));
        assert!(fragment.contains("let factor = saturate(shadow + 0.8);"));
        // (d): the depth factor, then the shadow on colour and alpha, then
        // the highlight, then the fog of war.
        let at = |line: &str| fragment.find(line).unwrap_or_else(|| panic!("{line}"));
        assert!(at("* saturate(1.2 - depth);") < at("colour *= factor;"));
        assert!(at("alpha *= factor;") < at("colour += s * (1.0 - colour);"));
        assert!(at("alpha += 0.35 * s;") < at("colour *= visible;"));
        // (b): one point-sampled tap, no percentage-closer filter; (a) only
        // while the frame draws shadows.
        let tap = &src[src.find("fn shadow_lit").unwrap()..src.find("@fragment").unwrap()];
        assert!(tap.contains("textureLoad(directional_shadow_textures, texel, layer, 0)"));
        assert!(!tap.contains("fetch_directional_shadow"));
        assert!(!tap.contains("textureSampleCompare"));
        assert!(tap.contains("DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) == 0u"));

        // The tail as the shader runs it: lit 1 or 0, s, the surface's test.
        let tail = |lit: f32, s: f32, test: bool| {
            let (mut colour, mut alpha) = ([0.6f32, 0.7, 0.8], 0.3f32);
            if test {
                let shadow = s + (1.0 - s) * lit;
                let factor = (shadow + 0.8).clamp(0.0, 1.0);
                colour = colour.map(|c| c * factor);
                alpha *= factor;
            }
            let h = 0.1;
            (colour.map(|c| c + h * (1.0 - c)), alpha + 0.35 * h)
        };
        let close = |(c, a): ([f32; 3], f32), want: [f32; 4]| {
            for (got, want) in c.iter().chain([a].iter()).zip(want) {
                assert!((got - want).abs() < 5e-4, "{c:?} {a} against {want:?}");
            }
        };
        let lit = [0.64, 0.73, 0.82, 0.335];
        close(tail(1.0, 0.1, true), lit);
        close(tail(0.0, 0.1, true), [0.586, 0.667, 0.748, 0.305]);
        close(tail(0.0, 0.8, true), lit);
        close(tail(0.0, 0.1, false), lit);
    }

    // GFX-WATER-4.5a (b), T21 (Q-1039 answered): the default camera at tick
    // 1500, eye (-331.05, 67.02, -222.47), the water point under the
    // picture's centre p = (-331.0, -172.1), N_x = 0.1, N_z = 0.2,
    // reflectionNoise 1.2: c = eye + (63.0, 19.9) = (-268.05, -202.57), D =
    // (0.160, 0.164) + (0.476, -0.230) = (0.64, -0.07), the tap at (-330.4,
    // -172.2); with N_x = N_z = 0 the point itself. The shader's function
    // done here on the CPU in Bevy's frame, and the shader's constants and
    // call.
    // spec: GFX-WATER-4.5a
    #[test]
    fn t21_the_water_takes_its_shadow_at_the_shifted_point() {
        use super::REFLECTION_NOISE;
        let src = include_str!("../shaders/water.wesl");
        assert!(src.contains("const SHADOW_SHIFT_X: vec2<f32> = vec2<f32>(1.33, 1.37);"));
        assert!(src.contains("const SHADOW_SHIFT_K: f32 = 0.0315;"));
        assert!(src.contains("const SHADOW_SHIFT_C: vec2<f32> = vec2<f32>(63.0, 19.9);"));
        let fragment = &src[src.find("fn fragment").unwrap()..];
        assert!(fragment.contains(
            "let tap = shadow_point(in.world_position, view.world_position.xyz,\n            vec2<f32>(n.x, -n.z), params.mirror.zw);"
        ));
        assert!(!src.contains("Q-1039"));
        // The shader's `shadow_point`, Bevy frame in and out.
        let shift = |p: [f32; 3], eye: [f32; 3], n: [f32; 2], r: [f32; 2]| {
            let (px, pz) = (p[0], -p[2]);
            let (cx, cz) = (eye[0] + 63.0, -eye[2] + 19.9);
            let dx = r[0] * n[0] * 1.33 + r[1] * n[1] * 0.0315 * (cx - px);
            let dz = r[0] * n[0] * 1.37 + r[1] * n[1] * 0.0315 * (cz - pz);
            [p[0] + dx, p[1], p[2] - dz]
        };
        let bevy = |[x, y, z]: [f32; 3]| [x, y, -z];
        let (eye, p) = (bevy([-331.05, 67.02, -222.47]), bevy([-331.0, 3.3, -172.1]));
        let got = bevy(shift(p, eye, [0.1, 0.2], REFLECTION_NOISE));
        assert!(
            (got[0] + 330.36).abs() < 0.01 && (got[2] + 172.17).abs() < 0.01,
            "{got:?}"
        );
        assert_eq!(shift(p, eye, [0.0, 0.0], REFLECTION_NOISE), p);
    }

    // spec: none (hook W18: no foam by default, a project's foam passed on)
    #[test]
    fn the_shore_foam_is_off_by_default() {
        use super::*;
        assert_eq!(foam_params(None), (Vec4::ZERO, Vec4::ZERO));
        let foam = crate::extend::ShoreFoam {
            reach: 300.0,
            colour: [0.9, 0.95, 1.0],
            strength: 0.6,
        };
        assert_eq!(
            foam_params(Some(foam)),
            (
                Vec4::new(0.9, 0.95, 1.0, 0.6),
                Vec4::new(300.0, 0.0, 0.0, 0.0)
            )
        );
        let none = crate::extend::ShoreFoam {
            strength: 0.0,
            ..foam
        };
        assert_eq!(foam_params(Some(none)), (Vec4::ZERO, Vec4::ZERO));
    }

    // GFX-WATER-4.5a (a), 6.1: the map border's water is the surface's
    // material with the shadow test off; everything else the same.
    // spec: GFX-WATER-4.5a, GFX-WATER-6.2
    #[test]
    fn the_map_border_takes_no_shadows() {
        use super::*;
        let surface = WaterMaterial {
            params: WaterParams {
                water_color: Vec4::ONE,
                wave_color: Vec4::ONE,
                opacity: Vec4::new(0.85, 0.94, 1.0, 0.0),
                rotation: Vec4::ZERO,
                drift: Vec4::ZERO,
                world_rect: Vec4::ZERO,
                light_dir: Vec4::ZERO,
                light_color: Vec4::ONE,
                ambient: Vec4::ZERO,
                mirror: Vec4::ZERO,
                foam: Vec4::ZERO,
                foam_reach: Vec4::ZERO,
            },
            wave0: Handle::default(),
            wave1: Handle::default(),
            world: Handle::default(),
            mirror: Handle::default(),
        };
        let border = crate::render::view3d::skirt_material(surface.clone());
        assert_eq!(border.params.opacity, Vec4::new(0.85, 0.94, 0.0, 0.0));
        assert_eq!(border.world, surface.world);
        assert_eq!(border.params.light_dir, surface.params.light_dir);
    }

    // GFX-VIEW-2.1a: the terrain shader takes the vertex's decoded tangent
    // and normal as they come (no tangent made from the normal, no
    // renormalising), the bitangent cross(n, t) (T x N of the game frame,
    // `terrain3d::tests::the_shaders_bitangent_is_t_cross_n`), and lights
    // the normal map with the light turned into (T, B, N) and normalised.
    // spec: GFX-VIEW-2.1a
    #[test]
    fn the_terrain_shader_takes_the_vertex_frame() {
        let src = include_str!("../shaders/terrain.wesl");
        assert!(src.contains("@location(3) tangent: vec4<f32>,"));
        assert!(src.contains("let n = in.normal;"));
        assert!(src.contains("let t = in.tangent;"));
        assert!(src.contains("let b = cross(n, t);"));
        assert!(src.contains(
            "let l_ts = normalize(vec3<f32>(dot(to_light, t), dot(to_light, b), dot(to_light, n)));"
        ));
        assert!(src.contains("saturate(dot(n_ts, l_ts))"));
        let rust = include_str!("materials.rs");
        let terrain = &rust[rust.find("impl Material for TerrainMaterial").unwrap()..];
        let layout = &terrain[..terrain.find("Ok(())").unwrap()];
        assert!(layout.contains("Mesh::ATTRIBUTE_TANGENT.at_shader_location(3)"));
    }
}
