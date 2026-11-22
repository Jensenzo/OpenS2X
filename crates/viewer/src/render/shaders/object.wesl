// Models standing on the map: buildings, construction sites, trees, stones,
// doodads, flags, border markers, settlers, animals, wares and ships, each
// shaded by its family (specs/GFX-objects.md GFX-OBJ-2, 3): one directional
// light with its shadow map (GFX-VIEW-2.5) and ambient, the family's colouring,
// and the field's visibility (GFX-VIEW-5.1). Colours are display values and
// the shader writes the stored value itself; see-through layers blend
// straight on those values (GFX-VIEW-2.6 c1, `stored.rs`).

import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::view_transformations::position_world_to_clip;
import bevy_pbr::render::mesh_view_bindings::{globals, lights, view};
import bevy_pbr::render::shadows;
@if(SCREEN_SPACE_AMBIENT_OCCLUSION)
import bevy_pbr::render::mesh_view_bindings::screen_space_ambient_occlusion_texture;

const PLAIN: u32 = 0u;
const SETTLER: u32 = 1u;
const HOUSE: u32 = 2u;
const TREE: u32 = 3u;
const DOODAD: u32 = 4u;
const HAZE: u32 = 5u;
const STONE: u32 = 6u;
const WHEAT: u32 = 7u;
const SHIP: u32 = 8u;
const FLAG: u32 = 9u;
const BORDER: u32 = 10u;
const ANIMAL: u32 = 11u;
const ROAD: u32 = 12u;
const DECAL: u32 = 13u;
const SKY: u32 = 14u;
// Hook W30: a project's model and its ground (`models3d/project.rs`).
const FACET: u32 = 15u;
const FACET_GROUND: u32 = 16u;
// Hook W30b: a project's tree.
const FACET_TREE: u32 = 17u;
// Hook W30c: a project's map object (a project's tree without the sway).
const FACET_OBJECT: u32 = 18u;
// Hook W30b: a project's building at night is darkened as the original's
// houses are by their light map (`project::NIGHT_LIGHT_MAP`), by how much
// of the night it is: the light's and ambient's luminances added, 1.4377
// at noon, 0.5993 at night (`project::night_amount`).
const NIGHT_LIGHT_MAP: f32 = 0.533;
const NIGHT_DAY: f32 = 1.4377;
const NIGHT_DARK: f32 = 0.5993;

struct ObjectParams {
    // Multiplies the texture: white, or a player colour.
    tint: vec4<f32>,
    world_rect: vec4<f32>,
    // x: the family.
    mode: vec4<f32>,
    // Settler: the palette atlas block (offset u, v; scale u, v). House:
    // x the colour-map offset. Ship: x the player colour weight.
    extra: vec4<f32>,
    // The owner's player colour.
    player: vec4<f32>,
};

@group(3) @binding(0) var<uniform> params: ObjectParams;
@group(3) @binding(1) var base: texture_2d<f32>;
@group(3) @binding(2) var base_sampler: sampler;
@group(3) @binding(3) var world: texture_2d<f32>;
@group(3) @binding(4) var world_sampler: sampler;

// GFX-ANIM-8.2: the mesh's bone palette, 50 matrices (GFX-ANIM-8.3);
// a rigid part has the identity in slot 0.
// F51: every material's palette in one buffer, this one's from matrix
// params.mode.y.
@group(3) @binding(5) var<storage, read> palettes: array<mat4x4<f32>>;
@group(3) @binding(6) var second: texture_2d<f32>;
@group(3) @binding(7) var second_sampler: sampler;
@group(3) @binding(8) var third: texture_2d<f32>;
@group(3) @binding(9) var third_sampler: sampler;

// GFX-VIEW-2.3, 2.3c: the day light, one buffer for every object: the
// direction (w: the shadow factor), the light colour and the ambient (w:
// their luminances).
struct SceneLight {
    light_dir: vec4<f32>,
    light_color: vec4<f32>,
    ambient: vec4<f32>,
};
@group(3) @binding(10) var<storage, read> light: SceneLight;

