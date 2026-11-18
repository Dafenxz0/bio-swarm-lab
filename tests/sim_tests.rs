use bio_swarm_lab::ffi;
use bio_swarm_lab::fields::EnvFields;
use bio_swarm_lab::genome::{CellRole, Genome, VirusStrain};
use bio_swarm_lab::neural::{NeuralNet, NN_INPUTS, NN_OUTPUTS};
use bio_swarm_lab::rng::Rng;
use bio_swarm_lab::world::{World, INSPECTOR_LEN, STATS_LEN};

#[test]
fn neural_forward_and_mutation_work() {
    let mut rng = Rng::new(42);
    let mut net = NeuralNet::for_role(CellRole::Protector, &mut rng);
    let before = net.w1;

    let inputs = [0.8, 0.2, 0.9, 0.1, 0.7, 0.3, 0.6, 0.0, 0.2, 1.0];
    let out = net.forward(inputs, 0.1);
    assert_eq!(out.len(), NN_OUTPUTS);
    for &v in &out {
        assert!(v.is_finite() && (-1.0..=1.0).contains(&v));
    }
    assert_eq!(net.last_inputs, inputs);
    assert_eq!(net.last_outputs, out);

    net.mutate(&mut rng, 0.5, 0.4);
    let changed = net
        .w1
        .iter()
        .zip(before.iter())
        .filter(|(a, b)| (*a - *b).abs() > 1e-6)
        .count();
    assert!(changed > 5, "expected multiple weights to mutate");
    let _ = NN_INPUTS;
}

