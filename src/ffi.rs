//! Flat C-ABI WASM FFI exposing simulation stepping, 7 environmental fields + walls,
//! Cell Lab 8-mode organism designer, custom mutagenesis forge, 3-strategy virology lab, and Brain X-Ray inspector.

use crate::genome::CellRole;
use crate::world::{
    World, BOND_STRIDE, CELL_STRIDE, GRID_H, GRID_W, INSPECTOR_LEN, MUTATION_LOG_STRIDE, STATS_LEN,
    VIRION_STRIDE, WORLD_H, WORLD_W,
};

#[no_mangle]
pub extern "C" fn sim_create(seed: u32, scenario: u8) -> *mut World {
    Box::into_raw(Box::new(World::new(seed, scenario)))
}

/// # Safety
/// `ptr` must be a valid pointer returned by `sim_create` or null.
#[no_mangle]
pub unsafe extern "C" fn sim_destroy(ptr: *mut World) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// # Safety
/// `ptr` must be a valid mutable pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_reset_scenario(ptr: *mut World, scenario: u8) {
    if let Some(w) = ptr.as_mut() {
        w.reset_scenario(scenario);
    }
}

/// # Safety
/// `ptr` must be a valid mutable pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_step(ptr: *mut World, dt: f32) {
    if let Some(w) = ptr.as_mut() {
        w.step(dt);
    }
}

#[no_mangle]
pub extern "C" fn sim_world_width() -> f32 {
    WORLD_W
}

#[no_mangle]
pub extern "C" fn sim_world_height() -> f32 {
    WORLD_H
}

#[no_mangle]
pub extern "C" fn sim_grid_w() -> u32 {
    GRID_W as u32
}

