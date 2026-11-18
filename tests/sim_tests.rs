use bio_swarm_lab::ffi;
use bio_swarm_lab::fields::EnvFields;
use bio_swarm_lab::genome::{CellMode, CellRole, OrganismGenome};
use bio_swarm_lab::mutation::{
    MUT_TARGET_ALL, MUT_TARGET_DIFFERENTIATION, MUT_TARGET_NEURAL, MUT_TRIGGER_DIRECT_ONLY,
    MUT_TRIGGER_ON_MUTAGEN_FIELD,
};
use bio_swarm_lab::neural::{CognitiveState, NeuralNet, NN_INPUTS, NN_MEMORY, NN_OUTPUTS};
use bio_swarm_lab::rng::Rng;
use bio_swarm_lab::virus::{ViralStrategy, ALL_ROLES_MASK};
use bio_swarm_lab::world::{
    World, BOND_STRIDE, CELL_STRIDE, INSPECTOR_LEN, MUTATION_LOG_STRIDE, STATS_LEN, VIRION_STRIDE,
};

#[test]
fn recurrent_neural_forward_memory_and_cognitive_states() {
    let mut rng = Rng::new(42);
    let mut net = NeuralNet::for_role(CellRole::Neurocyte, &mut rng);
    let mem_before = net.memory;

    let inputs: [f32; NN_INPUTS] = [
        0.75, 0.85, 0.05, 0.30, 0.45, 0.25, 0.60, 0.82, 0.0, 0.0, 0.55, 0.20,
    ];
    let out = net.forward(inputs, 0.1);
    assert_eq!(out.len(), NN_OUTPUTS);
    for &v in &out {
        assert!(v.is_finite() && (-1.0..=1.0).contains(&v));
    }
    assert_eq!(net.last_inputs, inputs);
    assert_eq!(net.last_outputs, out);

    let mem_changed = net
        .memory
        .iter()
        .zip(mem_before.iter())
        .any(|(a, b)| (*a - *b).abs() > 1e-5);
    assert!(
        mem_changed,
        "4 recurrent working-memory cells must update on forward pass"
    );
    assert_eq!(net.memory.len(), NN_MEMORY);

    let state_coop = net.classify_cognitive_state(CellRole::Neurocyte, 0.80, 0.0, 2);
    assert!(matches!(
        state_coop,
        CognitiveState::CooperateSwarm | CognitiveState::Reproduce | CognitiveState::Forage
    ));

    let state_def = net.classify_cognitive_state(CellRole::Immunocyte, 0.70, 0.45, 0);
    assert_eq!(state_def, CognitiveState::DefendHeal);
}

#[test]
fn hebbian_synaptic_plasticity_adapts_weights() {
    let mut rng = Rng::new(101);
    let mut net = NeuralNet::for_role(CellRole::Phagocyte, &mut rng);
    let inputs: [f32; NN_INPUTS] = [0.6, 0.5, 0.0, 0.2, 0.4, 0.85, 0.1, 0.75, 0.0, 0.0, 0.3, 0.1];
    net.forward(inputs, 0.0);
    let w2_before = net.w2;

    net.adapt(0.95, 0.35);
    let changed = net
        .w2
        .iter()
        .zip(w2_before.iter())
        .filter(|(a, b)| (*a - *b).abs() > 1e-6)
        .count();
    assert!(
        changed > 10,
        "Hebbian adaptation must modify active synaptic weights"
    );
    assert!(
        net.hebbian_delta > 0.0,
        "hebbian_delta telemetry must be positive after adaptation"
    );
}

#[test]
fn q10_scaling_and_thermal_damage_outside_tolerance() {
    let mut m = CellMode::default_for_role(CellRole::Photocyte, 0);
    m.t_opt = 25.0;
    m.t_tol = 15.0;

    let r_opt = m.q10_rate(25.0);
    let r_hot = m.q10_rate(35.0);
    let r_cold = m.q10_rate(15.0);
    assert!((r_opt - 1.0).abs() < 1e-4);
    assert!((r_hot - 2.0).abs() < 1e-4);
    assert!((r_cold - 0.5).abs() < 1e-4);

    assert_eq!(m.thermal_stress(25.0), 0.0);
    assert!(m.thermal_stress(65.0) > 1.0);
    assert!(m.thermal_stress(-10.0) < -1.0);

    let mut world = World::new(99, 5);
    world.fields.reset(88.0, 0.6);
    let cid = world.spawn_custom_cell(
        600.0,
        400.0,
        CellRole::Photocyte,
        0.1,
        1.0,
        25.0,
        10.0,
        0.0,
        0.05,
        0.2,
        0.0,
        1,
        10.0,
    );
    let init_hp = world.cells.iter().find(|c| c.id == cid).unwrap().hp;
    for _ in 0..30 {
        world.step(1.0);
    }
    let hp_dropped = match world.cells.iter().find(|c| c.id == cid) {
        Some(c) => c.hp < init_hp - 5.0,
        None => true,
    };
    assert!(
        hp_dropped,
        "extreme heat outside tolerance must damage or kill non-extremophile cell"
    );
}