// Hook W30b: how much of the night it is for a project's building.
fn night_amount() -> f32 {
    return saturate((NIGHT_DAY - (light.light_color.w + light.ambient.w)) / (NIGHT_DAY - NIGHT_DARK));
}

fn family() -> u32 {
    return u32(params.mode.x + 0.5);
}

// GFX-VIEW-6.3: texture coordinates turned by a: u' = u cos a - v sin a,
// v' = u sin a + v cos a.
fn sky_turn(p: vec2<f32>, a: f32) -> vec2<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);
}

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) bone_index: vec4<u32>,
    @location(4) bone_weight: vec4<f32>,
    @location(5) uv1: vec2<f32>,
    @location(6) uv2: vec2<f32>,
    @location(7) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) uv1: vec2<f32>,
    @location(4) uv2: vec2<f32>,
    @location(5) color: vec4<f32>,
    // The instance's y scale (a tree's or field's growth).
    @location(6) m22: f32,
    // The normal the light uses: the world normal, except in the house
    // pass (GFX-OBJ-2.1, T31).
    @location(7) light_normal: vec3<f32>,
};

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    var out: VertexOutput;
    // GFX-ANIM-8.1: four weighted palette matrices per vertex.
    let w = v.bone_weight;
    let o = u32(params.mode.y + 0.5);
    let skin = palettes[o + v.bone_index.x] * w.x + palettes[o + v.bone_index.y] * w.y
        + palettes[o + v.bone_index.z] * w.z + palettes[o + v.bone_index.w] * w.w;
    var local = skin * vec4<f32>(v.position, 1.0);
    let normal = (skin * vec4<f32>(v.normal, 0.0)).xyz;
    let world_from_local = mesh_functions::get_world_from_local(v.instance_index);
    // The instance's translation in the game frame (z is Bevy's negated)
    // and its y scale.
    let tx = world_from_local[3].x;
    let tz = -world_from_local[3].z;
    let m22 = length(world_from_local[1].xyz);
    let t = globals.time;
    let f = family();
    if f == TREE || f == FACET_TREE {
        // GFX-OBJ-2.4: the tops sway (hook W30b: a project's tree too).
        let p = local.xyz;
        let s = saturate(dot(p, p) * 0.003) * m22;
        local.x += s * sin(0.35 * 3.14159265 * tx + t);
        local.z -= s * cos(0.35 * 3.14159265 * tz + t);
    } else if f == FLAG {
        // GFX-OBJ-2.9: the cloth waves; the pole at the origin stays.
        let p = vec3<f32>(local.x, local.y, -local.z);
        let s = 0.33 * saturate(-0.02 + p.x * p.x + p.z * p.z);
        let w = tx + tz + p.x + p.y + t;
        let c = 0.5 + 0.5 * cos(4.0 * w);
        local.z -= s * c;
        local.x += s * (1.0 - c);
    }
    let world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local, vec4<f32>(local.xyz, 1.0));
    out.world_position = world_position.xyz;
    out.clip = position_world_to_clip(world_position.xyz);
    out.normal = mesh_functions::mesh_normal_local_to_world(normal, v.instance_index);
    out.light_normal = out.normal;
    if f == HOUSE && params.extra.w < 1.0 {
        // GFX-OBJ-2.1 (T31): the house pass (not the site's frame pass,
        // extra.w = 1 + f) lights the normal turned by its bone matrix
        // only: the model's own frame after the load-time conversion, not
        // turned by the building's heading. A building's parts have no
        // transform of their own and its scale is 1, so that is the
        // skinned normal.
        out.light_normal = normal;
    }
    out.uv = v.uv;
    out.uv1 = v.uv1;
    out.uv2 = v.uv2;
    out.color = v.color;
    if f == HOUSE && params.extra.w >= 1.0 {
        // GFX-OBJ-2.6a (T37), 2.2a (6): the site's frame pieces take N . L
        // per vertex, from the mesh normal under the building's matrix (the
        // world normal); its x goes to the fragment in place of the
        // vertex colour, which the frame pass does not read.
        let lambert = saturate(dot(normalize(out.normal), -light.light_dir.xyz));
        out.color = vec4<f32>(lambert, 0.0, 0.0, 1.0);
    }
    out.m22 = m22;
    return out;
}