#[no_mangle]
pub extern "C" fn sim_grid_h() -> u32 {
    GRID_H as u32
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_tick(ptr: *const World) -> u32 {
    ptr.as_ref().map_or(0, |w| w.tick)
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_temp_ptr(ptr: *const World) -> *const f32 {
    ptr.as_ref()
        .map_or(core::ptr::null(), |w| w.fields.temp.as_ptr())
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_light_ptr(ptr: *const World) -> *const f32 {
    ptr.as_ref()
        .map_or(core::ptr::null(), |w| w.fields.light.as_ptr())
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_nutrient_ptr(ptr: *const World) -> *const f32 {
    ptr.as_ref()
        .map_or(core::ptr::null(), |w| w.fields.nutrients.as_ptr())
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_alarm_ptr(ptr: *const World) -> *const f32 {
    ptr.as_ref()
        .map_or(core::ptr::null(), |w| w.fields.pheromone_alarm.as_ptr())
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_food_ptr(ptr: *const World) -> *const f32 {
    ptr.as_ref()
        .map_or(core::ptr::null(), |w| w.fields.pheromone_food.as_ptr())
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_toxin_ptr(ptr: *const World) -> *const f32 {
    ptr.as_ref()
        .map_or(core::ptr::null(), |w| w.fields.toxin.as_ptr())
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_mutagen_ptr(ptr: *const World) -> *const f32 {
    ptr.as_ref()
        .map_or(core::ptr::null(), |w| w.fields.mutagen.as_ptr())
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_walls_ptr(ptr: *const World) -> *const u8 {
    ptr.as_ref()
        .map_or(core::ptr::null(), |w| w.fields.walls.as_ptr())
}

/// # Safety
/// `ptr` must be a valid mutable pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_clear_walls(ptr: *mut World) {
    if let Some(w) = ptr.as_mut() {
        w.fields.clear_walls();
        w.fields.rebuild_baseline_light();
    }
}

#[no_mangle]
pub extern "C" fn sim_cell_stride() -> u32 {
    CELL_STRIDE as u32
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_cell_count(ptr: *const World) -> u32 {
    ptr.as_ref()
        .map_or(0, |w| (w.cells_buf.len() / CELL_STRIDE) as u32)
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_cells_ptr(ptr: *const World) -> *const f32 {
    ptr.as_ref()
        .map_or(core::ptr::null(), |w| w.cells_buf.as_ptr())
}

#[no_mangle]
pub extern "C" fn sim_virion_stride() -> u32 {
    VIRION_STRIDE as u32
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_virion_count(ptr: *const World) -> u32 {
    ptr.as_ref()
        .map_or(0, |w| (w.virions_buf.len() / VIRION_STRIDE) as u32)
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_virions_ptr(ptr: *const World) -> *const f32 {
    ptr.as_ref()
        .map_or(core::ptr::null(), |w| w.virions_buf.as_ptr())
}

#[no_mangle]
pub extern "C" fn sim_bond_stride() -> u32 {
    BOND_STRIDE as u32
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_bond_count(ptr: *const World) -> u32 {
    ptr.as_ref()
        .map_or(0, |w| (w.bonds_buf.len() / BOND_STRIDE) as u32)
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_bonds_ptr(ptr: *const World) -> *const f32 {
    ptr.as_ref()
        .map_or(core::ptr::null(), |w| w.bonds_buf.as_ptr())
}

#[no_mangle]
pub extern "C" fn sim_stats_len() -> u32 {
    STATS_LEN as u32
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_stats_ptr(ptr: *const World) -> *const f32 {
    ptr.as_ref()
        .map_or(core::ptr::null(), |w| w.stats_buf.as_ptr())
}

#[no_mangle]
pub extern "C" fn sim_inspector_len() -> u32 {
    INSPECTOR_LEN as u32
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_inspector_ptr(ptr: *const World) -> *const f32 {
    ptr.as_ref()
        .map_or(core::ptr::null(), |w| w.inspector_buf.as_ptr())
}

#[no_mangle]
pub extern "C" fn sim_mutation_log_stride() -> u32 {
    MUTATION_LOG_STRIDE as u32
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_mutation_log_count(ptr: *const World) -> u32 {
    ptr.as_ref().map_or(0, |w| w.mutation_log.len() as u32)
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_mutation_log_ptr(ptr: *const World) -> *const f32 {
    ptr.as_ref()
        .map_or(core::ptr::null(), |w| w.mutation_log_buf.as_ptr())
}

/// # Safety
/// `ptr` must be a valid mutable pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_register_genome(
    ptr: *mut World,
    preset_id: u8,
    active_modes: u8,
    initial_mode: u8,
) -> u32 {
    if let Some(w) = ptr.as_mut() {
        w.register_genome(preset_id, active_modes, initial_mode)
    } else {
        0
    }
}

/// # Safety
/// `ptr` must be a valid mutable pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_load_preset_genome(
    ptr: *mut World,
    genome_id: u32,
    preset_id: u8,
) -> u32 {
    if let Some(w) = ptr.as_mut() {
        w.load_preset_genome(genome_id, preset_id)
    } else {
        0
    }
}

/// # Safety
/// `ptr` must be a valid mutable pointer to `World`.
#[allow(clippy::too_many_arguments)]
#[no_mangle]
pub unsafe extern "C" fn sim_set_genome_mode(
    ptr: *mut World,
    genome_id: u32,
    mode_idx: u8,
    role: u8,
    child_a_mode: u8,
    child_b_mode: u8,
    split_mass: f32,
    split_angle: f32,
    keep_adhesion_parent: u8,
    bond_children: u8,
    adhesion_stiffness: f32,
    max_bonds: u8,
    t_opt: f32,
    t_tol: f32,
    metabolism: f32,
    membrane_armor: f32,
    immune_resistance: f32,
    vascular_share: f32,
    secretion_kind: u8,
    sensory_range: f32,
    hebbian_rate: f32,
    mutation_rate: f32,
    hue_shift: f32,
) -> u32 {
    if let Some(w) = ptr.as_mut() {
        w.set_genome_mode(
            genome_id,
            mode_idx,
            role,
            child_a_mode,
            child_b_mode,
            split_mass,
            split_angle,
            keep_adhesion_parent != 0,
            bond_children != 0,
            adhesion_stiffness,
            max_bonds,
            t_opt,
            t_tol,
            metabolism,
            membrane_armor,
            immune_resistance,
            vascular_share,
            secretion_kind,
            sensory_range,
            hebbian_rate,
            mutation_rate,
            hue_shift,
        )
    } else {
        0
    }
}

/// # Safety
/// `ptr` must be a valid mutable pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_get_genome_mode_ptr(
    ptr: *mut World,
    genome_id: u32,
    mode_idx: u8,
) -> *const f32 {
    if let Some(w) = ptr.as_mut() {
        w.pack_genome_mode(genome_id, mode_idx)
    } else {
        core::ptr::null()
    }
}

/// # Safety
/// `ptr` must be a valid mutable pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_spawn_organism(
    ptr: *mut World,
    genome_id: u32,
    x: f32,
    y: f32,
    count: u32,
    spread: f32,
) -> u32 {
    if let Some(w) = ptr.as_mut() {
        w.spawn_organism(genome_id, x, y, count, spread)
    } else {
        0
    }
}

/// # Safety
/// `ptr` must be a valid mutable pointer to `World`.
#[allow(clippy::too_many_arguments)]
#[no_mangle]
pub unsafe extern "C" fn sim_spawn_custom_cell(
    ptr: *mut World,
    x: f32,
    y: f32,
    role: u8,
    armor: f32,
    metabolism: f32,
    t_opt: f32,
    t_tol: f32,
    adhesion: f32,
    mutation_rate: f32,
    immune_resistance: f32,
    neural_bias: f32,
    count: u32,
    spread: f32,
) -> u32 {
    if let Some(w) = ptr.as_mut() {
        w.spawn_custom_cell(
            x,
            y,
            CellRole::from_u8(role),
            armor,
            metabolism,
            t_opt,
            t_tol,
            adhesion,
            mutation_rate,
            immune_resistance,
            neural_bias,
            count,
            spread,
        )
    } else {
        0
    }
}

/// # Safety
/// `ptr` must be a valid mutable pointer to `World`.
#[allow(clippy::too_many_arguments)]
#[no_mangle]
pub unsafe extern "C" fn sim_upsert_virus_strain(
    ptr: *mut World,
    strain_id: u32,
    strategy: u8,
    target_role_mask: u32,
    infectivity: f32,
    incubation_ticks: f32,
    burst_count: u8,
    lethality: f32,
    mutation_rate: f32,
    immune_evasion: f32,
    thermal_resistance: f32,
    enveloped: u8,
    hue: f32,
) -> u32 {
    if let Some(w) = ptr.as_mut() {
        w.upsert_virus_strain(
            strain_id,
            strategy,
            target_role_mask as u16,
            infectivity,
            incubation_ticks,
            burst_count,
            lethality,
            mutation_rate,
            immune_evasion,
            thermal_resistance,
            enveloped != 0,
            hue,
        )
    } else {
        0
    }
}

/// # Safety
/// `ptr` must be a valid mutable pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_inject_strain(
    ptr: *mut World,
    strain_id: u32,
    x: f32,
    y: f32,
    count: u32,
    spread: f32,
) -> u32 {
    if let Some(w) = ptr.as_mut() {
        w.inject_strain(strain_id, x, y, count, spread)
    } else {
        0
    }
}

/// # Safety
/// `ptr` must be a valid mutable pointer to `World`.
#[allow(clippy::too_many_arguments)]
#[no_mangle]
pub unsafe extern "C" fn sim_inject_custom_virus(
    ptr: *mut World,
    x: f32,
    y: f32,
    infectivity: f32,
    incubation_ticks: f32,
    burst_count: u8,
    lethality: f32,
    mutation_rate: f32,
    immune_evasion: f32,
    count: u32,
    spread: f32,
) -> u32 {
    if let Some(w) = ptr.as_mut() {
        w.inject_custom_virus(
            x,
            y,
            infectivity,
            incubation_ticks,
            burst_count,
            lethality,
            mutation_rate,
            immune_evasion,
            count,
            spread,
        )
    } else {
        0
    }
}

/// # Safety
/// `ptr` must be a valid mutable pointer to `World`.
#[allow(clippy::too_many_arguments)]
#[no_mangle]
pub unsafe extern "C" fn sim_upsert_mutation_rule(
    ptr: *mut World,
    rule_id: u32,
    enabled: u8,
    target_mask: u32,
    bias: f32,
    strength: f32,
    probability: f32,
    trigger: u8,
) -> u32 {
    if let Some(w) = ptr.as_mut() {
        w.upsert_mutation_rule(
            rule_id,
            enabled != 0,
            target_mask,
            bias,
            strength,
            probability,
            trigger,
        )
    } else {
        0
    }
}

/// # Safety
/// `ptr` must be a valid mutable pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_clear_mutation_rules(ptr: *mut World) {
    if let Some(w) = ptr.as_mut() {
        w.clear_mutation_rules();
    }
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_mutation_rule_count(ptr: *const World) -> u32 {
    ptr.as_ref().map_or(0, |w| w.mutation_rules.len() as u32)
}

/// # Safety
/// `ptr` must be a valid mutable pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_trigger_mutation(ptr: *mut World, cell_id: u32, rule_id: u32) -> u32 {
    if let Some(w) = ptr.as_mut() {
        w.trigger_mutation(cell_id, rule_id)
    } else {
        0
    }
}

/// # Safety
/// `ptr` must be a valid mutable pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_apply_brush(
    ptr: *mut World,
    brush_kind: u8,
    x: f32,
    y: f32,
    radius: f32,
    intensity: f32,
) {
    if let Some(w) = ptr.as_mut() {
        w.apply_brush(brush_kind, x, y, radius, intensity);
    }
}

/// # Safety
/// `ptr` must be a valid mutable pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_pick_cell(ptr: *mut World, x: f32, y: f32, max_dist: f32) -> u32 {
    if let Some(w) = ptr.as_mut() {
        w.pick_cell(x, y, max_dist)
    } else {
        0
    }
}

/// # Safety
/// `ptr` must be a valid pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_selected_cell_id(ptr: *const World) -> u32 {
    ptr.as_ref().map_or(0, |w| w.selected_cell_id)
}

/// # Safety
/// `ptr` must be a valid mutable pointer to `World`.
#[no_mangle]
pub unsafe extern "C" fn sim_set_growth_limit(
    ptr: *mut World,
    enabled: u8,
    max_org_cells: u32,
    max_total_cells: u32,
) {
    if let Some(w) = ptr.as_mut() {
        w.growth_limit_enabled = enabled != 0;
        w.max_org_cells = max_org_cells.clamp(4, 64) as u16;
        w.max_total_cells = max_total_cells.clamp(40, crate::world::MAX_CELLS as u32) as u16;
    }
}