#[test]
fn cell_lab_multicellular_differentiation_and_adhesion_bonds() {
    let mut world = World::new(202, 5);
    // Spawn Apex Leviathan (preset 0, genome_id 1) in blank sandbox
    let last_id = world.spawn_organism(1, 600.0, 400.0, 2, 30.0);
    assert_ne!(last_id, 0);
    assert!(
        world.cells.len() >= 6,
        "multi-mode organism spawn must create differentiated embryonic cells"
    );
    assert!(
        !world.bonds.is_empty(),
        "multi-mode organism must form cytoskeletal adhesion bonds"
    );

    // Boost ATP so cells undergo additional mitotic mode differentiation (`child_a_mode` / `child_b_mode`)
    for c in &mut world.cells {
        c.atp = c.max_atp;
        c.divide_cooldown = 0;
    }
    for _ in 0..25 {
        world.step(1.0);
    }

    let mut distinct_modes = std::collections::BTreeSet::new();
    let mut distinct_roles = std::collections::BTreeSet::new();
    for c in &world.cells {
        distinct_modes.insert(c.mode_idx);
        distinct_roles.insert(c.role().as_u8());
    }
    assert!(
        distinct_modes.len() >= 3,
        "expected >= 3 differentiated modes in multi-mode creature, got {distinct_modes:?}"
    );
    assert!(
        distinct_roles.len() >= 3,
        "expected >= 3 specialized cell roles in multi-mode creature, got {distinct_roles:?}"
    );
}

#[test]
fn synaptic_propagation_and_myocyte_muscle_flex_locomotion() {
    let mut world = World::new(303, 5);
    // Spawn Segmented Muscle Worm (preset 4, genome_id 5) which pairs Senseocyte/Neurocyte + Myocytes
    world.spawn_organism(5, 600.0, 400.0, 1, 10.0);
    assert!(world.cells.len() >= 3);
    assert!(!world.bonds.is_empty());

    let start_cx: f32 = world.cells.iter().map(|c| c.x).sum::<f32>() / world.cells.len() as f32;
    let start_cy: f32 = world.cells.iter().map(|c| c.y).sum::<f32>() / world.cells.len() as f32;

    let mut saw_synaptic_flow = false;
    let mut saw_myocyte_flex = false;
    for _ in 0..35 {
        world.step(1.0);
        if world.bonds.iter().any(|b| b.last_synaptic_flow > 0.01) {
            saw_synaptic_flow = true;
        }
        if world
            .cells
            .iter()
            .any(|c| c.role() == CellRole::Myocyte && c.myocyte_flex.abs() > 0.01)
        {
            saw_myocyte_flex = true;
        }
    }

    let end_cx: f32 = world.cells.iter().map(|c| c.x).sum::<f32>() / world.cells.len() as f32;
    let end_cy: f32 = world.cells.iter().map(|c| c.y).sum::<f32>() / world.cells.len() as f32;
    let disp = ((end_cx - start_cx).powi(2) + (end_cy - start_cy).powi(2)).sqrt();

    assert!(
        saw_synaptic_flow,
        "Neurocyte/Senseocyte must propagate synaptic impulses across adhesion bonds"
    );
    assert!(
        saw_myocyte_flex,
        "Myocyte cells must produce non-zero contractile flex outputs"
    );
    assert!(
        disp > 1.5,
        "multicellular worm must locomote across the dish (disp={disp})"
    );
}

