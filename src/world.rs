//! Continuous biological world, spatial hash, cell interactions, viral lytic cycle, and multicellular adhesion.

use crate::fields::EnvFields;
use crate::genome::{CellRole, Genome, Virion, VirusStrain};
use crate::neural::{NeuralNet, NN_HIDDEN, NN_INPUTS, NN_OUTPUTS, NN_W1_LEN, NN_W2_LEN};
use crate::rng::Rng;

pub const WORLD_W: f32 = 1200.0;
pub const WORLD_H: f32 = 800.0;
pub const GRID_W: usize = 120;
pub const GRID_H: usize = 80;
pub const FIELD_CELL_SIZE: f32 = 10.0;

pub const MAX_CELLS: usize = 520;
pub const MAX_VIRIONS: usize = 700;
pub const MAX_BONDS: usize = 900;

pub const CELL_STRIDE: usize = 16;
pub const VIRION_STRIDE: usize = 5;
pub const BOND_STRIDE: usize = 5;
pub const STATS_LEN: usize = 20;
pub const INSPECTOR_LEN: usize =
    20 + 4 + NN_INPUTS + NN_HIDDEN + NN_OUTPUTS + NN_W1_LEN + NN_W2_LEN; // 176

pub const SIGNAL_ALARM: u8 = 1;
pub const SIGNAL_FOOD: u8 = 2;
pub const SIGNAL_ANTIBODY: u8 = 4;
pub const SIGNAL_DIVIDE: u8 = 8;

const HASH_CELL: f32 = 40.0;
const HASH_W: usize = 30;
const HASH_H: usize = 20;

#[derive(Clone, Debug)]
pub struct Cell {
    pub id: u32,
    pub lineage_id: u32,
    pub parent_id: u32,
    pub generation: u32,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub angle: f32,
    pub radius: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub atp: f32,
    pub max_atp: f32,
    pub age: u32,
    pub divide_cooldown: u16,
    pub base_role: CellRole,
    pub genome: Genome,
    pub brain: NeuralNet,
    pub viral_load: f32,
    pub incubation_progress: f32,
    pub infected_by: Option<VirusStrain>,
    pub temp_stress: f32,
    pub q10_rate: f32,
    pub signal_flags: u8,
    pub alive: bool,
}