// Hook W17b: a project's screen-space ambient occlusion at the pixel `frag`
// (its value in Bevy's occlusion texture, 1 unoccluded); 1 on every
// camera without it and for blended draws (the original's look, the
// water's mirror, decals and roads).
// Bevy's occlusion on a flat surface (High quality: 1 - 1/6, view3d's
// `AO_FLAT`): divided out, so open ground reads 1.
const AO_FLAT: f32 = 0.8333333;

fn ambient_occlusion(frag: vec4<f32>) -> f32 {
    var ao = 1.0;
    @if(SCREEN_SPACE_AMBIENT_OCCLUSION) {
        let p = vec2<i32>(frag.xy - view.viewport.xy);
        ao = min(textureLoad(screen_space_ambient_occlusion_texture, p, 0i).r / AO_FLAT, 1.0);
    }
    return ao;
}

// GFX-VIEW-2.3, 2.5: the factor of the direct light at a point: 1 where
// lit, the shadow factor (light_dir.w) in a shadow.
fn shadow_at(world_position: vec3<f32>, normal: vec3<f32>, frag: vec4<f32>) -> f32 {
    if lights.n_directional_lights == 0u {
        return 1.0;
    }
    let p = vec4<f32>(world_position, 1.0);
    let view_z = dot(vec4<f32>(view.view_from_world[0].z, view.view_from_world[1].z,
        view.view_from_world[2].z, view.view_from_world[3].z), p);
    let lit = shadows::fetch_directional_shadow(0u, p, normal, view_z, frag.xy);
    return mix(light.light_dir.w, 1.0, lit);
}


// Hook W30, W30b, W30c: a project model's flat face, nearly edge-on to the
// sun, tested against the shadow map at its own depth (the constant bias
// only, no normal offset, GFX-VIEW-2.5 c) shades itself in stripes one map
// texel wide (shadow acne). A face of a project's model is tested 1.5 map
// texels of its cascade out along its own normal instead (the light's
// normal offset that the original's houses go without).
const FACET_NORMAL_OFFSET: f32 = 1.5;

fn facet_shadow_point(world_position: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    if lights.n_directional_lights == 0u {
        return world_position;
    }
    let p = vec4<f32>(world_position, 1.0);
    let view_z = dot(vec4<f32>(view.view_from_world[0].z, view.view_from_world[1].z,
        view.view_from_world[2].z, view.view_from_world[3].z), p);
    let index = shadows::get_cascade_index(0u, view_z);
    let light = &lights.directional_lights[0];
    if index >= (*light).num_cascades {
        return world_position;
    }
    let texel = (*light).cascades[index].texel_size;
    return world_position + FACET_NORMAL_OFFSET * texel * normal;
}

// GFX-OBJ-3.3: a settler's pixel. The colour-index map gives the
// material row (red, the index as a byte), the luminance (green), the
// specular mask (blue) and where the player colour shows (alpha 0). The
// lit luminance picks the palette column of the row in the atlas block.
// D = saturate(N . L) x lum(light) x shadow + lum(ambient), the
// luminances in the fourth values; N the interpolated vertex normal as it
// is, not renormalised (T41).
fn settler(in: VertexOutput, n: vec3<f32>, raw_n: vec3<f32>, visible: f32, shadow: f32,
        occlusion: f32) -> vec4<f32> {
    let texel = textureSampleLevel(base, base_sampler, in.uv, 0.0);
    let d = saturate(dot(raw_n, -light.light_dir.xyz)) * light.light_color.w * shadow
        + light.ambient.w * occlusion;
    let half_texel = 0.5 / 1024.0;
    let u = saturate(texel.g * d) * params.extra.z + params.extra.x + half_texel;
    let v = texel.r * params.extra.w + params.extra.y + half_texel;
    var colour = textureSampleLevel(second, second_sampler, vec2<f32>(u, v), 0.0).rgb;
    colour = mix(params.player.rgb * texel.g * d * 2.0, colour, texel.a);
    // GFX-OBJ-3.3 (T41): the highlight's direction R = normalize(2 D N +
    // L), N the renormalised normal, L the light's way down, D the scalar
    // above (not a mirror reflection of the light); added and clamped
    // before the fog of war.
    let to_eye = normalize(view.world_position - in.world_position);
    let r = normalize(2.0 * d * n + light.light_dir.xyz);
    colour = saturate(colour + vec3<f32>(pow(saturate(dot(r, to_eye)), 16.0) * texel.b));
    return vec4<f32>(colour * visible, 1.0);
}