#[test]
fn vascularocyte_atp_pumping_and_keratinocyte_shielding() {
    let mut world = World::new(404, 5);
    let mut g = OrganismGenome::preset(1);
    g.id = 99;
    g.modes[0] = CellMode::default_for_role(CellRole::Vascularocyte, 0);
    g.modes[0].child_a_mode = 1;
    g.modes[0].child_b_mode = 2;
    g.modes[0].keep_adhesion_parent = true;
    g.modes[0].bond_children = true;
    g.modes[1] = CellMode::default_for_role(CellRole::Keratinocyte, 1);
    g.modes[2] = CellMode::default_for_role(CellRole::Photocyte, 2);
    world.genomes.push(g);

    world.spawn_organism(99, 500.0, 400.0, 1, 5.0);
    assert_eq!(world.cells.len(), 3);
    world.cells[0].atp = 140.0;
    world.cells[1].atp = 15.0;
    world.cells[2].atp = 15.0;

    world.step(1.0);
    assert!(
        world.cells[1].atp > 20.0,
        "Vascularocyte must rapidly pump ATP across bonds to low-ATP neighbors"
    );
    assert!(
        world.cells[0].keratin_shield > 0.1,
        "Keratinocyte must grant armor shielding to bonded neighbors"
    );
}

#[test]
fn virus_lytic_infection_incubation_and_burst() {
    let mut world = World::new(505, 5);
    world.spawn_custom_cell(
        600.0,
        400.0,
        CellRole::Photocyte,
        0.05,
        1.0,
        25.0,
        20.0,
        0.0,
        0.05,
        0.02,
        0.0,
        8,
        16.0,
    );

    let sid = world.upsert_virus_strain(
        0,
        ViralStrategy::Lytic.as_u8(),
        ALL_ROLES_MASK,
        0.98,
        24.0,
        6,
        0.75,
        0.30,
        0.90,
        0.50,
        false,
        0.04,
    );
    world.inject_strain(sid, 600.0, 400.0, 35, 16.0);

    let mut saw_infected = false;
    for _ in 0..65 {
        world.step(1.0);
        if world.cells.iter().any(|c| c.infected_by.is_some()) {
            saw_infected = true;
        }
        if world.total_bursts > 0 {
            break;
        }
    }
    assert!(saw_infected, "Lytic strain must infect susceptible cells");
    assert!(
        world.total_bursts > 0,
        "Lytic strain must complete incubation and burst host cell"
    );
}

#[test]
fn virus_lysogenic_dormant_inheritance_and_stress_activation() {
    let mut world = World::new(606, 5);
    let cid = world.spawn_custom_cell(
        600.0,
        400.0,
        CellRole::Stemocyte,
        0.3,
        1.1,
        25.0,
        20.0,
        0.0,
        0.05,
        0.05,
        0.0,
        1,
        5.0,
    );

    let sid = world.upsert_virus_strain(
        0,
        ViralStrategy::Lysogenic.as_u8(),
        ALL_ROLES_MASK,
        0.98,
        40.0,
        5,
        0.50,
        0.20,
        0.90,
        0.60,
        true,
        0.48,
    );
    world.inject_strain(sid, 600.0, 400.0, 10, 10.0);

    {
        let host = world.cells.iter_mut().find(|c| c.id == cid).unwrap();
        assert!(
            host.lysogenic_dormant,
            "Lysogenic virus must enter dormant provirus state on infection"
        );
        host.atp = host.max_atp;
        host.divide_cooldown = 0;
    }

    world.step(1.0);
    let lysogenic_count = world
        .cells
        .iter()
        .filter(|c| c.infected_by.is_some() && c.lysogenic_dormant)
        .count();
    assert!(
        lysogenic_count >= 2,
        "Lysogenic dormant provirus must be inherited across host mitosis"
    );

    // Apply Mutagen brush to induce stress activation of dormant provirus into lytic phase!
    world.apply_brush(5, 600.0, 400.0, 80.0, 1.0);
    for _ in 0..8 {
        world.step(1.0);
    }
    let activated = world
        .cells
        .iter()
        .any(|c| c.infected_by.is_some() && !c.lysogenic_dormant);
    assert!(
        activated,
        "mutagen/stress exposure must activate dormant Lysogenic provirus into lytic replication"
    );
}