#[test]
fn q10_scaling_and_thermal_damage_outside_tolerance() {
    let mut rng = Rng::new(7);
    let mut g = Genome::default_for_role(CellRole::Producer, &mut rng);
    g.t_opt = 25.0;
    g.t_tol = 15.0;

    let r_opt = g.q10_rate(25.0);
    let r_hot = g.q10_rate(35.0);
    let r_cold = g.q10_rate(15.0);
    assert!((r_opt - 1.0).abs() < 1e-4);
    assert!((r_hot - 2.0).abs() < 1e-4);
    assert!((r_cold - 0.5).abs() < 1e-4);

    assert_eq!(g.thermal_stress(25.0), 0.0);
    assert!(
        g.thermal_stress(65.0) > 1.0,
        "heat shock must trigger above T_opt+T_tol"
    );
    assert!(
        g.thermal_stress(-10.0) < -1.0,
        "frostbite must trigger below T_opt-T_tol"
    );

    // Verify thermal shock actually reduces cell HP in World simulation
    let mut world = World::new(99, 0);
    world.cells.clear();
    world.virions.clear();
    world.fields.reset(88.0, 0.5);
    let cid = world.spawn_custom_cell(
        600.0,
        400.0,
        CellRole::Producer,
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
    for _ in 0..35 {
        world.step(1.0);
    }
    let after = world.cells.iter().find(|c| c.id == cid);
    let hp_dropped = match after {
        Some(c) => c.hp < init_hp - 5.0,
        None => true,
    };
    assert!(
        hp_dropped,
        "extreme heat outside tolerance must damage or kill non-extremophile cell"
    );
}

#[test]
fn virus_infection_incubation_and_lytic_burst() {
    let mut world = World::new(123, 0);
    world.cells.clear();
    world.virions.clear();
    world.fields.reset(25.0, 0.8);

    // Spawn a susceptible host cluster with zero immune resistance and zero Protectors.
    world.spawn_custom_cell(
        600.0,
        400.0,
        CellRole::Producer,
        0.1,
        1.0,
        25.0,
        20.0,
        0.0,
        0.05,
        0.02,
        0.0,
        10,
        18.0,
    );

    let strain = VirusStrain {
        id: 1,
        generation: 1,
        infectivity: 1.0,
        incubation_ticks: 30.0,
        burst_count: 6,
        lethality: 0.75,
        mutation_rate: 0.4,
        immune_evasion: 0.9,
        strain_hue: 0.85,
    };
    world.spawn_virion_cloud(600.0, 400.0, strain, 40, 16.0);

    let mut saw_infected = false;
    for _ in 0..90 {
        world.step(1.0);
        if world
            .cells
            .iter()
            .any(|c| c.effective_role() == CellRole::Infected)
        {
            saw_infected = true;
        }
        if world.total_bursts > 0 {
            break;
        }
    }
    assert!(saw_infected, "virions must infect susceptible host cells");
    assert!(
        world.total_bursts > 0,
        "infected cells must undergo lytic burst spawning virions"
    );
    assert!(
        !world.virions.is_empty(),
        "lytic burst must release new virions"
    );
}

#[test]
fn immunocytes_and_antibodies_neutralize_virions() {
    let mut world = World::new(555, 1);
    world.cells.clear();
    world.virions.clear();
    world.fields.reset(27.0, 0.8);

    // Dense Protector/Immunocyte squad + high-infectivity virion cloud
    world.spawn_custom_cell(
        600.0,
        400.0,
        CellRole::Protector,
        0.5,
        1.1,
        27.0,
        20.0,
        0.2,
        0.05,
        0.85,
        0.2,
        18,
        28.0,
    );
    world.inject_custom_virus(600.0, 400.0, 0.7, 100.0, 4, 0.5, 0.1, 0.1, 35, 30.0);

    for _ in 0..45 {
        world.step(1.0);
    }
    assert!(
        world.total_neutralized > 0,
        "Protectors and secreted antibodies must neutralize virions"
    );

    // Antibiotic brush directly drops antiviral toxin and clears virions
    world.inject_custom_virus(200.0, 200.0, 0.8, 100.0, 4, 0.5, 0.1, 0.1, 20, 15.0);
    let before_neut = world.total_neutralized;
    world.apply_brush(3, 200.0, 200.0, 50.0, 1.0);
    assert!(world.total_neutralized > before_neut);
    assert!(world.fields.sample_toxin(200.0, 200.0) > 0.2);
}

#[test]
fn multicellular_adhesion_bonds_form_and_share_atp() {
    let mut world = World::new(777, 3);
    world.cells.clear();
    world.virions.clear();
    world.bonds.clear();
    world.fields.reset(24.0, 0.6);

    let id1 = world.spawn_custom_cell(
        500.0,
        400.0,
        CellRole::Producer,
        0.3,
        0.9,
        24.0,
        20.0,
        0.9,
        0.05,
        0.3,
        0.0,
        1,
        2.0,
    );
    let id2 = world.spawn_custom_cell(
        516.0,
        400.0,
        CellRole::Producer,
        0.3,
        0.9,
        24.0,
        20.0,
        0.9,
        0.05,
        0.3,
        0.0,
        1,
        2.0,
    );
    assert_ne!(id1, id2);

    // Set asymmetric ATP to verify sharing across bond
    if let Some(c1) = world.cells.iter_mut().find(|c| c.id == id1) {
        c1.atp = 105.0;
    }
    if let Some(c2) = world.cells.iter_mut().find(|c| c.id == id2) {
        c2.atp = 20.0;
    }

    world.step(1.0);
    assert!(
        !world.bonds.is_empty(),
        "adhesive cells within range must form adhesion bonds"
    );

    let c2_atp = world.cells.iter().find(|c| c.id == id2).unwrap().atp;
    assert!(
        c2_atp > 22.0,
        "ATP must transfer across multicellular adhesion bonds (got {c2_atp})"
    );
}

#[test]
fn environmental_diffusion_works() {
    let mut fields = EnvFields::new(40, 30, 10.0, 25.0);
    fields.apply_brush(0, 200.0, 150.0, 30.0, 1.0); // Heat
    fields.apply_brush(2, 100.0, 100.0, 25.0, 1.0); // Nutrient
    fields.add_alarm(300.0, 200.0, 1.0);
    fields.add_food_pheromone(150.0, 200.0, 1.0);
    fields.add_toxin(250.0, 100.0, 1.0);

    let t_center_before = fields.sample_temp(200.0, 150.0);
    let t_edge_before = fields.sample_temp(225.0, 150.0);
    assert!(t_center_before > 35.0);

    for _ in 0..8 {
        fields.step(1.0);
    }

    let t_center_after = fields.sample_temp(200.0, 150.0);
    let t_edge_after = fields.sample_temp(225.0, 150.0);
    assert!(t_center_after < t_center_before);
    assert!(t_edge_after >= t_edge_before - 1.0);
    assert!(
        fields.sample_alarm(310.0, 200.0) > 0.001,
        "alarm pheromone must diffuse"
    );
    assert!(
        fields.sample_food(160.0, 200.0) > 0.001,
        "food pheromone must diffuse"
    );
    assert!(
        fields.sample_toxin(260.0, 100.0) > 0.001,
        "toxin must diffuse"
    );
}

#[test]
fn multi_generation_survival_and_ffi_contract() {
    for scenario in 0..=3 {
        let mut world = World::new(100 + scenario as u32, scenario);
        assert!(
            !world.cells.is_empty(),
            "scenario {scenario} must spawn initial cells"
        );
        for _ in 0..160 {
            world.step(1.0);
        }
        assert!(
            !world.cells.is_empty(),
            "scenario {scenario} must sustain living cells"
        );
        if scenario == 0 {
            assert!(
                world.total_births > 0,
                "primordial scenario should reproduce over 160 ticks"
            );
        }
    }

    // Exercise C FFI roundtrip as Node/Browser WASM will call it
    unsafe {
        let ptr = ffi::sim_create(42, 0);
        assert!(!ptr.is_null());
        for _ in 0..20 {
            ffi::sim_step(ptr, 1.0);
        }
        let count = ffi::sim_cell_count(ptr);
        assert!(count > 0);
        let cells_ptr = ffi::sim_cells_ptr(ptr);
        let stride = ffi::sim_cell_stride() as usize;
        let first_slice = std::slice::from_raw_parts(cells_ptr, stride);
        let fx = first_slice[1];
        let fy = first_slice[2];
        let picked = ffi::sim_pick_cell(ptr, fx, fy, 20.0);
        assert_ne!(picked, 0);

        let insp_ptr = ffi::sim_inspector_ptr(ptr);
        let insp = std::slice::from_raw_parts(insp_ptr, INSPECTOR_LEN);
        assert_eq!(insp[0], 1.0);
        assert_eq!(insp[1] as u32, picked);

        let stats_ptr = ffi::sim_stats_ptr(ptr);
        let stats = std::slice::from_raw_parts(stats_ptr, STATS_LEN);
        assert_eq!(stats[0], 20.0);
        assert!(stats[1] > 0.0);

        ffi::sim_destroy(ptr);
    }
}