// GFX-WATER-2.6a: the mirror camera (`view3d/mirror.rs`) is the only one
// whose transform holds a reflection (a negative determinant).
fn mirror_view() -> bool {
    let m = view.world_from_view;
    return dot(cross(m[0].xyz, m[1].xyz), m[2].xyz) < 0.0;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    // Hook W30b: the normal of the face as drawn (the screen derivatives of
    // its world position), taken before anything is discarded: a project
    // tree's face stays one plane with one normal while the wind moves its
    // corners.
    let face_normal = cross(dpdx(in.world_position), dpdy(in.world_position));
    // GFX-WATER-2.6a: the mirror image's clip plane at the surface's level
    // (-0.1, GFX-WATER-3.3) keeps the eye's side, above it.
    let mirrored = mirror_view();
    if mirrored && in.world_position.y < -0.1 {
        discard;
    }
    let f = family();
    if f == SKY {
        // GFX-VIEW-6.3 (T44): the sky cap: vertex colour x lerp(T0, T1, b),
        // each texture turned by its skydome entry's rotation (extra x, y),
        // b in extra z; unlit, no fog of war, no shadow.
        let t0 = textureSample(base, base_sampler, sky_turn(in.uv, params.extra.x));
        let t1 = textureSample(second, second_sampler, sky_turn(in.uv, params.extra.y));
        return in.color * mix(t0, t1, params.extra.z);
    }
    let g = vec2<f32>(in.world_position.x, -in.world_position.z);
    let rect = params.world_rect;
    let world_uv = (g - rect.xy) / (rect.zw - rect.xy);
    let seen = textureSampleLevel(world, world_sampler, world_uv, 0.0).r;
    // Two-sided: leaves and cloth are seen from both sides. The ground
    // decal keeps its mesh's own up normal whichever way its triangles
    // wind (GFX-OBJ-2.3a d).
    var n = normalize(in.normal);
    var raw_n = in.normal;
    var ln = normalize(in.light_normal);
    // GFX-WATER-2.6a 3: the reflection reverses the winding.
    if front == mirrored && f != DECAL && f != FACET_GROUND {
        n = -n;
        raw_n = -raw_n;
        ln = -ln;
    }
    if f == FACET_TREE || f == FACET_OBJECT {
        // Hook W30b: the moved face's own normal, turned to the eye (both
        // sides drawn, as the file's normals are); W30c: a map object's
        // the same (it does not move).
        let to_eye = view.world_position - in.world_position;
        n = normalize(face_normal);
        if dot(n, to_eye) < 0.0 {
            n = -n;
        }
        raw_n = n;
        ln = n;
    }
    // GFX-VIEW-2.5 b, GFX-OBJ-2.3a d (Run D): every object receives
    // shadows, the ground decal too.
    var shadow_point = in.world_position;
    if f == FACET || f == FACET_TREE || f == FACET_OBJECT {
        shadow_point = facet_shadow_point(in.world_position, n);
    }
    let shadow = shadow_at(shadow_point, n, in.clip);
    // Hook W17b: the ambient light times the screen-space ambient occlusion
    // at the pixel (1 without the hook: the ambient as it is).
    let occlusion = ambient_occlusion(in.clip);
    let ambient = light.ambient.rgb * occlusion;
    if f == SETTLER {
        return settler(in, n, raw_n, seen, shadow, occlusion);
    }
    // GFX-OBJ-8.2: a border flag's texture shifted down by its row.
    var uv = in.uv;
    if f == FLAG {
        uv.y += params.extra.y;
    }
    let tex = textureSample(base, base_sampler, uv) * params.tint;
    let to_light = -light.light_dir.xyz;
    if f == ROAD {
        // GFX-OBJ-6.3: lit per pixel through the normal map (tangent along
        // the street), blended by the texture's alpha.
        let tangent = normalize(vec3<f32>(in.uv1.x, 0.0, in.uv1.y));
        let bitangent = cross(n, tangent);
        let bump = textureSample(second, second_sampler, in.uv).rgb * 2.0 - 1.0;
        let nn = normalize(tangent * bump.x + bitangent * bump.y + n * bump.z);
        let lit = light.light_color.rgb * saturate(dot(nn, to_light)) * shadow
            + ambient;
        return vec4<f32>(saturate(lit * tex.rgb * seen), tex.a);
    }
    // GFX-OBJ-2.1 (T31): a house's light term with its own normal.
    let lambert = saturate(dot(ln, to_light));
    let d = light.light_color.rgb * lambert * shadow + ambient;
    if f == DECAL {
        // GFX-OBJ-2.3, 2.3a d: the ground decal, (ambient + N.L x light x
        // shadow) x texture with its own up normal, blended by its alpha.
        return vec4<f32>(saturate(d * tex.rgb * seen), tex.a);
    }
    if f == FACET_GROUND {
        // Hook W30: a project model's ground, laid on the terrain as the
        // decal: (ambient + N.L x light x shadow) x palette x vertex colour,
        // blended by the vertex alpha (its fade).
        return vec4<f32>(saturate(d * tex.rgb * in.color.rgb * seen), in.color.a);
    }
    let player = params.player.rgb * 1.8;
    var rgb = d * tex.rgb;
    var alpha_test = true;
    if f == HOUSE {
        // GFX-OBJ-2.6: a house being built is cut by a horizontal plane at
        // world height extra.y.
        if params.extra.z > 1.5 {
            // GFX-OBJ-8.4: the old house of an upgrade shows above it.
            if in.world_position.y < params.extra.y {
                discard;
            }
        } else if params.extra.z > 0.5 && in.world_position.y > params.extra.y {
            discard;
        }
        if params.extra.w >= 1.0 {
            // GFX-OBJ-2.6a (T37), 2.2a (6): the site's frame pass: (ambient +
            // light x N . L per vertex x shadow) x texel, nothing else: no
            // colour column, no light map, opaque (no alpha test).
            let frame = light.light_color.rgb * in.color.x * shadow + ambient;
            rgb = frame * tex.rgb;
            alpha_test = false;
        } else {
            // GFX-OBJ-2.1: colour x colour map; the light map shows in the
            // dark. GFX-OBJ-2.2b (T38): the third set plus the column offset
            // plus half a texel, bilinear with wrap (the colour map's
            // sampler), its top level.
            let half = vec2<f32>(0.5) / vec2<f32>(textureDimensions(second, 0));
            let cuv = vec2<f32>(in.uv2.x + params.extra.x, in.uv2.y) + half;
            let colour_map = textureSampleLevel(second, second_sampler, cuv, 0.0).rgb;
            let light_map = textureSample(third, third_sampler, in.uv1).rgb;
            let result = d * tex.rgb * colour_map;
            rgb = mix(result, result * light_map, saturate(vec3<f32>(1.5) - d));
        }
    } else if f == FACET || f == FACET_TREE || f == FACET_OBJECT {
        // Hook W30: a project's model. Cut by the rising plane as a house
        // (GFX-OBJ-2.6b, 8.4). Colour: palette x vertex colour (the face's
        // occlusion), times the owner's colour on the team part (mode.z).
        // W30b: a project's tree the same, never cut; W30c: a map object
        // as a tree.
        if f == FACET && params.extra.z > 1.5 {
            if in.world_position.y < params.extra.y {
                discard;
            }
        } else if f == FACET && params.extra.z > 0.5 && in.world_position.y > params.extra.y {
            discard;
        }
        var albedo = tex.rgb * in.color.rgb;
        albedo = mix(albedo, albedo * params.player.rgb, params.mode.z);
        // Roughness (green) and metallic (blue) of the second image: a
        // highlight of the sun that is small and bright on smooth faces,
        // none on rough ones, in the face's colour on metal.
        let orm = textureSample(third, third_sampler, in.uv).rgb;
        let rough = orm.g;
        let metal = orm.b;
        let to_eye = normalize(view.world_position - in.world_position);
        let h = normalize(to_light + to_eye);
        let lit = select(0.0, 1.0, lambert > 0.0);
        let spec = pow(saturate(dot(n, h)), mix(64.0, 4.0, rough)) * (1.0 - rough)
            * mix(0.04, 1.0, metal) * lit * shadow;
        rgb = d * albedo * (1.0 - 0.5 * metal)
            + light.light_color.rgb * spec * mix(vec3<f32>(1.0), albedo, metal);
        if f == FACET {
            // W30b: a building at night, as by the original's light maps.
            rgb = mix(rgb, rgb * NIGHT_LIGHT_MAP, night_amount());
        }
        alpha_test = false;
    } else if f == TREE {
        // GFX-OBJ-2.4 (GFX-VIEW-2.3d f): the mesh's vertex colour scales the
        // direct light only, the ambient stays whole
        // (`world3d::DayLight::tree_d`).
        let direct = light.light_color.rgb * lambert * in.color.rgb * shadow;
        rgb = (direct + ambient) * tex.rgb;
    } else if f == HAZE {
        // GFX-OBJ-2.5: the underwater haze by world height.
        let a = saturate((in.world_position.y + 7.05) / 7.0);
        let haze = mix(vec3<f32>(0.0, 0.0, 0.05), vec3<f32>(0.0, 0.8, 0.3), a - 0.3);
        rgb = mix(haze, rgb, a);
    } else if f == STONE {
        // GFX-OBJ-2.6: per-pixel light and a gloss in the texture's alpha.
        let to_eye = normalize(view.world_position - in.world_position);
        let r = normalize(2.0 * d * n + light.light_dir.xyz);
        let gloss = pow(saturate(dot(r, to_eye)), 16.0);
        rgb = d * tex.rgb + gloss * tex.a * tex.rgb;
        alpha_test = false;
    } else if f == WHEAT {
        // GFX-OBJ-2.7: young fields are strongly green.
        let young = mix(vec3<f32>(0.5, 8.0, 0.2), vec3<f32>(1.0), in.m22);
        rgb = young * (light.light_color.rgb - light.ambient.rgb) * tex.rgb;
    } else if f == SHIP {
        // GFX-OBJ-2.8: the player colour where the texture's alpha is,
        // by the weight. GFX-OBJ-2.8a: the weight is the part's number:
        // the hull (0) untinted with the alpha test at 127, the sails and
        // banner (1) tinted, nothing discarded.
        let c = tex.rgb * d;
        let p = c * mix(vec3<f32>(1.0), player, tex.a);
        rgb = mix(c, p, params.extra.x);
        alpha_test = params.extra.x < 0.5;
    } else if f == FLAG {
        // GFX-OBJ-2.9: the owner's colour where the cloth is transparent.
        rgb = mix(player, tex.rgb, tex.a) * d;
        alpha_test = false;
    } else if f == BORDER {
        // GFX-OBJ-2.10. Its alpha test at 127 would drop the player-
        // coloured part (alpha 0) and leave a stub in the ground; the
        // original shows the whole marker in the owner's colour (play-test
        // 4, Q-207): no alpha test, as for the flag.
        rgb = d * mix(player, tex.rgb, tex.a);
        alpha_test = false;
    } else if f == ANIMAL {
        alpha_test = false;
    }
    if alpha_test && tex.a <= 127.0 / 255.0 {
        discard;
    }
    return vec4<f32>(saturate(rgb * seen), 1.0);
}