#[test]
fn virus_oncogenic_mutator_hijacks_mitosis_and_genome() {
    let mut world = World::new(707, 5);
    world.spawn_custom_cell(
        600.0,
        400.0,
        CellRole::Photocyte,
        0.3,
        1.0,
        25.0,
        20.0,
        0.0,
        0.05,
        0.05,
        0.0,
        2,
        8.0,
    );

    let sid = world.upsert_virus_strain(
        0,
        ViralStrategy::OncogenicMutator.as_u8(),
        ALL_ROLES_MASK,
        0.98,
        120.0,
        4,
        0.25,
        0.45,
        0.92,
        0.55,
        true,
        0.14,
    );
    world.inject_strain(sid, 600.0, 400.0, 12, 10.0);

    for _ in 0..45 {
        for c in &mut world.cells {
            c.atp = c.max_atp * 0.85;
        }
        world.step(1.0);
    }
    assert!(
        world.total_mutations > 0,
        "OncogenicMutator virus must induce host genome/neural mutations"
    );
    assert!(
        world.cells.len() > 2,
        "OncogenicMutator virus must drive rapid host proliferation"
    );
}

#[test]
fn immunocyte_crispr_adaptive_immunity_and_virion_clearance() {
    let mut world = World::new(808, 5);
    world.spawn_custom_cell(
        600.0,
        400.0,
        CellRole::Immunocyte,
        0.5,
        1.1,
        25.0,
        20.0,
        0.6,
        0.05,
        0.85,
        0.2,
        14,
        24.0,
    );
    world.inject_custom_virus(600.0, 400.0, 0.75, 100.0, 4, 0.5, 0.1, 0.1, 35, 25.0);

    for _ in 0..35 {
        world.step(1.0);
    }
    assert!(
        world.total_neutralized > 0,
        "Immunocytes and secreted antibodies must neutralize virions"
    );
    let any_crispr = world.cells.iter().any(|c| c.crispr_mask != 0);
    assert!(
        any_crispr,
        "Immunocytes must record CRISPR adaptive immunity bitmasks after viral encounter"
    );
}

#[test]
fn custom_mutation_rules_and_mutagen_field_logging() {
    let mut world = World::new(909, 5);
    world.clear_mutation_rules();
    assert_eq!(world.mutation_rules.len(), 0);

    let rule_id = world.upsert_mutation_rule(
        0,
        true,
        MUT_TARGET_ALL,
        0.35,
        0.65,
        1.0,
        MUT_TRIGGER_DIRECT_ONLY,
    );
    let mut_field_rule = world.upsert_mutation_rule(
        0,
        true,
        MUT_TARGET_DIFFERENTIATION | MUT_TARGET_NEURAL,
        0.20,
        0.50,
        1.0,
        MUT_TRIGGER_ON_MUTAGEN_FIELD,
    );
    assert_ne!(rule_id, mut_field_rule);

    let cid = world.spawn_organism(1, 600.0, 400.0, 1, 10.0);
    let applied = world.trigger_mutation(cid, rule_id);
    assert!(
        applied >= 1,
        "direct mutation rule execution must apply >= 1 gene target event"
    );
    assert!(!world.mutation_log.is_empty());
    assert_eq!(
        world.mutation_log_buf.len(),
        world.mutation_log.len() * MUTATION_LOG_STRIDE
    );

    let before_muts = world.total_mutations;
    world.apply_brush(5, 600.0, 400.0, 75.0, 1.0); // Mutagen Ray brush
    assert!(
        world.total_mutations > before_muts,
        "Mutagen brush must trigger ON_MUTAGEN_FIELD rules and log mutation events"
    );
}

#[test]
fn petri_walls_block_diffusion_and_resolve_cell_collisions() {
    let mut fields = EnvFields::new(40, 30, 10.0, 25.0);
    // Draw a solid vertical wall splitting the chamber at x = 190..210
    fields.set_wall_rect(190.0, 0.0, 210.0, 300.0, 1);
    assert!(fields.is_wall_at(200.0, 150.0));

    // Drop high toxin on left side of wall; verify it does not diffuse through the wall
    fields.apply_brush(3, 175.0, 150.0, 12.0, 1.0);
    for _ in 0..15 {
        fields.step(1.0);
    }
    assert_eq!(
        fields.sample_toxin(225.0, 150.0),
        0.0,
        "impermeable Petri wall must block chemical diffusion across barrier"
    );

    // Verify soft-body wall collision pushes cell out of wall cell
    let (rx, _ry, rvx, _rvy) = fields.resolve_wall_collision(188.0, 150.0, 2.5, 0.0, 8.0);
    assert!(
        rx < 188.0 && rvx <= 0.0,
        "wall collision must push cell out of wall and reflect velocity"
    );
}