impl Cell {
    pub fn effective_role(&self) -> CellRole {
        if self.infected_by.is_some() || self.viral_load > 0.12 {
            CellRole::Infected
        } else {
            self.base_role
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AdhesionBond {
    pub a_id: u32,
    pub b_id: u32,
    pub rest_len: f32,
    pub strength: f32,
}

pub struct World {
    pub fields: EnvFields,
    pub cells: Vec<Cell>,
    pub virions: Vec<Virion>,
    pub bonds: Vec<AdhesionBond>,
    pub rng: Rng,
    pub tick: u32,
    pub scenario: u8,

    pub next_cell_id: u32,
    pub next_lineage_id: u32,
    pub next_strain_id: u32,
    pub selected_cell_id: u32,

    pub total_births: u32,
    pub total_bursts: u32,
    pub total_neutralized: u32,

    cell_buckets: Vec<Vec<usize>>,
    virion_buckets: Vec<Vec<usize>>,
    id_to_idx: Vec<(u32, usize)>,

    pub cells_buf: Vec<f32>,
    pub virions_buf: Vec<f32>,
    pub bonds_buf: Vec<f32>,
    pub stats_buf: [f32; STATS_LEN],
    pub inspector_buf: [f32; INSPECTOR_LEN],
}

impl World {
    pub fn new(seed: u32, scenario: u8) -> Self {
        let mut world = Self {
            fields: EnvFields::new(GRID_W, GRID_H, FIELD_CELL_SIZE, 25.0),
            cells: Vec::with_capacity(MAX_CELLS),
            virions: Vec::with_capacity(MAX_VIRIONS),
            bonds: Vec::with_capacity(MAX_BONDS),
            rng: Rng::new(seed as u64 + 1),
            tick: 0,
            scenario,
            next_cell_id: 1,
            next_lineage_id: 1,
            next_strain_id: 1,
            selected_cell_id: 0,
            total_births: 0,
            total_bursts: 0,
            total_neutralized: 0,
            cell_buckets: vec![Vec::with_capacity(8); HASH_W * HASH_H],
            virion_buckets: vec![Vec::with_capacity(8); HASH_W * HASH_H],
            id_to_idx: Vec::with_capacity(MAX_CELLS),
            cells_buf: Vec::with_capacity(MAX_CELLS * CELL_STRIDE),
            virions_buf: Vec::with_capacity(MAX_VIRIONS * VIRION_STRIDE),
            bonds_buf: Vec::with_capacity(MAX_BONDS * BOND_STRIDE),
            stats_buf: [0.0; STATS_LEN],
            inspector_buf: [0.0; INSPECTOR_LEN],
        };
        world.reset_scenario(scenario);
        world
    }

    pub fn reset_scenario(&mut self, scenario: u8) {
        self.scenario = scenario;
        self.cells.clear();
        self.virions.clear();
        self.bonds.clear();
        self.tick = 0;
        self.total_births = 0;
        self.total_bursts = 0;
        self.total_neutralized = 0;
        self.selected_cell_id = 0;

        match scenario {
            // 0: Primordial Evolution
            0 => {
                self.fields.reset(25.0, 0.62);
                self.fields.nutrient_floor = 0.48;
                self.fields
                    .add_thermal_source(220.0, 220.0, 120.0, 44.0, 0.14);
                self.fields
                    .add_thermal_source(980.0, 580.0, 120.0, 8.0, 0.14);

                self.spawn_cluster(300.0, 400.0, CellRole::Producer, 34, 210.0, None);
                self.spawn_cluster(600.0, 400.0, CellRole::Producer, 28, 260.0, None);
                self.spawn_cluster(900.0, 400.0, CellRole::Swimmer, 22, 220.0, None);
                self.spawn_cluster(600.0, 400.0, CellRole::Replicator, 18, 220.0, None);
                self.spawn_cluster(540.0, 380.0, CellRole::Protector, 16, 200.0, None);
                self.spawn_cluster(760.0, 260.0, CellRole::Predator, 10, 160.0, None);
            }
            // 1: Immune Swarm vs Pandemic
            1 => {
                self.fields.reset(27.0, 0.65);
                self.fields.nutrient_floor = 0.52;
                self.spawn_cluster(600.0, 400.0, CellRole::Producer, 48, 280.0, None);
                self.spawn_cluster(600.0, 400.0, CellRole::Protector, 32, 240.0, None);
                self.spawn_cluster(550.0, 360.0, CellRole::Swimmer, 20, 240.0, None);
                self.spawn_cluster(650.0, 440.0, CellRole::Replicator, 16, 220.0, None);
                self.spawn_cluster(260.0, 220.0, CellRole::Predator, 6, 100.0, None);

                let strain = VirusStrain {
                    id: self.alloc_strain_id(),
                    generation: 1,
                    infectivity: 0.72,
                    incubation_ticks: 95.0,
                    burst_count: 6,
                    lethality: 0.65,
                    mutation_rate: 0.24,
                    immune_evasion: 0.25,
                    strain_hue: 0.84,
                };
                self.spawn_virion_cloud(420.0, 320.0, strain.clone(), 42, 70.0);
                self.spawn_virion_cloud(800.0, 480.0, strain, 42, 70.0);
            }
            // 2: Thermal Vent Extremophiles
            2 => {
                self.fields.reset(24.0, 0.56);
                self.fields.nutrient_floor = 0.45;
                // Hydrothermal vents on left, Cryo glacial pockets on right.
                self.fields
                    .add_thermal_source(200.0, 250.0, 170.0, 86.0, 0.28);
                self.fields
                    .add_thermal_source(230.0, 580.0, 150.0, 78.0, 0.25);
                self.fields
                    .add_thermal_source(980.0, 240.0, 165.0, -18.0, 0.28);
                self.fields
                    .add_thermal_source(950.0, 570.0, 155.0, -14.0, 0.24);
                // Warm up initial fields immediately
                for _ in 0..8 {
                    self.fields.step(1.0);
                }

                // Thermophiles near vents
                let mut thermo_g = Genome::default_for_role(CellRole::Producer, &mut self.rng);
                thermo_g.t_opt = 72.0;
                thermo_g.t_tol = 22.0;
                self.spawn_cluster(260.0, 380.0, CellRole::Producer, 22, 130.0, Some(thermo_g));

                let mut thermo_rep = Genome::default_for_role(CellRole::Replicator, &mut self.rng);
                thermo_rep.t_opt = 68.0;
                thermo_rep.t_tol = 22.0;
                self.spawn_cluster(
                    260.0,
                    420.0,
                    CellRole::Replicator,
                    12,
                    120.0,
                    Some(thermo_rep),
                );

                // Temperate bridge cells in center
                self.spawn_cluster(600.0, 400.0, CellRole::Producer, 26, 150.0, None);
                self.spawn_cluster(600.0, 400.0, CellRole::Swimmer, 18, 160.0, None);
                self.spawn_cluster(600.0, 400.0, CellRole::Protector, 12, 140.0, None);
                self.spawn_cluster(600.0, 400.0, CellRole::Predator, 8, 120.0, None);

                // Cryophiles on right
                let mut cryo_g = Genome::default_for_role(CellRole::Producer, &mut self.rng);
                cryo_g.t_opt = -8.0;
                cryo_g.t_tol = 20.0;
                self.spawn_cluster(920.0, 400.0, CellRole::Producer, 22, 130.0, Some(cryo_g));

                let mut cryo_swim = Genome::default_for_role(CellRole::Swimmer, &mut self.rng);
                cryo_swim.t_opt = -4.0;
                cryo_swim.t_tol = 20.0;
                self.spawn_cluster(920.0, 400.0, CellRole::Swimmer, 12, 120.0, Some(cryo_swim));
            }
            // 3: Multicellular Reef
            _ => {
                self.fields.reset(24.0, 0.66);
                self.fields.nutrient_floor = 0.50;
                let mut reef_prod = Genome::default_for_role(CellRole::Producer, &mut self.rng);
                reef_prod.adhesion = 0.85;
                reef_prod.membrane_armor = 0.42;
                self.spawn_cluster(
                    420.0,
                    320.0,
                    CellRole::Producer,
                    26,
                    75.0,
                    Some(reef_prod.clone()),
                );
                self.spawn_cluster(760.0, 460.0, CellRole::Producer, 26, 75.0, Some(reef_prod));

                let mut reef_prot = Genome::default_for_role(CellRole::Protector, &mut self.rng);
                reef_prot.adhesion = 0.78;
                self.spawn_cluster(
                    420.0,
                    320.0,
                    CellRole::Protector,
                    14,
                    80.0,
                    Some(reef_prot.clone()),
                );
                self.spawn_cluster(760.0, 460.0, CellRole::Protector, 14, 80.0, Some(reef_prot));

                let mut reef_rep = Genome::default_for_role(CellRole::Replicator, &mut self.rng);
                reef_rep.adhesion = 0.80;
                self.spawn_cluster(590.0, 390.0, CellRole::Replicator, 14, 90.0, Some(reef_rep));

                self.spawn_cluster(200.0, 620.0, CellRole::Predator, 10, 110.0, None);
                self.spawn_cluster(980.0, 180.0, CellRole::Swimmer, 18, 120.0, None);

                let strain = VirusStrain {
                    id: self.alloc_strain_id(),
                    generation: 1,
                    infectivity: 0.55,
                    incubation_ticks: 120.0,
                    burst_count: 5,
                    lethality: 0.55,
                    mutation_rate: 0.18,
                    immune_evasion: 0.20,
                    strain_hue: 0.88,
                };
                self.spawn_virion_cloud(180.0, 180.0, strain, 18, 50.0);
            }
        }

        // Form initial adhesion bonds for multicellular scenarios & select first cell for X-ray preview.
        self.rebuild_spatial_hashes();
        self.update_adhesion_bonds(1.0);
        if let Some(first) = self.cells.first() {
            self.selected_cell_id = first.id;
        }
        self.rebuild_render_buffers();
    }

    fn alloc_cell_id(&mut self) -> u32 {
        let id = self.next_cell_id;
        self.next_cell_id = self.next_cell_id.wrapping_add(1).max(1);
        id
    }

    fn alloc_lineage_id(&mut self) -> u32 {
        let id = self.next_lineage_id;
        self.next_lineage_id = self.next_lineage_id.wrapping_add(1).max(1);
        id
    }

    fn alloc_strain_id(&mut self) -> u32 {
        let id = self.next_strain_id;
        self.next_strain_id = self.next_strain_id.wrapping_add(1).max(1);
        id
    }

    pub fn spawn_cluster(
        &mut self,
        cx: f32,
        cy: f32,
        role: CellRole,
        count: u32,
        spread: f32,
        template: Option<Genome>,
    ) -> u32 {
        let mut last_id = 0;
        let lineage_id = self.alloc_lineage_id();
        for _ in 0..count {
            if self.cells.len() >= MAX_CELLS {
                break;
            }
            let ang = self.rng.range_f32(0.0, std::f32::consts::TAU);
            let dist = self.rng.next_f32().sqrt() * spread;
            let x = (cx + ang.cos() * dist).clamp(20.0, WORLD_W - 20.0);
            let y = (cy + ang.sin() * dist).clamp(20.0, WORLD_H - 20.0);
            let mut genome = match &template {
                Some(g) => g.clone(),
                None => Genome::default_for_role(role, &mut self.rng),
            };
            genome.role = role;
            let brain = NeuralNet::for_role(role, &mut self.rng);
            last_id = self.push_cell(x, y, role, genome, brain, lineage_id, 0, 1);
        }
        last_id
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn_custom_cell(
        &mut self,
        x: f32,
        y: f32,
        role: CellRole,
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
        let role = if role == CellRole::Infected {
            CellRole::Producer
        } else {
            role
        };
        let genome = Genome {
            role,
            t_opt: t_opt.clamp(-25.0, 95.0),
            t_tol: t_tol.clamp(6.0, 38.0),
            metabolism: metabolism.clamp(0.45, 2.2),
            membrane_armor: armor.clamp(0.02, 0.90),
            adhesion: adhesion.clamp(0.0, 1.0),
            immune_resistance: immune_resistance.clamp(0.02, 0.95),
            mutation_rate: mutation_rate.clamp(0.01, 0.40),
            neural_bias: neural_bias.clamp(-1.0, 1.0),
            hue_shift: self.rng.range_f32(-0.06, 0.06),
        };
        let last_id = self.spawn_cluster(
            x,
            y,
            role,
            count.clamp(1, 30),
            spread.max(8.0),
            Some(genome),
        );
        if last_id != 0 {
            self.selected_cell_id = last_id;
        }
        self.rebuild_spatial_hashes();
        self.update_adhesion_bonds(0.2);
        self.rebuild_render_buffers();
        last_id
    }

    #[allow(clippy::too_many_arguments)]
    fn push_cell(
        &mut self,
        x: f32,
        y: f32,
        base_role: CellRole,
        genome: Genome,
        brain: NeuralNet,
        lineage_id: u32,
        parent_id: u32,
        generation: u32,
    ) -> u32 {
        let id = self.alloc_cell_id();
        let max_hp = 85.0 + genome.membrane_armor * 75.0;
        let max_atp = match base_role {
            CellRole::Replicator => 95.0,
            CellRole::Predator => 135.0,
            _ => 115.0,
        };
        let cell = Cell {
            id,
            lineage_id,
            parent_id,
            generation,
            x,
            y,
            vx: self.rng.range_f32(-0.5, 0.5),
            vy: self.rng.range_f32(-0.5, 0.5),
            angle: self.rng.range_f32(0.0, std::f32::consts::TAU),
            radius: base_role.base_radius(),
            hp: max_hp,
            max_hp,
            atp: max_atp * 0.65,
            max_atp,
            age: 0,
            divide_cooldown: 40,
            base_role,
            genome,
            brain,
            viral_load: 0.0,
            incubation_progress: 0.0,
            infected_by: None,
            temp_stress: 0.0,
            q10_rate: 1.0,
            signal_flags: 0,
            alive: true,
        };
        self.cells.push(cell);
        id
    }

    pub fn spawn_virion_cloud(
        &mut self,
        cx: f32,
        cy: f32,
        strain: VirusStrain,
        count: u32,
        spread: f32,
    ) -> u32 {
        let mut spawned = 0;
        for _ in 0..count {
            if self.virions.len() >= MAX_VIRIONS {
                break;
            }
            let ang = self.rng.range_f32(0.0, std::f32::consts::TAU);
            let dist = self.rng.next_f32().sqrt() * spread;
            let x = (cx + ang.cos() * dist).clamp(8.0, WORLD_W - 8.0);
            let y = (cy + ang.sin() * dist).clamp(8.0, WORLD_H - 8.0);
            let vang = self.rng.range_f32(0.0, std::f32::consts::TAU);
            self.virions.push(Virion {
                x,
                y,
                vx: vang.cos() * 1.4,
                vy: vang.sin() * 1.4,
                age: 0.0,
                max_age: self.rng.range_f32(420.0, 760.0),
                strain: strain.clone(),
                alive: true,
            });
            spawned += 1;
        }
        self.rebuild_render_buffers();
        spawned
    }

    #[allow(clippy::too_many_arguments)]
    pub fn inject_custom_virus(
        &mut self,
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
        let mut strain = VirusStrain {
            id: self.alloc_strain_id(),
            generation: 1,
            infectivity,
            incubation_ticks,
            burst_count,
            lethality,
            mutation_rate,
            immune_evasion,
            strain_hue: 0.85,
        };
        strain.clamp_fields();
        self.spawn_virion_cloud(x, y, strain, count.clamp(1, 80), spread.max(8.0))
    }

    pub fn apply_brush(&mut self, brush_kind: u8, x: f32, y: f32, radius: f32, intensity: f32) {
        self.fields.apply_brush(brush_kind, x, y, radius, intensity);
        // Antibiotic brush also directly damages nearby free virions.
        if brush_kind == 3 {
            let r2 = radius * radius;
            for v in &mut self.virions {
                let dx = v.x - x;
                let dy = v.y - y;
                if dx * dx + dy * dy <= r2 {
                    v.alive = false;
                    self.total_neutralized = self.total_neutralized.saturating_add(1);
                }
            }
            self.virions.retain(|v| v.alive);
        }
        self.rebuild_render_buffers();
    }

    pub fn pick_cell(&mut self, x: f32, y: f32, max_dist: f32) -> u32 {
        let max_d2 = max_dist * max_dist;
        let mut best_id = 0;
        let mut best_d2 = max_d2;
        for c in &self.cells {
            if !c.alive {
                continue;
            }
            let dx = c.x - x;
            let dy = c.y - y;
            let d2 = dx * dx + dy * dy;
            if d2 <= best_d2 {
                best_d2 = d2;
                best_id = c.id;
            }
        }
        if best_id != 0 {
            self.selected_cell_id = best_id;
        }
        self.rebuild_render_buffers();
        best_id
    }

    /// Advance the full biological simulation by `dt` (typically `1.0`).
    pub fn step(&mut self, dt: f32) {
        let dt = dt.clamp(0.1, 2.0);
        self.tick = self.tick.saturating_add(1);

        // 1. Environmental diffusion, vents, nutrient regeneration, pheromone & toxin decay.
        self.fields.step(dt);

        // 2. Rebuild spatial hash grids for cells and virions.
        self.rebuild_spatial_hashes();

        // 3. Sensory inputs + Per-cell Neural Network forward pass + metabolism + Q10 + thermal damage.
        let n_cells = self.cells.len();
        let mut new_offspring: Vec<Cell> = Vec::new();
        let mut new_virions: Vec<Virion> = Vec::new();

        for idx in 0..n_cells {
            if !self.cells[idx].alive {
                continue;
            }
            self.step_cell_brain_and_physiology(idx, dt, &mut new_offspring, &mut new_virions);
        }

        // 4. Cell-cell interactions: Predator engulfment, Protector antiviral immunity & ally healing, soft-body repulsion.
        self.step_cell_interactions(dt);

        // 5. Virion chemotactic drift, antibody neutralization, and host infection attachment.
        self.step_virions(dt);

        // 6. Append newborn cells & burst virions respecting population caps.
        for child in new_offspring {
            if self.cells.len() < MAX_CELLS {
                self.cells.push(child);
                self.total_births = self.total_births.saturating_add(1);
            }
        }
        for v in new_virions {
            if self.virions.len() < MAX_VIRIONS {
                self.virions.push(v);
            }
        }

        // 7. Prune dead cells & virions.
        self.cells.retain(|c| c.alive);
        self.virions.retain(|v| v.alive);

        // 8. Multicellular adhesion bonds: spring physics + ATP nutrient sharing + bond formation/pruning.
        self.rebuild_spatial_hashes();
        self.update_adhesion_bonds(dt);

        // 9. Keep selected cell valid if alive; pack flat FFI buffers.
        self.rebuild_render_buffers();
    }

    fn step_cell_brain_and_physiology(
        &mut self,
        idx: usize,
        dt: f32,
        new_offspring: &mut Vec<Cell>,
        new_virions: &mut Vec<Virion>,
    ) {
        let (cx, cy, c_id, c_lineage, base_role, eff_role, atp, max_atp, viral_load) = {
            let c = &self.cells[idx];
            (
                c.x,
                c.y,
                c.id,
                c.lineage_id,
                c.base_role,
                c.effective_role(),
                c.atp,
                c.max_atp,
                c.viral_load,
            )
        };

        let local_temp = self.fields.sample_temp(cx, cy);
        let local_nutrient = self.fields.sample_nutrient(cx, cy);
        let local_alarm = self.fields.sample_alarm(cx, cy);
        let local_food = self.fields.sample_food(cx, cy);
        let local_toxin = self.fields.sample_toxin(cx, cy);

        // Scan nearby cells & virions via spatial hash for sensory inputs + chemotactic target.
        let mut nearest_enemy_d2 = 95.0_f32 * 95.0;
        let mut enemy_dir: Option<(f32, f32)> = None;
        let mut nearest_ally_d2 = 95.0_f32 * 95.0;
        let mut ally_dir: Option<(f32, f32)> = None;
        let mut crowd_count = 0.0_f32;

        let gx = ((cx / HASH_CELL) as isize).clamp(0, HASH_W as isize - 1) as usize;
        let gy = ((cy / HASH_CELL) as isize).clamp(0, HASH_H as isize - 1) as usize;

        for ny in gy.saturating_sub(1)..=(gy + 1).min(HASH_H - 1) {
            for nx in gx.saturating_sub(1)..=(gx + 1).min(HASH_W - 1) {
                let b_idx = ny * HASH_W + nx;
                for &other_idx in &self.cell_buckets[b_idx] {
                    if other_idx == idx {
                        continue;
                    }
                    let o = &self.cells[other_idx];
                    if !o.alive {
                        continue;
                    }
                    let dx = o.x - cx;
                    let dy = o.y - cy;
                    let d2 = dx * dx + dy * dy;
                    if d2 < 32.0 * 32.0 {
                        crowd_count += 1.0;
                    }
                    let o_role = o.effective_role();
                    let is_threat = match base_role {
                        CellRole::Protector => {
                            o_role == CellRole::Infected || o_role == CellRole::Predator
                        }
                        CellRole::Predator => {
                            o.lineage_id != c_lineage && o.base_role != CellRole::Predator
                        }
                        _ => o_role == CellRole::Predator || o_role == CellRole::Infected,
                    };
                    if is_threat && d2 < nearest_enemy_d2 {
                        nearest_enemy_d2 = d2;
                        enemy_dir = Some((dx, dy));
                    } else if !is_threat && o.lineage_id == c_lineage && d2 < nearest_ally_d2 {
                        nearest_ally_d2 = d2;
                        ally_dir = Some((dx, dy));
                    }
                }
                if base_role == CellRole::Protector || base_role == CellRole::Swimmer {
                    for &v_idx in &self.virion_buckets[b_idx] {
                        let v = &self.virions[v_idx];
                        if !v.alive {
                            continue;
                        }
                        let dx = v.x - cx;
                        let dy = v.y - cy;
                        let d2 = dx * dx + dy * dy;
                        if d2 < nearest_enemy_d2 {
                            nearest_enemy_d2 = d2;
                            enemy_dir = Some((dx, dy));
                        }
                    }
                }
            }
        }

        let enemy_prox = (1.0 - nearest_enemy_d2.sqrt() / 95.0).clamp(0.0, 1.0);
        let ally_prox = (1.0 - nearest_ally_d2.sqrt() / 95.0).clamp(0.0, 1.0);
        let crowding = (crowd_count / 7.0).clamp(0.0, 1.0);

        let (t_opt, t_tol, metabolism, armor, immune_res, neural_bias) = {
            let g = &self.cells[idx].genome;
            (
                g.t_opt,
                g.t_tol,
                g.metabolism,
                g.membrane_armor,
                g.immune_resistance,
                g.neural_bias,
            )
        };

        let temp_delta_norm = ((local_temp - t_opt) / t_tol.max(2.0)).clamp(-2.0, 2.0);
        let inputs: [f32; NN_INPUTS] = [
            local_nutrient.clamp(0.0, 1.0),
            temp_delta_norm,
            local_alarm.clamp(0.0, 1.0),
            local_food.clamp(0.0, 1.0),
            enemy_prox,
            ally_prox,
            (atp / max_atp.max(1.0)).clamp(0.0, 1.0),
            viral_load.clamp(0.0, 1.0),
            crowding,
            1.0,
        ];

        let outputs = self.cells[idx].brain.forward(inputs, neural_bias);

        // Compute chemotaxis steering hint combined with neural turn output.
        let mut chemotaxis_angle_bias = 0.0_f32;
        let current_angle = self.cells[idx].angle;
        if let Some((ex, ey)) = enemy_dir {
            let target_ang = ey.atan2(ex);
            let diff = wrap_angle(target_ang - current_angle);
            match base_role {
                CellRole::Protector | CellRole::Predator => {
                    chemotaxis_angle_bias += diff * 0.35;
                }
                _ => {
                    // Flee predators/infected!
                    chemotaxis_angle_bias -= diff * 0.25;
                }
            }
        } else if base_role == CellRole::Swimmer || base_role == CellRole::Producer {
            // Follow nutrient or food pheromone gradient.
            let (ngx, ngy) = self.fields.sample_gradient(0, cx, cy);
            let (fgx, fgy) = self.fields.sample_gradient(2, cx, cy);
            let gx = ngx + fgx * 0.8;
            let gy = ngy + fgy * 0.8;
            if gx * gx + gy * gy > 0.0001 {
                let target_ang = gy.atan2(gx);
                chemotaxis_angle_bias += wrap_angle(target_ang - current_angle) * 0.28;
            }
        } else if let Some((ax, ay)) = ally_dir {
            if self.cells[idx].genome.adhesion > 0.45 {
                let target_ang = ay.atan2(ax);
                chemotaxis_angle_bias += wrap_angle(target_ang - current_angle) * 0.12;
            }
        }

        // Q10 metabolic rate & thermal stress.
        let q10 = self.cells[idx].genome.q10_rate(local_temp);
        let stress = self.cells[idx].genome.thermal_stress(local_temp);

        // Movement from NN outputs[0] (thrust) and outputs[1] (turn).
        let base_speed = match base_role {
            CellRole::Swimmer => 2.55,
            CellRole::Predator => 2.05,
            CellRole::Protector => 1.95,
            CellRole::Replicator => 1.15,
            CellRole::Producer => 0.80,
            CellRole::Infected => 0.70,
        };
        let speed_scale = if eff_role == CellRole::Infected {
            0.55
        } else {
            1.0
        };
        let thrust =
            ((outputs[0] + 0.65) * 0.6).clamp(0.0, 1.35) * base_speed * q10.sqrt() * speed_scale;
        let turn = outputs[1] * 0.24 + chemotaxis_angle_bias + self.rng.normal_approx() * 0.03;

        let next_angle = wrap_angle(current_angle + turn * dt);
        let fx = next_angle.cos() * thrust;
        let fy = next_angle.sin() * thrust;

        // Energy (ATP) harvest, basal burn, and thermal shock damage.
        let taken_nutrient = match base_role {
            CellRole::Producer => self.fields.consume_nutrient(cx, cy, 0.022 * dt),
            CellRole::Swimmer | CellRole::Replicator => {
                self.fields.consume_nutrient(cx, cy, 0.016 * dt)
            }
            CellRole::Protector => self.fields.consume_nutrient(cx, cy, 0.014 * dt),
            CellRole::Predator => self.fields.consume_nutrient(cx, cy, 0.008 * dt),
            CellRole::Infected => 0.0,
        };
        let photosynth = if base_role == CellRole::Producer && eff_role != CellRole::Infected {
            // Photocytes gain steady light energy modulated by Q10 & low crowding.
            0.34 * q10 * (1.0 - crowding * 0.45) * dt
        } else {
            0.0
        };
        let atp_gain = taken_nutrient * 24.0 + photosynth;
        let basal_cost = (0.14 + thrust * 0.06 + armor * 0.05) * metabolism * q10 * dt;

        // Neural secretion outputs -> environmental pheromone & antibody fields!
        let mut signals = 0_u8;
        let mut secretion_atp_cost = 0.0_f32;
        if outputs[3] > 0.22 && atp > 18.0 {
            self.fields.add_alarm(cx, cy, 0.14 * dt);
            signals |= SIGNAL_ALARM;
            secretion_atp_cost += 0.04 * dt;
        }
        if outputs[4] > 0.22 && atp > 18.0 && local_nutrient > 0.25 {
            self.fields.add_food_pheromone(cx, cy, 0.12 * dt);
            signals |= SIGNAL_FOOD;
            secretion_atp_cost += 0.03 * dt;
        }
        if base_role == CellRole::Protector && outputs[5] > 0.12 && atp > 22.0 {
            self.fields.add_toxin(cx, cy, 0.16 * dt);
            signals |= SIGNAL_ANTIBODY;
            secretion_atp_cost += 0.06 * dt;
        }

        // Update cell physics, ATP, HP, and viral incubation.
        let mut should_divide = false;
        let mut should_lyse_strain: Option<VirusStrain> = None;
        let current_cells_len = self.cells.len();

        {
            let c = &mut self.cells[idx];
            c.age = c.age.saturating_add(1);
            c.divide_cooldown = c.divide_cooldown.saturating_sub(1);
            c.angle = next_angle;
            c.vx = (c.vx + fx * 0.28 * dt) * 0.82;
            c.vy = (c.vy + fy * 0.28 * dt) * 0.82;
            c.x = (c.x + c.vx * dt).clamp(c.radius, WORLD_W - c.radius);
            c.y = (c.y + c.vy * dt).clamp(c.radius, WORLD_H - c.radius);
            if c.x <= c.radius + 0.5 || c.x >= WORLD_W - c.radius - 0.5 {
                c.vx *= -0.7;
            }
            if c.y <= c.radius + 0.5 || c.y >= WORLD_H - c.radius - 0.5 {
                c.vy *= -0.7;
            }

            c.temp_stress = stress;
            c.q10_rate = q10;
            c.signal_flags = signals;

            c.atp = (c.atp + atp_gain - basal_cost - secretion_atp_cost).clamp(0.0, c.max_atp);

            // Heat-shock / Frostbite damage outside thermal tolerance!
            if stress.abs() > 0.02 {
                let thermal_dmg = stress.abs() * 0.55 * (1.0 - armor * 0.35) * dt;
                c.hp -= thermal_dmg;
            }

            // Starvation or toxin tolerance check.
            if c.atp <= 0.01 {
                c.hp -= 0.45 * dt;
            } else if c.hp < c.max_hp && c.atp > c.max_atp * 0.35 && stress.abs() < 0.1 {
                c.hp = (c.hp + 0.14 * dt).min(c.max_hp);
            }
            if local_toxin > 0.35 && base_role == CellRole::Predator {
                c.hp -= (local_toxin - 0.35) * 0.35 * (1.0 - armor * 0.5) * dt;
            }

            // Viral incubation & CRISPR/antibody suppression.
            if let Some(strain) = &c.infected_by {
                // Host ATP is hijacked to replicate viral particles.
                c.atp = (c.atp - 0.16 * dt).max(0.0);
                // Antibodies + host CRISPR resistance suppress viral replication!
                let suppression =
                    local_toxin * 0.55 + immune_res * (1.0 - strain.immune_evasion) * 0.45;
                if suppression > 0.55 {
                    c.viral_load = (c.viral_load - 0.018 * dt).max(0.0);
                    if c.viral_load <= 0.01 {
                        c.infected_by = None;
                        c.incubation_progress = 0.0;
                    }
                } else {
                    let progress_step = (q10 / strain.incubation_ticks.max(20.0)) * dt;
                    c.incubation_progress += progress_step;
                    c.viral_load = (c.viral_load + progress_step * 0.95).clamp(0.0, 1.0);
                    c.hp -= strain.lethality * 0.24 * dt;

                    if c.incubation_progress >= 1.0 || (c.hp <= 0.0 && c.viral_load > 0.35) {
                        should_lyse_strain = Some(strain.clone());
                        c.alive = false;
                    }
                }
            }

            if c.hp <= 0.0 || c.age > 9500 {
                c.alive = false;
            }

            // Mitosis check: healthy, uninfected, off cooldown, sufficient ATP, neural urge > threshold.
            let divide_thresh = match base_role {
                CellRole::Replicator => c.max_atp * 0.58,
                _ => c.max_atp * 0.74,
            };
            if c.alive
                && c.infected_by.is_none()
                && c.divide_cooldown == 0
                && c.atp >= divide_thresh
                && outputs[2] > 0.05
                && current_cells_len + new_offspring.len() < MAX_CELLS
            {
                should_divide = true;
                c.atp *= 0.48;
                c.divide_cooldown = match base_role {
                    CellRole::Replicator => 55,
                    _ => 95,
                };
                c.signal_flags |= SIGNAL_DIVIDE;
            }
        }

        if let Some(strain) = should_lyse_strain {
            self.total_bursts = self.total_bursts.saturating_add(1);
            // Emit alarm pheromone on cell lysis so nearby Protectors swarm!
            self.fields.add_alarm(cx, cy, 0.55);
            for _ in 0..strain.burst_count {
                let next_sid = self.alloc_strain_id();
                let child_strain = strain.mutated_offspring(next_sid, &mut self.rng);
                let ang = self.rng.range_f32(0.0, std::f32::consts::TAU);
                new_virions.push(Virion {
                    x: (cx + ang.cos() * 6.0).clamp(6.0, WORLD_W - 6.0),
                    y: (cy + ang.sin() * 6.0).clamp(6.0, WORLD_H - 6.0),
                    vx: ang.cos() * 2.4,
                    vy: ang.sin() * 2.4,
                    age: 0.0,
                    max_age: self.rng.range_f32(420.0, 760.0),
                    strain: child_strain,
                    alive: true,
                });
            }
        }

        if should_divide {
            let parent = self.cells[idx].clone();
            let mut child_genome = parent.genome.clone();
            child_genome.mutate(&mut self.rng);
            let mut child_brain = parent.brain.clone();
            child_brain.mutate(&mut self.rng, parent.genome.mutation_rate, 0.25);

            // Replicators (stem cells) can occasionally differentiate if ecosystem needs diversity,
            // otherwise preserve lineage role.
            let child_role = if parent.base_role == CellRole::Replicator && self.rng.chance(0.08) {
                match self.rng.next_u32() % 4 {
                    0 => CellRole::Producer,
                    1 => CellRole::Protector,
                    2 => CellRole::Swimmer,
                    _ => CellRole::Replicator,
                }
            } else {
                parent.base_role
            };
            child_genome.role = child_role;

            let off_ang =
                parent.angle + std::f32::consts::PI * 0.5 + self.rng.normal_approx() * 0.4;
            let dist = parent.radius * 1.65;
            let child_id = self.alloc_cell_id();
            let max_hp = 85.0 + child_genome.membrane_armor * 75.0;
            let child = Cell {
                id: child_id,
                lineage_id: parent.lineage_id,
                parent_id: c_id,
                generation: parent.generation.saturating_add(1),
                x: (parent.x + off_ang.cos() * dist).clamp(parent.radius, WORLD_W - parent.radius),
                y: (parent.y + off_ang.sin() * dist).clamp(parent.radius, WORLD_H - parent.radius),
                vx: off_ang.cos() * 0.9,
                vy: off_ang.sin() * 0.9,
                angle: off_ang,
                radius: child_role.base_radius(),
                hp: max_hp,
                max_hp,
                atp: parent.atp * 0.95,
                max_atp: parent.max_atp,
                age: 0,
                divide_cooldown: 65,
                base_role: child_role,
                genome: child_genome,
                brain: child_brain,
                viral_load: 0.0,
                incubation_progress: 0.0,
                infected_by: None,
                temp_stress: parent.temp_stress,
                q10_rate: parent.q10_rate,
                signal_flags: SIGNAL_DIVIDE,
                alive: true,
            };
            new_offspring.push(child);
        }
    }

    fn step_cell_interactions(&mut self, dt: f32) {
        let n = self.cells.len();
        for i in 0..n {
            if !self.cells[i].alive {
                continue;
            }
            let (ax, ay, ar, a_role, a_eff, a_lin) = {
                let c = &self.cells[i];
                (
                    c.x,
                    c.y,
                    c.radius,
                    c.base_role,
                    c.effective_role(),
                    c.lineage_id,
                )
            };

            let gx = ((ax / HASH_CELL) as isize).clamp(0, HASH_W as isize - 1) as usize;
            let gy = ((ay / HASH_CELL) as isize).clamp(0, HASH_H as isize - 1) as usize;

            for ny in gy.saturating_sub(1)..=(gy + 1).min(HASH_H - 1) {
                for nx in gx.saturating_sub(1)..=(gx + 1).min(HASH_W - 1) {
                    let b_idx = ny * HASH_W + nx;
                    for &j in &self.cell_buckets[b_idx] {
                        if j <= i || !self.cells[j].alive {
                            continue;
                        }
                        let (bx, by, br, b_role, b_eff, b_lin, b_armor) = {
                            let o = &self.cells[j];
                            (
                                o.x,
                                o.y,
                                o.radius,
                                o.base_role,
                                o.effective_role(),
                                o.lineage_id,
                                o.genome.membrane_armor,
                            )
                        };
                        let dx = bx - ax;
                        let dy = by - ay;
                        let d2 = dx * dx + dy * dy;
                        let min_d = ar + br;

                        // Soft-body elastic collision separation
                        if d2 > 0.0001 && d2 < min_d * min_d {
                            let d = d2.sqrt();
                            let overlap = min_d - d;
                            let nx_dir = dx / d;
                            let ny_dir = dy / d;
                            let push = overlap * 0.25;
                            self.cells[i].x =
                                (self.cells[i].x - nx_dir * push).clamp(ar, WORLD_W - ar);
                            self.cells[i].y =
                                (self.cells[i].y - ny_dir * push).clamp(ar, WORLD_H - ar);
                            self.cells[j].x =
                                (self.cells[j].x + nx_dir * push).clamp(br, WORLD_W - br);
                            self.cells[j].y =
                                (self.cells[j].y + ny_dir * push).clamp(br, WORLD_H - br);
                        }

                        // Protector / Immunocyte actions within immune range (28 px)
                        if d2 < 28.0 * 28.0 {
                            if a_role == CellRole::Protector && a_eff != CellRole::Infected {
                                if b_eff == CellRole::Infected {
                                    self.cells[j].hp -= 3.2 * dt;
                                    self.cells[j].viral_load =
                                        (self.cells[j].viral_load - 0.04 * dt).max(0.0);
                                    if self.cells[j].hp <= 0.0 {
                                        self.cells[j].alive = false;
                                        self.total_neutralized =
                                            self.total_neutralized.saturating_add(1);
                                    }
                                } else if b_role != CellRole::Predator
                                    && self.cells[j].hp < self.cells[j].max_hp
                                {
                                    self.cells[j].hp =
                                        (self.cells[j].hp + 0.35 * dt).min(self.cells[j].max_hp);
                                }
                            }
                            if b_role == CellRole::Protector && b_eff != CellRole::Infected {
                                if a_eff == CellRole::Infected {
                                    self.cells[i].hp -= 3.2 * dt;
                                    self.cells[i].viral_load =
                                        (self.cells[i].viral_load - 0.04 * dt).max(0.0);
                                    if self.cells[i].hp <= 0.0 {
                                        self.cells[i].alive = false;
                                        self.total_neutralized =
                                            self.total_neutralized.saturating_add(1);
                                    }
                                } else if a_role != CellRole::Predator
                                    && self.cells[i].hp < self.cells[i].max_hp
                                {
                                    self.cells[i].hp =
                                        (self.cells[i].hp + 0.35 * dt).min(self.cells[i].max_hp);
                                }
                            }
                        }

                        // Predator engulfment / phagocytosis on contact
                        if d2 < (min_d + 4.0) * (min_d + 4.0) {
                            if a_role == CellRole::Predator
                                && a_eff != CellRole::Infected
                                && b_lin != a_lin
                                && b_role != CellRole::Predator
                            {
                                let dmg = 5.5 * (1.0 - b_armor * 0.65) * dt;
                                self.cells[j].hp -= dmg;
                                self.cells[i].atp =
                                    (self.cells[i].atp + dmg * 0.95).min(self.cells[i].max_atp);
                                if self.cells[j].hp <= 0.0 {
                                    self.cells[j].alive = false;
                                    self.cells[i].atp =
                                        (self.cells[i].atp + 28.0).min(self.cells[i].max_atp);
                                }
                            } else if b_role == CellRole::Predator
                                && b_eff != CellRole::Infected
                                && a_lin != b_lin
                                && a_role != CellRole::Predator
                            {
                                let a_armor = self.cells[i].genome.membrane_armor;
                                let dmg = 5.5 * (1.0 - a_armor * 0.65) * dt;
                                self.cells[i].hp -= dmg;
                                self.cells[j].atp =
                                    (self.cells[j].atp + dmg * 0.95).min(self.cells[j].max_atp);
                                if self.cells[i].hp <= 0.0 {
                                    self.cells[i].alive = false;
                                    self.cells[j].atp =
                                        (self.cells[j].atp + 28.0).min(self.cells[j].max_atp);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn step_virions(&mut self, dt: f32) {
        for v_idx in 0..self.virions.len() {
            if !self.virions[v_idx].alive {
                continue;
            }
            let (vx, vy, age, max_age, strain) = {
                let v = &self.virions[v_idx];
                (v.x, v.y, v.age, v.max_age, v.strain.clone())
            };

            let local_toxin = self.fields.sample_toxin(vx, vy);
            if age >= max_age || local_toxin > 0.24 {
                self.virions[v_idx].alive = false;
                if local_toxin > 0.24 {
                    self.total_neutralized = self.total_neutralized.saturating_add(1);
                }
                continue;
            }

            // Brownian drift + mild chemotaxis toward nearest susceptible cell.
            let gx = ((vx / HASH_CELL) as isize).clamp(0, HASH_W as isize - 1) as usize;
            let gy = ((vy / HASH_CELL) as isize).clamp(0, HASH_H as isize - 1) as usize;
            let mut attract_x = 0.0_f32;
            let mut attract_y = 0.0_f32;
            let mut attached = false;

            'bucket_loop: for ny in gy.saturating_sub(1)..=(gy + 1).min(HASH_H - 1) {
                for nx in gx.saturating_sub(1)..=(gx + 1).min(HASH_W - 1) {
                    let b_idx = ny * HASH_W + nx;
                    for &c_idx in &self.cell_buckets[b_idx] {
                        let c = &mut self.cells[c_idx];
                        if !c.alive {
                            continue;
                        }
                        let dx = c.x - vx;
                        let dy = c.y - vy;
                        let d2 = dx * dx + dy * dy;

                        // Protector destroys adjacent virion immediately!
                        if c.base_role == CellRole::Protector
                            && c.infected_by.is_none()
                            && d2 < (c.radius + 14.0) * (c.radius + 14.0)
                        {
                            self.virions[v_idx].alive = false;
                            self.total_neutralized = self.total_neutralized.saturating_add(1);
                            self.fields.add_alarm(c.x, c.y, 0.25);
                            attached = true;
                            break 'bucket_loop;
                        }

                        if c.infected_by.is_none() && d2 < (c.radius + 5.0) * (c.radius + 5.0) {
                            // Infection probability gated by host CRISPR/immune resistance vs viral evasion!
                            let effective_res =
                                c.genome.immune_resistance * (1.0 - strain.immune_evasion);
                            let p_infect =
                                (strain.infectivity * (1.0 - effective_res)).clamp(0.03, 0.98);
                            if self.rng.chance(p_infect) {
                                c.infected_by = Some(strain.clone());
                                c.viral_load = 0.18;
                                c.incubation_progress = 0.05;
                                self.fields.add_alarm(c.x, c.y, 0.30);
                            } else {
                                // CRISPR immune defense neutralized the virion!
                                self.total_neutralized = self.total_neutralized.saturating_add(1);
                            }
                            self.virions[v_idx].alive = false;
                            attached = true;
                            break 'bucket_loop;
                        } else if c.infected_by.is_none() && d2 < 3200.0 && d2 > 1.0 {
                            let inv = 1.0 / d2.sqrt();
                            attract_x += dx * inv;
                            attract_y += dy * inv;
                        }
                    }
                }
            }

            if !attached {
                let v = &mut self.virions[v_idx];
                v.age += dt;
                v.vx = (v.vx + attract_x * 0.18 + self.rng.normal_approx() * 0.35) * 0.90;
                v.vy = (v.vy + attract_y * 0.18 + self.rng.normal_approx() * 0.35) * 0.90;
                v.x = (v.x + v.vx * dt).clamp(6.0, WORLD_W - 6.0);
                v.y = (v.y + v.vy * dt).clamp(6.0, WORLD_H - 6.0);
            }
        }
    }

    fn update_adhesion_bonds(&mut self, dt: f32) {
        // Build id -> index lookup sorted for O(log N) lookup.
        self.id_to_idx.clear();
        for (i, c) in self.cells.iter().enumerate() {
            if c.alive {
                self.id_to_idx.push((c.id, i));
            }
        }
        self.id_to_idx.sort_unstable_by_key(|&(id, _)| id);

        let find_idx = |map: &[(u32, usize)], target_id: u32| -> Option<usize> {
            map.binary_search_by_key(&target_id, |&(id, _)| id)
                .ok()
                .map(|pos| map[pos].1)
        };

        // Update existing bonds: Hooke spring physics + ATP nutrient sharing.
        let mut kept_bonds = Vec::with_capacity(self.bonds.len());
        for bond in &self.bonds {
            let (Some(ia), Some(ib)) = (
                find_idx(&self.id_to_idx, bond.a_id),
                find_idx(&self.id_to_idx, bond.b_id),
            ) else {
                continue;
            };
            if ia == ib {
                continue;
            }
            let dx = self.cells[ib].x - self.cells[ia].x;
            let dy = self.cells[ib].y - self.cells[ia].y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > bond.rest_len * 2.4 || dist < 0.001 {
                continue;
            }
            let stretch = dist - bond.rest_len;
            let force = stretch * 0.08 * bond.strength * dt;
            let fx = (dx / dist) * force;
            let fy = (dy / dist) * force;
            self.cells[ia].vx += fx;
            self.cells[ia].vy += fy;
            self.cells[ib].vx -= fx;
            self.cells[ib].vy -= fy;

            // Share ATP across multicellular tissue junctions!
            let atp_diff = self.cells[ia].atp - self.cells[ib].atp;
            let transfer = atp_diff * 0.04 * dt;
            self.cells[ia].atp -= transfer;
            self.cells[ib].atp += transfer;

            kept_bonds.push(*bond);
        }
        self.bonds = kept_bonds;

        // Form new adhesion bonds between nearby adhesive cells (adhesion > 0.45).
        let n = self.cells.len();
        for i in 0..n {
            if self.bonds.len() >= MAX_BONDS {
                break;
            }
            let ca = &self.cells[i];
            if !ca.alive || ca.genome.adhesion <= 0.45 || ca.base_role == CellRole::Predator {
                continue;
            }
            let gx = ((ca.x / HASH_CELL) as isize).clamp(0, HASH_W as isize - 1) as usize;
            let gy = ((ca.y / HASH_CELL) as isize).clamp(0, HASH_H as isize - 1) as usize;
            let mut local_links = 0_u8;
            for b in &self.bonds {
                if b.a_id == ca.id || b.b_id == ca.id {
                    local_links += 1;
                }
            }
            if local_links >= 4 {
                continue;
            }

            for ny in gy.saturating_sub(1)..=(gy + 1).min(HASH_H - 1) {
                for nx in gx.saturating_sub(1)..=(gx + 1).min(HASH_W - 1) {
                    if self.bonds.len() >= MAX_BONDS || local_links >= 4 {
                        break;
                    }
                    let b_idx = ny * HASH_W + nx;
                    for &j in &self.cell_buckets[b_idx] {
                        if j <= i {
                            continue;
                        }
                        let cb = &self.cells[j];
                        if !cb.alive
                            || cb.genome.adhesion <= 0.45
                            || cb.base_role == CellRole::Predator
                        {
                            continue;
                        }
                        let dx = cb.x - ca.x;
                        let dy = cb.y - ca.y;
                        let max_bind = ca.radius + cb.radius + 14.0;
                        if dx * dx + dy * dy <= max_bind * max_bind {
                            let exists = self.bonds.iter().any(|b| {
                                (b.a_id == ca.id && b.b_id == cb.id)
                                    || (b.a_id == cb.id && b.b_id == ca.id)
                            });
                            if !exists {
                                self.bonds.push(AdhesionBond {
                                    a_id: ca.id,
                                    b_id: cb.id,
                                    rest_len: ca.radius + cb.radius + 4.0,
                                    strength: (ca.genome.adhesion + cb.genome.adhesion) * 0.5,
                                });
                                local_links += 1;
                                if self.bonds.len() >= MAX_BONDS || local_links >= 4 {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn rebuild_spatial_hashes(&mut self) {
        for b in &mut self.cell_buckets {
            b.clear();
        }
        for b in &mut self.virion_buckets {
            b.clear();
        }
        for (i, c) in self.cells.iter().enumerate() {
            if !c.alive {
                continue;
            }
            let gx = ((c.x / HASH_CELL) as isize).clamp(0, HASH_W as isize - 1) as usize;
            let gy = ((c.y / HASH_CELL) as isize).clamp(0, HASH_H as isize - 1) as usize;
            self.cell_buckets[gy * HASH_W + gx].push(i);
        }
        for (i, v) in self.virions.iter().enumerate() {
            if !v.alive {
                continue;
            }
            let gx = ((v.x / HASH_CELL) as isize).clamp(0, HASH_W as isize - 1) as usize;
            let gy = ((v.y / HASH_CELL) as isize).clamp(0, HASH_H as isize - 1) as usize;
            self.virion_buckets[gy * HASH_W + gx].push(i);
        }
    }

    pub fn rebuild_render_buffers(&mut self) {
        self.cells_buf.clear();
        self.virions_buf.clear();
        self.bonds_buf.clear();

        let mut role_counts = [0_u32; 6];
        let mut sum_t_opt = 0.0_f32;
        let mut sum_immune = 0.0_f32;
        let mut sum_adhesion = 0.0_f32;
        let mut sum_atp = 0.0_f32;
        let mut max_gen = 0_u32;
        let mut selected_idx: Option<usize> = None;

        for (i, c) in self.cells.iter().enumerate() {
            if !c.alive {
                continue;
            }
            let eff = c.effective_role();
            role_counts[eff as usize] += 1;
            sum_t_opt += c.genome.t_opt;
            sum_immune += c.genome.immune_resistance;
            sum_adhesion += c.genome.adhesion;
            sum_atp += c.atp / c.max_atp.max(1.0);
            if c.generation > max_gen {
                max_gen = c.generation;
            }
            if c.id == self.selected_cell_id {
                selected_idx = Some(i);
            }

            self.cells_buf.extend_from_slice(&[
                c.id as f32,
                c.x,
                c.y,
                c.angle,
                c.radius,
                eff as u8 as f32,
                (c.hp / c.max_hp.max(1.0)).clamp(0.0, 1.0),
                (c.atp / c.max_atp.max(1.0)).clamp(0.0, 1.0),
                c.viral_load,
                c.temp_stress,
                c.genome.adhesion,
                c.genome.hue_shift,
                c.generation as f32,
                c.signal_flags as f32,
                c.genome.immune_resistance,
                c.lineage_id as f32,
            ]);
        }

        for v in &self.virions {
            if !v.alive {
                continue;
            }
            self.virions_buf
                .extend_from_slice(&[v.x, v.y, v.vx, v.vy, v.strain.strain_hue]);
        }

        // Rebuild id->idx for bond coordinates
        self.id_to_idx.clear();
        for (i, c) in self.cells.iter().enumerate() {
            if c.alive {
                self.id_to_idx.push((c.id, i));
            }
        }
        self.id_to_idx.sort_unstable_by_key(|&(id, _)| id);
        for b in &self.bonds {
            if let (Ok(pa), Ok(pb)) = (
                self.id_to_idx.binary_search_by_key(&b.a_id, |&(id, _)| id),
                self.id_to_idx.binary_search_by_key(&b.b_id, |&(id, _)| id),
            ) {
                let ca = &self.cells[self.id_to_idx[pa].1];
                let cb = &self.cells[self.id_to_idx[pb].1];
                self.bonds_buf
                    .extend_from_slice(&[ca.x, ca.y, cb.x, cb.y, b.strength]);
            }
        }

        let pop = self.cells.len() as f32;
        let inv_pop = if pop > 0.0 { 1.0 / pop } else { 0.0 };
        self.stats_buf = [
            self.tick as f32,
            pop,
            role_counts[0] as f32,
            role_counts[1] as f32,
            role_counts[2] as f32,
            role_counts[3] as f32,
            role_counts[4] as f32,
            role_counts[5] as f32,
            self.virions.len() as f32,
            (self.bonds_buf.len() / BOND_STRIDE) as f32,
            sum_t_opt * inv_pop,
            sum_immune * inv_pop,
            sum_adhesion * inv_pop,
            sum_atp * inv_pop,
            self.fields.mean_temp(),
            max_gen as f32,
            self.total_births as f32,
            self.total_bursts as f32,
            self.total_neutralized as f32,
            if selected_idx.is_some() { 1.0 } else { 0.0 },
        ];

        self.inspector_buf.fill(0.0);
        if let Some(s_idx) = selected_idx {
            let c = &self.cells[s_idx];
            let local_temp = self.fields.sample_temp(c.x, c.y);
            let meta = [
                1.0,
                c.id as f32,
                c.effective_role() as u8 as f32,
                c.base_role as u8 as f32,
                c.generation as f32,
                c.lineage_id as f32,
                c.hp,
                c.max_hp,
                c.atp,
                c.max_atp,
                c.age as f32,
                local_temp,
                c.genome.t_opt,
                c.genome.t_tol,
                c.genome.metabolism,
                c.genome.membrane_armor,
                c.genome.adhesion,
                c.genome.immune_resistance,
                c.genome.mutation_rate,
                c.genome.neural_bias,
            ];
            self.inspector_buf[..20].copy_from_slice(&meta);
            self.inspector_buf[20] = c.viral_load;
            self.inspector_buf[21] = c.incubation_progress;
            self.inspector_buf[22] = c.temp_stress;
            self.inspector_buf[23] = c.q10_rate;

            let mut off = 24;
            self.inspector_buf[off..off + NN_INPUTS].copy_from_slice(&c.brain.last_inputs);
            off += NN_INPUTS;
            self.inspector_buf[off..off + NN_HIDDEN].copy_from_slice(&c.brain.last_hidden);
            off += NN_HIDDEN;
            self.inspector_buf[off..off + NN_OUTPUTS].copy_from_slice(&c.brain.last_outputs);
            off += NN_OUTPUTS;
            self.inspector_buf[off..off + NN_W1_LEN].copy_from_slice(&c.brain.w1);
            off += NN_W1_LEN;
            self.inspector_buf[off..off + NN_W2_LEN].copy_from_slice(&c.brain.w2);
        }
    }
}

#[inline]
fn wrap_angle(mut a: f32) -> f32 {
    while a > std::f32::consts::PI {
        a -= std::f32::consts::TAU;
    }
    while a < -std::f32::consts::PI {
        a += std::f32::consts::TAU;
    }
    a
}