#[test]
fn all_six_scenarios_and_full_ffi_contract_zero_nans() {
    for scenario in 0..=5 {
        let mut world = World::new(100 + scenario as u32, scenario);
        if scenario < 5 {
            assert!(
                !world.cells.is_empty(),
                "scenario {scenario} must spawn initial organisms"
            );
        } else {
            assert!(
                world.cells.is_empty(),
                "scenario 5 (Blank Petri Sandbox) must start with 0 cells"
            );
        }
        for _ in 0..50 {
            world.step(1.0);
        }
        for &v in &world.cells_buf {
            assert!(v.is_finite(), "cells_buf must contain zero NaNs/Infs");
        }
        for &v in &world.virions_buf {
            assert!(v.is_finite(), "virions_buf must contain zero NaNs/Infs");
        }
        for &v in &world.bonds_buf {
            assert!(v.is_finite(), "bonds_buf must contain zero NaNs/Infs");
        }
        for &v in &world.stats_buf {
            assert!(v.is_finite(), "stats_buf must contain zero NaNs/Infs");
        }
        for &v in &world.inspector_buf {
            assert!(v.is_finite(), "inspector_buf must contain zero NaNs/Infs");
        }
    }

    unsafe {
        let ptr = ffi::sim_create(42, 0);
        assert!(!ptr.is_null());
        assert_eq!(ffi::sim_cell_stride() as usize, CELL_STRIDE);
        assert_eq!(ffi::sim_virion_stride() as usize, VIRION_STRIDE);
        assert_eq!(ffi::sim_bond_stride() as usize, BOND_STRIDE);
        assert_eq!(ffi::sim_stats_len() as usize, STATS_LEN);
        assert_eq!(ffi::sim_inspector_len() as usize, INSPECTOR_LEN);
        assert_eq!(ffi::sim_mutation_log_stride() as usize, MUTATION_LOG_STRIDE);

        let gid = ffi::sim_register_genome(ptr, 0, 8, 0);
        assert!(gid > 0);
        let spawned_cid = ffi::sim_spawn_organism(ptr, gid, 600.0, 400.0, 2, 25.0);
        assert!(spawned_cid > 0);

        let rid = ffi::sim_upsert_mutation_rule(ptr, 0, 1, MUT_TARGET_ALL, 0.2, 0.5, 1.0, 3);
        let evs = ffi::sim_trigger_mutation(ptr, spawned_cid, rid);
        assert!(evs > 0);
        assert!(ffi::sim_mutation_log_count(ptr) > 0);

        let sid = ffi::sim_upsert_virus_strain(
            ptr,
            0,
            0,
            ALL_ROLES_MASK as u32,
            0.8,
            80.0,
            5,
            0.5,
            0.2,
            0.2,
            0.5,
            1,
            0.1,
        );
        let v_spawned = ffi::sim_inject_strain(ptr, sid, 600.0, 400.0, 15, 20.0);
        assert!(v_spawned > 0);

        for _ in 0..15 {
            ffi::sim_step(ptr, 1.0);
        }

        let cells_ptr = ffi::sim_cells_ptr(ptr);
        let first_cell = std::slice::from_raw_parts(cells_ptr, CELL_STRIDE);
        let picked = ffi::sim_pick_cell(ptr, first_cell[1], first_cell[2], 25.0);
        assert_ne!(picked, 0);

        let insp = std::slice::from_raw_parts(ffi::sim_inspector_ptr(ptr), INSPECTOR_LEN);
        assert_eq!(insp[0], 1.0);
        assert_eq!(insp[1] as u32, picked);
        // Check 4 recurrent working-memory slots at [54..58] are finite and populated
        let mem_slice = &insp[54..58];
        assert!(mem_slice.iter().any(|&m| m.abs() > 1e-4));

        // Verify toggleable growth limit caps dish growth and can be disabled
        ffi::sim_set_growth_limit(ptr, 1, 12, 80);
        assert!((*ptr).growth_limit_enabled);
        assert_eq!((*ptr).max_org_cells, 12);
        assert_eq!((*ptr).max_total_cells, 80);
        ffi::sim_set_growth_limit(ptr, 0, 16, 280);
        assert!(!(*ptr).growth_limit_enabled);

        ffi::sim_destroy(ptr);
    }
}
