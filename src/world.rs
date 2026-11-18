//! Continuous biological world, spatial hash grid, Cell Lab 8-mode differentiation,
//! vascular & synaptic bonds, 3-strategy virology, CRISPR immunity, and mutagenesis.

use crate::fields::EnvFields;
use crate::genome::{
    CellMode, CellRole, OrganismGenome, MAX_MODES, NUM_CELL_ROLES, SECRETION_ALARM,
    SECRETION_ANTIBODY, SECRETION_FOOD_PHEROMONE, SECRETION_NUTRIENT_MUCUS, SECRETION_TOXIN,
};
use crate::mutation::{
    apply_mutation_rules, apply_single_mutation_rule, MutationOutcome, MutationRule,
    MUT_TRIGGER_DIRECT_ONLY, MUT_TRIGGER_ON_MITOSIS, MUT_TRIGGER_ON_MUTAGEN_FIELD,
    MUT_TRIGGER_ON_THERMAL_STRESS,
};
use crate::neural::{
    CognitiveState, NeuralNet, NN_HIDDEN, NN_INPUTS, NN_MEMORY, NN_OUTPUTS, NN_W1_LEN, NN_W2_LEN,
    NN_W_MEM_LEN,
};
use crate::rng::Rng;
use crate::virus::{ViralStrategy, Virion, VirusStrain};
use core::f32::consts::{PI, TAU};

pub const WORLD_W: f32 = 1200.0;
pub const WORLD_H: f32 = 800.0;
pub const GRID_W: usize = 120;
pub const GRID_H: usize = 80;
pub const FIELD_CELL_SIZE: f32 = 10.0;

pub const MAX_CELLS: usize = 900;
pub const MAX_VIRIONS: usize = 1400;
pub const MAX_BONDS: usize = 1400;
pub const MAX_MUTATION_LOG: usize = 64;

pub const CELL_STRIDE: usize = 20;
pub const VIRION_STRIDE: usize = 6;
pub const BOND_STRIDE: usize = 7;
pub const MUTATION_LOG_STRIDE: usize = 6;
pub const STATS_LEN: usize = 32;
pub const INSPECTOR_LEN: usize =
    32 + NN_INPUTS + NN_HIDDEN + NN_MEMORY + NN_OUTPUTS + NN_W1_LEN + NN_W_MEM_LEN + NN_W2_LEN;

pub const SIGNAL_ALARM: u8 = 1;
pub const SIGNAL_FOOD: u8 = 2;
pub const SIGNAL_ANTIBODY: u8 = 4;
pub const SIGNAL_DIVIDE: u8 = 8;
pub const SIGNAL_SYNAPSE: u8 = 16;
pub const SIGNAL_MUTATED: u8 = 32;

const HASH_CELL: f32 = 40.0;
const HASH_W: usize = 30;
const HASH_H: usize = 20;

#[derive(Clone, Debug)]
pub struct Cell {
    pub id: u32,
    pub organism_id: u32,
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
    pub mode_idx: u8,
    pub genome: OrganismGenome,
    pub brain: NeuralNet,
    pub synaptic_pulse: f32,
    pub incoming_synaptic: f32,
    pub myocyte_flex: f32,
    pub crispr_mask: u16,
    pub viral_load: f32,
    pub incubation_progress: f32,
    pub infected_by: Option<VirusStrain>,
    pub lysogenic_dormant: bool,
    pub temp_stress: f32,
    pub q10_rate: f32,
    pub signal_flags: u8,
    pub bonded_count: u8,
    pub keratin_shield: f32,
    pub alive: bool,
}

impl Cell {
    #[inline]
    pub fn mode(&self) -> &CellMode {
        self.genome.mode(self.mode_idx)
    }

    #[inline]
    pub fn role(&self) -> CellRole {
        self.genome.mode(self.mode_idx).role
    }

    #[inline]
    pub fn is_actively_infected(&self) -> bool {
        self.infected_by.is_some() && (!self.lysogenic_dormant || self.viral_load > 0.20)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AdhesionBond {
    pub a_id: u32,
    pub b_id: u32,
    pub rest_len: f32,
    pub stiffness: f32,
    pub preferred_angle: f32,
    pub last_vascular_flow: f32,
    pub last_synaptic_flow: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct MutationLogEntry {
    pub tick: u32,
    pub organism_id: u32,
    pub role: u8,
    pub target_kind: u8,
    pub delta: f32,
    pub mode_idx: u8,
}

struct PendingMitosis {
    parent_idx: usize,
    parent_id: u32,
    child_a_mode: u8,
    child_b_mode: u8,
    split_angle: f32,
    keep_adhesion_parent: bool,
    bond_children: bool,
    adhesion_stiffness: f32,
}

pub struct World {
    pub fields: EnvFields,
    pub cells: Vec<Cell>,
    pub virions: Vec<Virion>,
    pub bonds: Vec<AdhesionBond>,
    pub genomes: Vec<OrganismGenome>,
    pub strains: Vec<VirusStrain>,
    pub mutation_rules: Vec<MutationRule>,
    pub mutation_log: Vec<MutationLogEntry>,

    pub rng: Rng,
    pub tick: u32,
    pub scenario: u8,

    pub next_cell_id: u32,
    pub next_organism_id: u32,
    pub next_lineage_id: u32,
    pub next_genome_id: u32,
    pub next_strain_id: u32,
    pub next_rule_id: u32,
    pub selected_cell_id: u32,

    pub total_births: u32,
    pub total_bursts: u32,
    pub total_neutralized: u32,
    pub total_mutations: u32,

    cell_buckets: Vec<Vec<usize>>,
    virion_buckets: Vec<Vec<usize>>,
    id_to_idx: Vec<(u32, usize)>,

    pub cells_buf: Vec<f32>,
    pub virions_buf: Vec<f32>,
    pub bonds_buf: Vec<f32>,
    pub mutation_log_buf: Vec<f32>,
    pub stats_buf: [f32; STATS_LEN],
    pub inspector_buf: [f32; INSPECTOR_LEN],
    pub genome_mode_buf: [f32; 20],
}

impl World {
    pub fn new(seed: u32, scenario: u8) -> Self {
        let mut world = Self {
            fields: EnvFields::new(GRID_W, GRID_H, FIELD_CELL_SIZE, 25.0),
            cells: Vec::with_capacity(MAX_CELLS),
            virions: Vec::with_capacity(MAX_VIRIONS),
            bonds: Vec::with_capacity(MAX_BONDS),
            genomes: Vec::with_capacity(16),
            strains: Vec::with_capacity(16),
            mutation_rules: Vec::with_capacity(16),
            mutation_log: Vec::with_capacity(MAX_MUTATION_LOG),
            rng: Rng::new(seed as u64 + 1),
            tick: 0,
            scenario,
            next_cell_id: 1,
            next_organism_id: 1,
            next_lineage_id: 1,
            next_genome_id: 1,
            next_strain_id: 1,
            next_rule_id: 1,
            selected_cell_id: 0,
            total_births: 0,
            total_bursts: 0,
            total_neutralized: 0,
            total_mutations: 0,
            cell_buckets: (0..HASH_W * HASH_H)
                .map(|_| Vec::with_capacity(12))
                .collect(),
            virion_buckets: (0..HASH_W * HASH_H)
                .map(|_| Vec::with_capacity(12))
                .collect(),
            id_to_idx: Vec::with_capacity(MAX_CELLS),
            cells_buf: Vec::with_capacity(MAX_CELLS * CELL_STRIDE),
            virions_buf: Vec::with_capacity(MAX_VIRIONS * VIRION_STRIDE),
            bonds_buf: Vec::with_capacity(MAX_BONDS * BOND_STRIDE),
            mutation_log_buf: Vec::with_capacity(MAX_MUTATION_LOG * MUTATION_LOG_STRIDE),
            stats_buf: [0.0; STATS_LEN],
            inspector_buf: [0.0; INSPECTOR_LEN],
            genome_mode_buf: [0.0; 20],
        };
        world.init_default_registries();
        world.reset_scenario(scenario);
        world
    }

    fn init_default_registries(&mut self) {
        self.genomes.clear();
        self.next_genome_id = 1;
        for preset_id in 0..5 {
            let mut g = OrganismGenome::preset(preset_id);
            g.id = self.alloc_genome_id();
            self.genomes.push(g);
        }
        // Slot 5 (id=6): customizable working genome slot initialized to Solar Reef Symbiont.
        let mut custom_g = OrganismGenome::preset(1);
        custom_g.id = self.alloc_genome_id();
        self.genomes.push(custom_g);

        self.strains.clear();
        self.next_strain_id = 1;
        for p in 0..3 {
            let sid = self.alloc_strain_id();
            self.strains.push(VirusStrain::preset(sid, p));
        }

        self.mutation_rules.clear();
        self.next_rule_id = 1;
        for r in 0..4 {
            let rid = self.alloc_rule_id();
            self.mutation_rules.push(MutationRule::preset(rid, r));
        }
    }

    pub fn reset_scenario(&mut self, scenario: u8) {
        self.scenario = scenario;
        self.cells.clear();
        self.virions.clear();
        self.bonds.clear();
        self.mutation_log.clear();
        self.tick = 0;
        self.total_births = 0;
        self.total_bursts = 0;
        self.total_neutralized = 0;
        self.total_mutations = 0;
        self.selected_cell_id = 0;

        if self.genomes.len() < 5 || self.strains.len() < 3 || self.mutation_rules.is_empty() {
            self.init_default_registries();
        }

        match scenario {
            // 0: Conscious Reef & Apex Leviathans
            0 => {
                self.fields.reset(25.0, 0.68);
                self.fields.nutrient_floor = 0.54;
                self.fields
                    .add_thermal_source(240.0, 220.0, 140.0, 38.0, 0.14);
                self.fields
                    .add_thermal_source(960.0, 580.0, 140.0, 14.0, 0.14);
                self.fields.add_light_source(600.0, 220.0, 260.0, 1.25);

                // Spawn multi-mode conscious organisms + single-role support colonies
                self.spawn_organism(2, 380.0, 340.0, 6, 95.0); // Solar Reef Symbiont
                self.spawn_organism(2, 820.0, 440.0, 6, 95.0);
                self.spawn_organism(5, 600.0, 540.0, 5, 90.0); // Segmented Muscle Worm
                self.spawn_organism(3, 520.0, 240.0, 5, 85.0); // Swarm Valkyrie
                self.spawn_organism(1, 240.0, 540.0, 3, 75.0); // Apex Leviathan
                self.spawn_organism(4, 920.0, 240.0, 4, 80.0); // Macrophage Sentinel
                self.spawn_role_cluster(600.0, 380.0, CellRole::Photocyte, 18, 180.0);
                self.spawn_role_cluster(600.0, 420.0, CellRole::Scavenger, 8, 160.0);
            }
            // 1: Multi-Strain Pandemic vs Adaptive Immunity
            1 => {
                self.fields.reset(27.0, 0.70);
                self.fields.nutrient_floor = 0.56;
                self.fields.add_light_source(600.0, 360.0, 300.0, 1.15);

                self.spawn_organism(2, 580.0, 400.0, 7, 130.0); // Solar Reef Symbiont
                self.spawn_organism(4, 600.0, 400.0, 8, 140.0); // Macrophage Sentinel
                self.spawn_organism(3, 440.0, 340.0, 5, 110.0); // Swarm Valkyrie
                self.spawn_role_cluster(600.0, 400.0, CellRole::Immunocyte, 18, 170.0);
                self.spawn_role_cluster(620.0, 400.0, CellRole::Photocyte, 22, 190.0);

                // Unleash Lytic, Lysogenic, and Oncogenic strains
                let s0 = VirusStrain::preset(self.alloc_strain_id(), 0);
                let s1 = VirusStrain::preset(self.alloc_strain_id(), 1);
                let s2 = VirusStrain::preset(self.alloc_strain_id(), 2);
                self.spawn_virion_cloud(360.0, 300.0, s0, 38, 65.0);
                self.spawn_virion_cloud(840.0, 480.0, s1, 34, 65.0);
                self.spawn_virion_cloud(600.0, 220.0, s2, 28, 55.0);
            }
            // 2: Hydrothermal Extremophiles & Cryo
            2 => {
                self.fields.reset(24.0, 0.62);
                self.fields.nutrient_floor = 0.50;
                self.fields
                    .add_thermal_source(210.0, 260.0, 175.0, 84.0, 0.28);
                self.fields
                    .add_thermal_source(230.0, 560.0, 160.0, 78.0, 0.25);
                self.fields
                    .add_thermal_source(980.0, 250.0, 170.0, -16.0, 0.28);
                self.fields
                    .add_thermal_source(960.0, 560.0, 160.0, -14.0, 0.25);
                for _ in 0..8 {
                    self.fields.step(1.0);
                }

                // Thermophile multi-mode organisms on left
                let mut thermo_worm = OrganismGenome::preset(4);
                thermo_worm.id = self.alloc_genome_id();
                for m in &mut thermo_worm.modes {
                    m.t_opt = 70.0;
                    m.t_tol = 24.0;
                }
                let tw_id = thermo_worm.id;
                self.genomes.push(thermo_worm);
                self.spawn_organism(tw_id, 250.0, 400.0, 7, 110.0);

                // Temperate center organisms
                self.spawn_organism(2, 600.0, 400.0, 6, 110.0);
                self.spawn_organism(3, 600.0, 380.0, 5, 110.0);

                // Cryophile multi-mode organisms on right
                let mut cryo_reef = OrganismGenome::preset(1);
                cryo_reef.id = self.alloc_genome_id();
                for m in &mut cryo_reef.modes {
                    m.t_opt = -8.0;
                    m.t_tol = 22.0;
                }
                let cr_id = cryo_reef.id;
                self.genomes.push(cryo_reef);
                self.spawn_organism(cr_id, 930.0, 400.0, 7, 110.0);
            }
            // 3: Mutagenic Petri Maze (Walls & Evolution)
            3 => {
                self.fields.reset(25.0, 0.68);
                self.fields.nutrient_floor = 0.55;
                // Construct Petri chamber maze walls with open gates
                self.fields.set_wall_rect(380.0, 60.0, 410.0, 520.0, 1);
                self.fields.set_wall_rect(790.0, 280.0, 820.0, 740.0, 1);
                self.fields.set_wall_rect(560.0, 370.0, 640.0, 430.0, 1);
                self.fields.rebuild_baseline_light();

                // Mutagenic radiation sources in the maze passages
                self.fields.add_mutagen_source(400.0, 640.0, 130.0, 0.95);
                self.fields.add_mutagen_source(800.0, 160.0, 130.0, 0.95);
                self.fields.add_mutagen_source(600.0, 220.0, 110.0, 0.75);
                self.fields.add_light_source(200.0, 260.0, 200.0, 1.30);
                self.fields.add_light_source(1000.0, 540.0, 200.0, 1.30);
                for _ in 0..5 {
                    self.fields.step(1.0);
                }

                self.spawn_organism(2, 200.0, 280.0, 6, 85.0);
                self.spawn_organism(5, 220.0, 580.0, 5, 85.0);
                self.spawn_organism(3, 600.0, 580.0, 5, 85.0);
                self.spawn_organism(4, 980.0, 500.0, 5, 85.0);
                self.spawn_organism(1, 980.0, 240.0, 3, 70.0);
            }
            // 4: Multicellular Locomotion & Combat
            4 => {
                self.fields.reset(26.0, 0.72);
                self.fields.nutrient_floor = 0.56;
                self.fields.add_light_source(600.0, 400.0, 320.0, 1.25);

                self.spawn_organism(5, 320.0, 320.0, 6, 95.0); // Segmented Muscle Worm
                self.spawn_organism(5, 880.0, 480.0, 6, 95.0);
                self.spawn_organism(1, 340.0, 540.0, 4, 80.0); // Apex Leviathan
                self.spawn_organism(3, 840.0, 260.0, 6, 90.0); // Swarm Valkyrie
                self.spawn_organism(4, 600.0, 400.0, 5, 90.0); // Macrophage Sentinel
                self.spawn_organism(2, 600.0, 400.0, 5, 100.0); // Solar Reef Symbiont
            }
            // 5: Blank Petri Sandbox
            _ => {
                self.fields.reset(25.0, 0.72);
                self.fields.nutrient_floor = 0.58;
                self.fields.add_light_source(600.0, 400.0, 340.0, 1.20);
            }
        }

        self.rebuild_spatial_hashes();
        self.update_adhesion_bonds(0.5);
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

    fn alloc_organism_id(&mut self) -> u32 {
        let id = self.next_organism_id;
        self.next_organism_id = self.next_organism_id.wrapping_add(1).max(1);
        id
    }

    fn alloc_lineage_id(&mut self) -> u32 {
        let id = self.next_lineage_id;
        self.next_lineage_id = self.next_lineage_id.wrapping_add(1).max(1);
        id
    }

    fn alloc_genome_id(&mut self) -> u32 {
        let id = self.next_genome_id;
        self.next_genome_id = self.next_genome_id.wrapping_add(1).max(1);
        id
    }

    fn alloc_strain_id(&mut self) -> u32 {
        let id = self.next_strain_id;
        self.next_strain_id = self.next_strain_id.wrapping_add(1).max(1);
        id
    }

    fn alloc_rule_id(&mut self) -> u32 {
        let id = self.next_rule_id;
        self.next_rule_id = self.next_rule_id.wrapping_add(1).max(1);
        id
    }

    pub fn register_genome(&mut self, preset_id: u8, active_modes: u8, initial_mode: u8) -> u32 {
        let mut g = if preset_id < 5 {
            OrganismGenome::preset(preset_id)
        } else {
            let role = CellRole::from_u8(preset_id.saturating_sub(5));
            OrganismGenome::single_role(role, &mut self.rng)
        };
        g.id = self.alloc_genome_id();
        g.active_modes = active_modes.clamp(1, MAX_MODES as u8);
        g.initial_mode = initial_mode.min((MAX_MODES - 1) as u8);
        g.clamp_all();
        let id = g.id;
        if self.genomes.len() >= 32 {
            self.genomes.remove(6);
        }
        self.genomes.push(g);
        id
    }

    pub fn load_preset_genome(&mut self, genome_id: u32, preset_id: u8) -> u32 {
        let mut g = OrganismGenome::preset(preset_id.min(4));
        if let Some(existing) = self.genomes.iter_mut().find(|item| item.id == genome_id) {
            g.id = genome_id;
            *existing = g;
            genome_id
        } else {
            let id = if genome_id == 0 {
                self.alloc_genome_id()
            } else {
                genome_id
            };
            g.id = id;
            self.genomes.push(g);
            id
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_genome_mode(
        &mut self,
        genome_id: u32,
        mode_idx: u8,
        role: u8,
        child_a_mode: u8,
        child_b_mode: u8,
        split_mass: f32,
        split_angle: f32,
        keep_adhesion_parent: bool,
        bond_children: bool,
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
        let pos = match self.genomes.iter().position(|g| g.id == genome_id) {
            Some(p) => p,
            None => {
                let mut g = OrganismGenome::preset(0);
                g.id = if genome_id == 0 {
                    self.alloc_genome_id()
                } else {
                    genome_id
                };
                self.genomes.push(g);
                self.genomes.len() - 1
            }
        };
        let g = &mut self.genomes[pos];
        let m_idx = mode_idx.min((MAX_MODES - 1) as u8);
        if m_idx + 1 > g.active_modes {
            g.active_modes = m_idx + 1;
        }
        if child_a_mode + 1 > g.active_modes && child_a_mode < MAX_MODES as u8 {
            g.active_modes = child_a_mode + 1;
        }
        if child_b_mode + 1 > g.active_modes && child_b_mode < MAX_MODES as u8 {
            g.active_modes = child_b_mode + 1;
        }
        let m = g.mode_mut(m_idx);
        m.role = CellRole::from_u8(role);
        m.child_a_mode = child_a_mode;
        m.child_b_mode = child_b_mode;
        m.split_mass = split_mass;
        m.split_angle = split_angle;
        m.keep_adhesion_parent = keep_adhesion_parent;
        m.bond_children = bond_children;
        m.adhesion_stiffness = adhesion_stiffness;
        m.max_bonds = max_bonds;
        m.t_opt = t_opt;
        m.t_tol = t_tol;
        m.metabolism = metabolism;
        m.membrane_armor = membrane_armor;
        m.immune_resistance = immune_resistance;
        m.vascular_share = vascular_share;
        m.secretion_kind = secretion_kind;
        m.sensory_range = sensory_range;
        m.hebbian_rate = hebbian_rate;
        m.mutation_rate = mutation_rate;
        m.hue_shift = hue_shift;
        m.clamp_fields();
        g.id
    }

    pub fn pack_genome_mode(&mut self, genome_id: u32, mode_idx: u8) -> *const f32 {
        let g = self
            .genomes
            .iter()
            .find(|g| g.id == genome_id)
            .or_else(|| self.genomes.first());
        if let Some(genome) = g {
            let m = genome.mode(mode_idx);
            self.genome_mode_buf = [
                m.role.as_u8() as f32,
                m.child_a_mode as f32,
                m.child_b_mode as f32,
                m.split_mass,
                m.split_angle,
                if m.keep_adhesion_parent { 1.0 } else { 0.0 },
                if m.bond_children { 1.0 } else { 0.0 },
                m.adhesion_stiffness,
                m.max_bonds as f32,
                m.t_opt,
                m.t_tol,
                m.metabolism,
                m.membrane_armor,
                m.immune_resistance,
                m.vascular_share,
                m.secretion_kind as f32,
                m.sensory_range,
                m.hebbian_rate,
                m.mutation_rate,
                m.hue_shift,
            ];
        }
        self.genome_mode_buf.as_ptr()
    }

    pub fn spawn_organism(
        &mut self,
        genome_id: u32,
        cx: f32,
        cy: f32,
        count: u32,
        spread: f32,
    ) -> u32 {
        let fallback = (genome_id.saturating_sub(1) as u8).min(4);
        let template = self
            .genomes
            .iter()
            .find(|g| g.id == genome_id)
            .cloned()
            .unwrap_or_else(|| OrganismGenome::preset(fallback));

        let lineage_id = self.alloc_lineage_id();
        let mut last_id = 0;
        let n = count.clamp(1, 24);
        let eff_spread = spread.max(8.0);

        for _ in 0..n {
            if self.cells.len() >= MAX_CELLS {
                break;
            }
            let ang = self.rng.range_f32(0.0, TAU);
            let dist = self.rng.next_f32().sqrt() * eff_spread;
            let ox = (cx + ang.cos() * dist).clamp(24.0, WORLD_W - 24.0);
            let oy = (cy + ang.sin() * dist).clamp(24.0, WORLD_H - 24.0);
            if self.fields.is_wall_at(ox, oy) {
                continue;
            }

            let org_id = self.alloc_organism_id();
            let m0_idx = template.initial_mode.min((MAX_MODES - 1) as u8);
            let m0 = *template.mode(m0_idx);
            let heading = self.rng.range_f32(0.0, TAU);

            // If multi-mode adhesive creature, spawn an articulated 3-cell differentiated organism
            // (M_init -> child_a_mode + child_b_mode) already linked by cytoskeletal bonds!
            if template.active_modes > 1 && (m0.keep_adhesion_parent || m0.bond_children) {
                let ma_idx = m0.child_a_mode.min(template.active_modes.saturating_sub(1));
                let mb_idx = m0.child_b_mode.min(template.active_modes.saturating_sub(1));
                let ma = *template.mode(ma_idx);
                let mb = *template.mode(mb_idx);

                let root_id = self.push_cell_from_genome(
                    ox,
                    oy,
                    heading,
                    m0_idx,
                    template.clone(),
                    org_id,
                    lineage_id,
                    0,
                    1,
                );
                last_id = root_id;

                if self.cells.len() < MAX_CELLS {
                    let ang_a = wrap_angle(heading + m0.split_angle);
                    let dist_a = m0.role.base_radius() + ma.role.base_radius() + 2.5;
                    let ax = (ox + ang_a.cos() * dist_a).clamp(16.0, WORLD_W - 16.0);
                    let ay = (oy + ang_a.sin() * dist_a).clamp(16.0, WORLD_H - 16.0);
                    let id_a = self.push_cell_from_genome(
                        ax,
                        ay,
                        ang_a,
                        ma_idx,
                        template.clone(),
                        org_id,
                        lineage_id,
                        root_id,
                        2,
                    );
                    last_id = id_a;
                    self.push_bond_pair(
                        root_id,
                        id_a,
                        dist_a,
                        (m0.adhesion_stiffness + ma.adhesion_stiffness) * 0.5,
                        m0.split_angle,
                    );

                    if self.cells.len() < MAX_CELLS {
                        let ang_b = wrap_angle(heading - m0.split_angle + PI * 0.65);
                        let dist_b = m0.role.base_radius() + mb.role.base_radius() + 2.5;
                        let bx = (ox + ang_b.cos() * dist_b).clamp(16.0, WORLD_W - 16.0);
                        let by = (oy + ang_b.sin() * dist_b).clamp(16.0, WORLD_H - 16.0);
                        let id_b = self.push_cell_from_genome(
                            bx,
                            by,
                            ang_b,
                            mb_idx,
                            template.clone(),
                            org_id,
                            lineage_id,
                            root_id,
                            2,
                        );
                        last_id = id_b;
                        self.push_bond_pair(
                            root_id,
                            id_b,
                            dist_b,
                            (m0.adhesion_stiffness + mb.adhesion_stiffness) * 0.5,
                            -m0.split_angle,
                        );
                        if m0.bond_children {
                            let dx = bx - ax;
                            let dy = by - ay;
                            let dab = (dx * dx + dy * dy).sqrt().max(12.0);
                            self.push_bond_pair(
                                id_a,
                                id_b,
                                dab,
                                (ma.adhesion_stiffness + mb.adhesion_stiffness) * 0.5,
                                0.0,
                            );
                        }
                    }
                }
            } else {
                last_id = self.push_cell_from_genome(
                    ox,
                    oy,
                    heading,
                    m0_idx,
                    template.clone(),
                    org_id,
                    lineage_id,
                    0,
                    1,
                );
            }
        }

        if last_id != 0 {
            self.selected_cell_id = last_id;
        }
        self.rebuild_spatial_hashes();
        self.rebuild_render_buffers();
        last_id
    }

    pub fn spawn_role_cluster(
        &mut self,
        cx: f32,
        cy: f32,
        role: CellRole,
        count: u32,
        spread: f32,
    ) -> u32 {
        let lineage_id = self.alloc_lineage_id();
        let mut last_id = 0;
        for _ in 0..count {
            if self.cells.len() >= MAX_CELLS {
                break;
            }
            let ang = self.rng.range_f32(0.0, TAU);
            let dist = self.rng.next_f32().sqrt() * spread;
            let x = (cx + ang.cos() * dist).clamp(20.0, WORLD_W - 20.0);
            let y = (cy + ang.sin() * dist).clamp(20.0, WORLD_H - 20.0);
            if self.fields.is_wall_at(x, y) {
                continue;
            }
            let mut genome = OrganismGenome::single_role(role, &mut self.rng);
            genome.id = self.alloc_genome_id();
            let org_id = self.alloc_organism_id();
            let heading = self.rng.range_f32(0.0, TAU);
            last_id =
                self.push_cell_from_genome(x, y, heading, 0, genome, org_id, lineage_id, 0, 1);
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
        let mut genome = OrganismGenome::single_role(role, &mut self.rng);
        genome.id = self.alloc_genome_id();
        {
            let m = genome.mode_mut(0);
            m.role = role;
            m.membrane_armor = armor.clamp(0.02, 0.95);
            m.metabolism = metabolism.clamp(0.40, 2.40);
            m.t_opt = t_opt.clamp(-25.0, 95.0);
            m.t_tol = t_tol.clamp(5.0, 40.0);
            m.adhesion_stiffness = adhesion.clamp(0.0, 1.0);
            m.keep_adhesion_parent = adhesion > 0.45;
            m.bond_children = adhesion > 0.45;
            m.mutation_rate = mutation_rate.clamp(0.005, 0.45);
            m.immune_resistance = immune_resistance.clamp(0.02, 0.98);
            m.hebbian_rate = (0.08 + neural_bias.abs() * 0.12).clamp(0.0, 0.50);
            m.clamp_fields();
        }
        let gid = genome.id;
        self.genomes.push(genome);
        self.spawn_organism(gid, x, y, count.clamp(1, 24), spread.max(8.0))
    }

    #[allow(clippy::too_many_arguments)]
    fn push_cell_from_genome(
        &mut self,
        x: f32,
        y: f32,
        angle: f32,
        mode_idx: u8,
        genome: OrganismGenome,
        organism_id: u32,
        lineage_id: u32,
        parent_id: u32,
        generation: u32,
    ) -> u32 {
        let id = self.alloc_cell_id();
        let m_idx = mode_idx.min((MAX_MODES - 1) as u8);
        let mode = *genome.mode(m_idx);
        let role = mode.role;
        let max_hp = 85.0 + mode.membrane_armor * 80.0;
        let max_atp = role.base_max_atp();
        let mut brain = NeuralNet::for_role(role, &mut self.rng);
        let dummy_in = [
            0.65, 0.70, 0.0, 0.0, 0.15, 0.0, 0.25, 0.78, 0.0, 0.0, 0.12, 0.15,
        ];
        brain.forward(dummy_in, mode.hue_shift);
        brain.classify_cognitive_state(role, 0.78, 0.0, 0);

        let cell = Cell {
            id,
            organism_id,
            lineage_id,
            parent_id,
            generation,
            x,
            y,
            vx: self.rng.range_f32(-0.4, 0.4),
            vy: self.rng.range_f32(-0.4, 0.4),
            angle,
            radius: role.base_radius(),
            hp: max_hp,
            max_hp,
            atp: max_atp * 0.76,
            max_atp,
            age: 0,
            divide_cooldown: 24,
            mode_idx: m_idx,
            genome,
            brain,
            synaptic_pulse: 0.0,
            incoming_synaptic: 0.0,
            myocyte_flex: 0.0,
            crispr_mask: 0,
            viral_load: 0.0,
            incubation_progress: 0.0,
            infected_by: None,
            lysogenic_dormant: false,
            temp_stress: 0.0,
            q10_rate: 1.0,
            signal_flags: 0,
            bonded_count: 0,
            keratin_shield: 0.0,
            alive: true,
        };
        self.cells.push(cell);
        self.total_births = self.total_births.saturating_add(1);
        id
    }

    fn push_bond_pair(
        &mut self,
        a_id: u32,
        b_id: u32,
        rest_len: f32,
        stiffness: f32,
        preferred_angle: f32,
    ) {
        if a_id == b_id || self.bonds.len() >= MAX_BONDS {
            return;
        }
        let exists = self
            .bonds
            .iter()
            .any(|b| (b.a_id == a_id && b.b_id == b_id) || (b.a_id == b_id && b.b_id == a_id));
        if !exists {
            self.bonds.push(AdhesionBond {
                a_id,
                b_id,
                rest_len: rest_len.clamp(10.0, 34.0),
                stiffness: stiffness.clamp(0.15, 1.0),
                preferred_angle,
                last_vascular_flow: 0.0,
                last_synaptic_flow: 0.0,
            });
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn upsert_virus_strain(
        &mut self,
        strain_id: u32,
        strategy: u8,
        target_role_mask: u16,
        infectivity: f32,
        incubation_ticks: f32,
        burst_count: u8,
        lethality: f32,
        mutation_rate: f32,
        immune_evasion: f32,
        thermal_resistance: f32,
        enveloped: bool,
        hue: f32,
    ) -> u32 {
        let id = if strain_id == 0 {
            self.alloc_strain_id()
        } else {
            strain_id
        };
        let mut strain = VirusStrain {
            id,
            generation: 1,
            strategy: ViralStrategy::from_u8(strategy),
            target_role_mask,
            infectivity,
            incubation_ticks,
            burst_count,
            lethality,
            mutation_rate,
            immune_evasion,
            thermal_resistance,
            enveloped,
            hue,
            antigenic_sig: (id as u8) & 15,
        };
        strain.clamp_fields();

        if let Some(existing) = self.strains.iter_mut().find(|s| s.id == id) {
            *existing = strain;
        } else {
            if self.strains.len() >= 16 {
                self.strains.remove(0);
            }
            self.strains.push(strain);
        }
        id
    }

    pub fn inject_strain(
        &mut self,
        strain_id: u32,
        x: f32,
        y: f32,
        count: u32,
        spread: f32,
    ) -> u32 {
        let strain = self
            .strains
            .iter()
            .find(|s| s.id == strain_id)
            .cloned()
            .unwrap_or_else(|| {
                let idx = (strain_id.saturating_sub(1) as u8).min(2);
                VirusStrain::preset(strain_id.max(1), idx)
            });

        let c_lim = count.clamp(1, 90);
        let s_lim = spread.max(8.0);
        let spawned = self.spawn_virion_cloud(x, y, strain.clone(), c_lim, s_lim);

        // Directly expose up to 2 nearby susceptible cells within `spread` for immediate infection.
        let r2 = (spread.max(28.0) + 18.0).powi(2);
        let mut directly_infected = 0;
        for c in &mut self.cells {
            if !c.alive || c.infected_by.is_some() {
                continue;
            }
            let dx = c.x - x;
            let dy = c.y - y;
            if dx * dx + dy * dy <= r2 && strain.can_infect_role(c.role()) {
                let has_crispr = (c.crispr_mask & strain.crispr_bit()) != 0;
                if !has_crispr {
                    c.infected_by = Some(strain.clone());
                    c.lysogenic_dormant = strain.strategy == ViralStrategy::Lysogenic;
                    c.viral_load = if c.lysogenic_dormant { 0.12 } else { 0.22 };
                    c.incubation_progress = 0.08;
                    directly_infected += 1;
                    if directly_infected >= 2 {
                        break;
                    }
                }
            }
        }
        self.rebuild_render_buffers();
        spawned
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
            let ang = self.rng.range_f32(0.0, TAU);
            let dist = self.rng.next_f32().sqrt() * spread;
            let x = (cx + ang.cos() * dist).clamp(8.0, WORLD_W - 8.0);
            let y = (cy + ang.sin() * dist).clamp(8.0, WORLD_H - 8.0);
            if self.fields.is_wall_at(x, y) {
                continue;
            }
            let vang = self.rng.range_f32(0.0, TAU);
            self.virions.push(Virion {
                x,
                y,
                vx: vang.cos() * 1.45,
                vy: vang.sin() * 1.45,
                age: 0.0,
                max_age: self.rng.range_f32(440.0, 780.0),
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
        let sid = self.upsert_virus_strain(
            0,
            0,
            crate::virus::ALL_ROLES_MASK,
            infectivity,
            incubation_ticks,
            burst_count,
            lethality,
            mutation_rate,
            immune_evasion,
            0.50,
            false,
            0.05,
        );
        self.inject_strain(sid, x, y, count, spread)
    }

    pub fn upsert_mutation_rule(
        &mut self,
        rule_id: u32,
        enabled: bool,
        target_mask: u32,
        bias: f32,
        strength: f32,
        probability: f32,
        trigger: u8,
    ) -> u32 {
        let id = if rule_id == 0 {
            self.alloc_rule_id()
        } else {
            rule_id
        };
        let mut rule = MutationRule {
            id,
            enabled,
            target_mask,
            bias,
            strength,
            probability,
            trigger,
        };
        rule.clamp_fields();

        if let Some(existing) = self.mutation_rules.iter_mut().find(|r| r.id == id) {
            *existing = rule;
        } else {
            if self.mutation_rules.len() >= 16 {
                self.mutation_rules.remove(0);
            }
            self.mutation_rules.push(rule);
        }
        id
    }

    pub fn clear_mutation_rules(&mut self) {
        self.mutation_rules.clear();
    }

    pub fn trigger_mutation(&mut self, cell_id: u32, rule_id: u32) -> u32 {
        let cid = if cell_id != 0 {
            cell_id
        } else if self.selected_cell_id != 0 {
            self.selected_cell_id
        } else {
            self.cells.first().map_or(0, |c| c.id)
        };

        let Some(c_idx) = self.cells.iter().position(|c| c.alive && c.id == cid) else {
            return 0;
        };

        let rule = self
            .mutation_rules
            .iter()
            .find(|r| r.id == rule_id)
            .copied()
            .or_else(|| self.mutation_rules.first().copied())
            .unwrap_or_else(|| MutationRule::preset(rule_id.max(1), 3));

        let mut genome = self.cells[c_idx].genome.clone();
        let mut brain = self.cells[c_idx].brain.clone();
        let mode_idx = self.cells[c_idx].mode_idx;
        let org_id = self.cells[c_idx].organism_id;

        let outcome = apply_single_mutation_rule(
            &mut genome,
            &mut brain,
            mode_idx,
            &rule,
            1.0,
            &mut self.rng,
        );
        let new_role = genome.mode(mode_idx).role;
        self.cells[c_idx].genome = genome;
        self.cells[c_idx].brain = brain;
        self.cells[c_idx].radius = new_role.base_radius();
        self.cells[c_idx].signal_flags |= SIGNAL_MUTATED;

        let count = outcome.events.len() as u32;
        self.record_mutation_outcome(org_id, new_role, &outcome);
        self.rebuild_render_buffers();
        count
    }

    fn record_mutation_outcome(
        &mut self,
        organism_id: u32,
        role: CellRole,
        outcome: &MutationOutcome,
    ) {
        if !outcome.mutated {
            return;
        }
        for ev in &outcome.events {
            self.total_mutations = self.total_mutations.saturating_add(1);
            if self.mutation_log.len() >= MAX_MUTATION_LOG {
                self.mutation_log.remove(0);
            }
            self.mutation_log.push(MutationLogEntry {
                tick: self.tick,
                organism_id,
                role: role.as_u8(),
                target_kind: ev.target_kind,
                delta: ev.delta,
                mode_idx: ev.mode_idx,
            });
        }
    }

    pub fn apply_brush(&mut self, brush_kind: u8, x: f32, y: f32, radius: f32, intensity: f32) {
        self.fields.apply_brush(brush_kind, x, y, radius, intensity);
        let r2 = radius.max(12.0).powi(2);

        // Antibiotic brush (3) directly neutralizes nearby free virions.
        if brush_kind == 3 {
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

        // Mutagen Ray brush (5) directly applies mutagen-triggered mutation rules to exposed cells!
        if brush_kind == 5 && !self.mutation_rules.is_empty() {
            let rules_snapshot = self.mutation_rules.clone();
            let mut logs: Vec<(u32, CellRole, MutationOutcome)> = Vec::new();
            for c in &mut self.cells {
                if !c.alive {
                    continue;
                }
                let dx = c.x - x;
                let dy = c.y - y;
                if dx * dx + dy * dy <= r2 {
                    let outcome = apply_mutation_rules(
                        &mut c.genome,
                        &mut c.brain,
                        c.mode_idx,
                        &rules_snapshot,
                        MUT_TRIGGER_ON_MUTAGEN_FIELD,
                        intensity.max(0.8),
                        &mut self.rng,
                    );
                    if outcome.mutated {
                        let new_role = c.role();
                        c.radius = new_role.base_radius();
                        c.signal_flags |= SIGNAL_MUTATED;
                        logs.push((c.organism_id, new_role, outcome));
                    }
                }
            }
            for (org_id, role, out) in logs {
                self.record_mutation_outcome(org_id, role, &out);
            }
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

    /// Advance the full biological, neural, viral, and multicellular physics simulation by `dt`.
    pub fn step(&mut self, dt: f32) {
        let dt = dt.clamp(0.1, 2.0);
        self.tick = self.tick.saturating_add(1);

        // 1. Environmental diffusion, vents, solar light, nutrients, pheromones, toxins, mutagen, walls.
        self.fields.step(dt);

        // 2. Rebuild spatial hash grids for O(N) neighbor queries.
        self.rebuild_spatial_hashes();

        // 3. Sensory inputs + Recurrent Neural Brain + Hebbian learning + Q10 physiology + viral cycles.
        let n_cells = self.cells.len();
        let mut pending_mitoses: Vec<PendingMitosis> = Vec::new();
        let mut new_virions: Vec<Virion> = Vec::new();

        for idx in 0..n_cells {
            if !self.cells[idx].alive {
                continue;
            }
            self.step_cell_brain_and_physiology(idx, dt, &mut pending_mitoses, &mut new_virions);
        }

        // 4. Execute Cell Lab multi-mode mitotic cleavage & daughter adhesion bonding.
        self.execute_mitoses(pending_mitoses);

        // 5. Cell-cell interactions: Phagocyte predation, Immunocyte antiviral/CRISPR & healing, Scavenger recycling.
        self.rebuild_spatial_hashes();
        self.step_cell_interactions(dt);

        // 6. Virion chemotactic drift, wall collision, antibody neutralization, and host tropism/CRISPR attachment.
        self.step_virions(dt);

        // 7. Append newly burst virions respecting MAX_VIRIONS cap.
        for v in new_virions {
            if self.virions.len() < MAX_VIRIONS {
                self.virions.push(v);
            }
        }

        // 8. Prune dead cells (recycling biomass into nutrients) & dead virions.
        for i in 0..self.cells.len() {
            if !self.cells[i].alive {
                let cx = self.cells[i].x;
                let cy = self.cells[i].y;
                self.fields.add_nutrient(cx, cy, 0.14);
            }
        }
        self.cells.retain(|c| c.alive);
        self.virions.retain(|v| v.alive);

        // 9. Multicellular adhesion bonds: spring physics + Myocyte muscle flexing + Vascular ATP sharing + Synaptic propagation!
        self.rebuild_spatial_hashes();
        self.update_adhesion_bonds(dt);

        // 10. Keep selected cell valid and pack flat WASM FFI buffers.
        self.rebuild_render_buffers();
    }

    fn step_cell_brain_and_physiology(
        &mut self,
        idx: usize,
        dt: f32,
        pending_mitoses: &mut Vec<PendingMitosis>,
        new_virions: &mut Vec<Virion>,
    ) {
        let (
            cx,
            cy,
            c_id,
            org_id,
            c_lineage,
            mode_idx,
            mode,
            atp,
            max_atp,
            hp_before,
            viral_load,
            in_synaptic,
            bonded_count,
            keratin_shield,
        ) = {
            let c = &self.cells[idx];
            (
                c.x,
                c.y,
                c.id,
                c.organism_id,
                c.lineage_id,
                c.mode_idx,
                *c.mode(),
                c.atp,
                c.max_atp,
                c.hp,
                c.viral_load,
                c.incoming_synaptic,
                c.bonded_count,
                c.keratin_shield,
            )
        };
        let role = mode.role;

        let local_temp = self.fields.sample_temp(cx, cy);
        let local_light = self.fields.sample_light(cx, cy);
        let local_nutrient = self.fields.sample_nutrient(cx, cy);
        let local_alarm = self.fields.sample_alarm(cx, cy);
        let local_food = self.fields.sample_food(cx, cy);
        let local_toxin = self.fields.sample_toxin(cx, cy);
        let local_mutagen = self.fields.sample_mutagen(cx, cy);

        // Scan nearby cells & virions within `mode.sensory_range` via spatial hash.
        let sense_r = mode.sensory_range.clamp(45.0, 220.0);
        let mut nearest_enemy_d2 = sense_r * sense_r;
        let mut enemy_dir: Option<(f32, f32)> = None;
        let mut nearest_ally_d2 = sense_r * sense_r;
        let mut ally_dir: Option<(f32, f32)> = None;
        let mut crowd_count = 0.0_f32;

        let gx = ((cx / HASH_CELL) as isize).clamp(0, HASH_W as isize - 1) as usize;
        let gy = ((cy / HASH_CELL) as isize).clamp(0, HASH_H as isize - 1) as usize;
        let rad_cells = ((sense_r / HASH_CELL).ceil() as usize).clamp(1, 3);

        for ny in gy.saturating_sub(rad_cells)..=(gy + rad_cells).min(HASH_H - 1) {
            for nx in gx.saturating_sub(rad_cells)..=(gx + rad_cells).min(HASH_W - 1) {
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
                    let o_role = o.role();
                    let o_infected = o.is_actively_infected();
                    let is_target_or_threat = match role {
                        CellRole::Immunocyte => o_infected || o_role == CellRole::Phagocyte,
                        CellRole::Phagocyte => o.lineage_id != c_lineage && o.organism_id != org_id,
                        _ => o_role == CellRole::Phagocyte || o_infected,
                    };
                    if is_target_or_threat && d2 < nearest_enemy_d2 {
                        nearest_enemy_d2 = d2;
                        enemy_dir = Some((dx, dy));
                    } else if !is_target_or_threat
                        && (o.organism_id == org_id || o.lineage_id == c_lineage)
                        && d2 < nearest_ally_d2
                    {
                        nearest_ally_d2 = d2;
                        ally_dir = Some((dx, dy));
                    }
                }
                if role == CellRole::Immunocyte || role == CellRole::Senseocyte {
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

        let enemy_prox = (1.0 - nearest_enemy_d2.sqrt() / sense_r).clamp(0.0, 1.0);
        let ally_prox = (1.0 - nearest_ally_d2.sqrt() / sense_r).clamp(0.0, 1.0);
        let crowding = (crowd_count / 7.0).clamp(0.0, 1.0);
        let q10 = mode.q10_rate(local_temp);
        let stress = mode.thermal_stress(local_temp);
        let atp_ratio = (atp / max_atp.max(1.0)).clamp(0.0, 1.0);

        let inputs: [f32; NN_INPUTS] = [
            local_nutrient.clamp(0.0, 1.0),
            local_light.clamp(0.0, 1.0),
            stress.clamp(-1.5, 1.5),
            local_alarm.clamp(0.0, 1.0),
            local_food.clamp(0.0, 1.0),
            enemy_prox,
            ally_prox,
            atp_ratio,
            viral_load.clamp(0.0, 1.0),
            local_mutagen.clamp(0.0, 1.0),
            in_synaptic.clamp(-1.0, 1.0),
            crowding,
        ];

        let outputs = self.cells[idx].brain.forward(inputs, mode.hue_shift);
        let brain = &mut self.cells[idx].brain;
        brain.classify_cognitive_state(role, atp_ratio, viral_load, bonded_count);

        // Compute chemotaxis & phototaxis steering bias combined with neural turn output.
        let mut angle_bias = 0.0_f32;
        let current_angle = self.cells[idx].angle;
        if let Some((ex, ey)) = enemy_dir {
            let target_ang = ey.atan2(ex);
            let diff = wrap_angle(target_ang - current_angle);
            match role {
                CellRole::Immunocyte | CellRole::Phagocyte => {
                    angle_bias += diff * 0.36;
                }
                _ => {
                    angle_bias -= diff * 0.26;
                }
            }
        } else if role == CellRole::Photocyte {
            let (lgx, lgy) = self.fields.sample_gradient(4, cx, cy);
            let (ngx, ngy) = self.fields.sample_gradient(0, cx, cy);
            let gx_sum = lgx * 1.1 + ngx * 0.6;
            let gy_sum = lgy * 1.1 + ngy * 0.6;
            if gx_sum * gx_sum + gy_sum * gy_sum > 0.0001 {
                angle_bias += wrap_angle(gy_sum.atan2(gx_sum) - current_angle) * 0.25;
            }
        } else {
            let (ngx, ngy) = self.fields.sample_gradient(0, cx, cy);
            let (fgx, fgy) = self.fields.sample_gradient(2, cx, cy);
            let gx_sum = ngx + fgx * 0.85;
            let gy_sum = ngy + fgy * 0.85;
            if gx_sum * gx_sum + gy_sum * gy_sum > 0.0001 {
                angle_bias += wrap_angle(gy_sum.atan2(gx_sum) - current_angle) * 0.28;
            } else if let Some((ax, ay)) = ally_dir {
                if mode.adhesion_stiffness > 0.45 {
                    angle_bias += wrap_angle(ay.atan2(ax) - current_angle) * 0.14;
                }
            }
        }

        // Locomotion thrust & turn (Flagellocytes & Myocytes respond strongly to incoming synaptic signals!).
        let syn_boost = if role == CellRole::Flagellocyte || role == CellRole::Myocyte {
            1.0 + in_synaptic.abs() * 0.45
        } else {
            1.0
        };
        let infected_scale = if self.cells[idx].is_actively_infected() {
            0.60
        } else {
            1.0
        };
        let thrust = ((outputs[0] + 0.68) * 0.62).clamp(0.0, 1.45)
            * role.base_speed()
            * q10.sqrt()
            * syn_boost
            * infected_scale;
        let turn = outputs[1] * 0.25 + angle_bias + self.rng.normal_approx() * 0.025;
        let next_angle = wrap_angle(current_angle + turn * dt);
        let fx = next_angle.cos() * thrust;
        let fy = next_angle.sin() * thrust;

        // Nutrient harvest, Photocyte photosynthesis, and Scavenger detoxification.
        let consume_rate = match role {
            CellRole::Photocyte => 0.018,
            CellRole::Scavenger => 0.024,
            CellRole::Vascularocyte | CellRole::Stemocyte => 0.020,
            CellRole::Phagocyte => 0.010,
            _ => 0.016,
        };
        let taken_nutrient = self.fields.consume_nutrient(cx, cy, consume_rate * dt);
        let photosynth = if !self.cells[idx].is_actively_infected() {
            let eff = if role == CellRole::Photocyte {
                mode.light_eff.max(1.1)
            } else {
                mode.light_eff * 0.35
            };
            eff * local_light * 0.32 * q10 * (1.0 - crowding * 0.38) * dt
        } else {
            0.0
        };
        let scav_bonus = if role == CellRole::Scavenger {
            let cleared_tox = (local_toxin * 0.12).min(0.08);
            if cleared_tox > 0.005 {
                self.fields.add_nutrient(cx, cy, cleared_tox * 0.6);
            }
            cleared_tox * 14.0 * dt
        } else {
            0.0
        };

        let atp_gain = taken_nutrient * 25.0 + photosynth + scav_bonus;
        let eff_armor = (mode.membrane_armor + keratin_shield).clamp(0.02, 0.96);
        let basal_cost =
            (0.13 + thrust * 0.055 + mode.membrane_armor * 0.04) * mode.metabolism * q10 * dt;

        // Chemical secretions & synaptic pulse emission.
        let mut signals = 0_u8;
        let mut secretion_cost = 0.0_f32;
        if (outputs[3] > 0.22 || mode.secretion_kind == SECRETION_ALARM) && atp > 18.0 {
            self.fields.add_alarm(cx, cy, 0.14 * dt);
            signals |= SIGNAL_ALARM;
            secretion_cost += 0.035 * dt;
        }
        if (outputs[4] > 0.22 || mode.secretion_kind == SECRETION_FOOD_PHEROMONE)
            && atp > 18.0
            && local_nutrient > 0.22
        {
            self.fields.add_food_pheromone(cx, cy, 0.12 * dt);
            signals |= SIGNAL_FOOD;
            secretion_cost += 0.03 * dt;
        }
        if (role == CellRole::Immunocyte
            || mode.secretion_kind == SECRETION_ANTIBODY
            || mode.secretion_kind == SECRETION_TOXIN)
            && (outputs[5] > 0.10 || role == CellRole::Secreocyte)
            && atp > 22.0
        {
            self.fields.add_toxin(cx, cy, 0.16 * dt);
            signals |= SIGNAL_ANTIBODY;
            secretion_cost += 0.05 * dt;
        }
        if (role == CellRole::Scavenger || mode.secretion_kind == SECRETION_NUTRIENT_MUCUS)
            && atp > 35.0
        {
            self.fields.add_nutrient(cx, cy, 0.045 * dt);
            secretion_cost += 0.03 * dt;
        }

        let syn_out = if role == CellRole::Neurocyte || role == CellRole::Senseocyte {
            (outputs[6] * 1.25).clamp(-1.0, 1.0)
        } else {
            (outputs[6] * 0.75 + in_synaptic * 0.35).clamp(-1.0, 1.0)
        };
        if syn_out.abs() > 0.18 {
            signals |= SIGNAL_SYNAPSE;
        }
        let flex_out = outputs[7].clamp(-1.0, 1.0);

        // Apply physics, wall collisions, thermal & viral progression, and mutation triggers.
        let mut should_lyse_strain: Option<VirusStrain> = None;
        let mut oncogenic_mut_trigger = false;
        let mut should_divide = false;

        {
            let c = &mut self.cells[idx];
            c.age = c.age.saturating_add(1);
            c.divide_cooldown = c.divide_cooldown.saturating_sub(1);
            c.angle = next_angle;
            c.synaptic_pulse = syn_out;
            c.myocyte_flex = flex_out;
            c.incoming_synaptic *= 0.35;

            let next_vx = (c.vx + fx * 0.28 * dt) * 0.82;
            let next_vy = (c.vy + fy * 0.28 * dt) * 0.82;
            let rx = (c.x + next_vx * dt).clamp(c.radius, WORLD_W - c.radius);
            let ry = (c.y + next_vy * dt).clamp(c.radius, WORLD_H - c.radius);
            let cr = c.radius;
            let (res_x, res_y, mut res_vx, mut res_vy) = self
                .fields
                .resolve_wall_collision(rx, ry, next_vx, next_vy, cr);
            if res_x <= c.radius + 0.5 || res_x >= WORLD_W - c.radius - 0.5 {
                res_vx *= -0.7;
            }
            if res_y <= c.radius + 0.5 || res_y >= WORLD_H - c.radius - 0.5 {
                res_vy *= -0.7;
            }
            c.x = res_x.clamp(c.radius, WORLD_W - c.radius);
            c.y = res_y.clamp(c.radius, WORLD_H - c.radius);
            c.vx = res_vx;
            c.vy = res_vy;

            c.temp_stress = stress;
            c.q10_rate = q10;
            c.signal_flags = signals;
            c.atp = (c.atp + atp_gain - basal_cost - secretion_cost).clamp(0.0, c.max_atp);

            // Thermal damage outside tolerance [T_opt - T_tol, T_opt + T_tol].
            if stress.abs() > 0.02 {
                let thermal_dmg = stress.abs() * 0.55 * (1.0 - eff_armor * 0.35) * dt;
                c.hp -= thermal_dmg;
            }

            if c.atp <= 0.01 {
                c.hp -= 0.42 * dt;
            } else if c.hp < c.max_hp && c.atp > c.max_atp * 0.32 && stress.abs() < 0.12 {
                c.hp = (c.hp + 0.16 * dt).min(c.max_hp);
            }
            if local_toxin > 0.38 && role == CellRole::Phagocyte {
                c.hp -= (local_toxin - 0.38) * 0.32 * (1.0 - eff_armor * 0.5) * dt;
            }

            // Viral progression for Lytic, Lysogenic, and OncogenicMutator strategies!
            let mut force_oncogenic_divide = false;
            if let Some(strain) = c.infected_by.clone() {
                let has_crispr = (c.crispr_mask & strain.crispr_bit()) != 0;
                let crispr_bonus = if has_crispr { 0.65 } else { 0.0 };
                let evade = 1.0 - strain.immune_evasion;
                let host_res = (mode.immune_resistance + crispr_bonus) * evade;
                let suppression = local_toxin * 0.55 + host_res * 0.48;

                // Check if intracellular immunity + CRISPR clears the infection
                if suppression > 0.62 {
                    c.viral_load = (c.viral_load - 0.025 * dt).max(0.0);
                    if c.viral_load <= 0.01 {
                        c.crispr_mask |= strain.crispr_bit();
                        c.infected_by = None;
                        c.lysogenic_dormant = false;
                        c.incubation_progress = 0.0;
                    }
                } else {
                    match strain.strategy {
                        ViralStrategy::Lytic => {
                            c.lysogenic_dormant = false;
                            c.atp = (c.atp - 0.16 * dt).max(0.0);
                            let step_p = (q10 / strain.incubation_ticks.max(20.0)) * dt;
                            c.incubation_progress += step_p;
                            c.viral_load = (c.viral_load + step_p * 0.95).clamp(0.0, 1.0);
                            c.hp -= strain.lethality * 0.24 * dt;
                            let dead_lyse = c.hp <= 0.0 && c.viral_load > 0.30;
                            if c.incubation_progress >= 1.0 || dead_lyse {
                                should_lyse_strain = Some(strain);
                                c.alive = false;
                            }
                        }
                        ViralStrategy::Lysogenic => {
                            // Remain dormant provirus inherited across mitosis until stress/mutagen/age triggers lysis!
                            if c.lysogenic_dormant {
                                c.viral_load = 0.12;
                                if stress.abs() > 0.22
                                    || local_mutagen > 0.16
                                    || c.atp < c.max_atp * 0.24
                                    || c.age > 180
                                {
                                    c.lysogenic_dormant = false;
                                }
                            } else {
                                c.atp = (c.atp - 0.14 * dt).max(0.0);
                                let step_p = (q10 / strain.incubation_ticks.max(25.0)) * dt;
                                c.incubation_progress += step_p;
                                c.viral_load = (c.viral_load + step_p * 0.90).clamp(0.12, 1.0);
                                c.hp -= strain.lethality * 0.20 * dt;
                                if c.incubation_progress >= 1.0
                                    || (c.hp <= 0.0 && c.viral_load > 0.30)
                                {
                                    should_lyse_strain = Some(strain);
                                    c.alive = false;
                                }
                            }
                        }
                        ViralStrategy::OncogenicMutator => {
                            c.lysogenic_dormant = false;
                            let step_p = (q10 / strain.incubation_ticks.max(30.0)) * dt;
                            c.incubation_progress += step_p;
                            c.viral_load = (c.viral_load + step_p * 0.65).clamp(0.18, 0.95);
                            // Hijack neural mitosis bias & trigger periodic oncogenic genome/brain mutation!
                            c.brain.b2[2] = (c.brain.b2[2] + 0.15 * dt).clamp(-1.0, 2.2);
                            force_oncogenic_divide = c.atp >= c.max_atp * 0.45;
                            if c.age % 20 == 0 {
                                oncogenic_mut_trigger = true;
                            }
                            if c.incubation_progress >= 1.35 {
                                should_lyse_strain = Some(strain);
                                c.alive = false;
                            }
                        }
                    }
                }
            }

            if c.hp <= 0.0 || c.age > 9500 {
                c.alive = false;
            }

            // Online reward-modulated Hebbian synaptic plasticity!
            let atp_delta = (c.atp - atp) / c.max_atp.max(1.0);
            let hp_delta = (c.hp - hp_before) / c.max_hp.max(1.0);
            let penalty = stress.abs() * 0.20 + c.viral_load * 0.15;
            let reward = atp_delta * 3.5 + hp_delta * 2.0 - penalty + 0.04;
            c.brain.adapt(reward, mode.hebbian_rate);

            // Mitosis check (healthy or lysogenic-dormant or oncogenic-forced).
            let can_divide_infection =
                c.infected_by.is_none() || c.lysogenic_dormant || force_oncogenic_divide;
            let split_thresh = if force_oncogenic_divide {
                c.max_atp * 0.46
            } else {
                mode.split_atp_threshold(c.max_atp)
            };
            if c.alive
                && can_divide_infection
                && c.divide_cooldown == 0
                && c.atp >= split_thresh
                && (outputs[2] > 0.02 || force_oncogenic_divide || role == CellRole::Stemocyte)
            {
                should_divide = true;
                c.divide_cooldown = match role {
                    CellRole::Stemocyte => 42,
                    _ => 78,
                };
                c.signal_flags |= SIGNAL_DIVIDE;
            }
        }

        // Execute environmental or oncogenic mutation rules on this cell if triggered.
        if self.cells[idx].alive && !self.mutation_rules.is_empty() {
            let mut trigger_to_run: Option<(u8, f32)> = None;
            if oncogenic_mut_trigger {
                trigger_to_run = Some((MUT_TRIGGER_DIRECT_ONLY, 1.0));
            } else if local_mutagen > 0.16 && (self.tick + c_id) % 18 == 0 {
                trigger_to_run = Some((MUT_TRIGGER_ON_MUTAGEN_FIELD, local_mutagen));
            } else if stress.abs() > 0.30 && (self.tick + c_id) % 24 == 0 {
                trigger_to_run = Some((MUT_TRIGGER_ON_THERMAL_STRESS, stress.abs()));
            }

            if let Some((trig, env_int)) = trigger_to_run {
                let rules_snapshot = self.mutation_rules.clone();
                let mut genome = self.cells[idx].genome.clone();
                let mut brain = self.cells[idx].brain.clone();
                let outcome = apply_mutation_rules(
                    &mut genome,
                    &mut brain,
                    mode_idx,
                    &rules_snapshot,
                    trig,
                    env_int,
                    &mut self.rng,
                );
                if outcome.mutated {
                    let new_role = genome.mode(mode_idx).role;
                    self.cells[idx].genome = genome;
                    self.cells[idx].brain = brain;
                    self.cells[idx].radius = new_role.base_radius();
                    self.cells[idx].signal_flags |= SIGNAL_MUTATED;
                    self.record_mutation_outcome(org_id, new_role, &outcome);
                }
            }
        } else if self.cells[idx].alive && oncogenic_mut_trigger {
            let rule = MutationRule::preset(99, 1);
            let mut genome = self.cells[idx].genome.clone();
            let mut brain = self.cells[idx].brain.clone();
            let outcome = apply_single_mutation_rule(
                &mut genome,
                &mut brain,
                mode_idx,
                &rule,
                0.85,
                &mut self.rng,
            );
            if outcome.mutated {
                let new_role = genome.mode(mode_idx).role;
                self.cells[idx].genome = genome;
                self.cells[idx].brain = brain;
                self.cells[idx].radius = new_role.base_radius();
                self.cells[idx].signal_flags |= SIGNAL_MUTATED;
                self.record_mutation_outcome(org_id, new_role, &outcome);
            }
        }

        if let Some(strain) = should_lyse_strain {
            self.total_bursts = self.total_bursts.saturating_add(1);
            self.fields.add_alarm(cx, cy, 0.55);
            for _ in 0..strain.burst_count {
                let next_sid = self.alloc_strain_id();
                let child_strain = strain.mutated_offspring(next_sid, &mut self.rng);
                let ang = self.rng.range_f32(0.0, TAU);
                new_virions.push(Virion {
                    x: (cx + ang.cos() * 6.0).clamp(6.0, WORLD_W - 6.0),
                    y: (cy + ang.sin() * 6.0).clamp(6.0, WORLD_H - 6.0),
                    vx: ang.cos() * 2.35,
                    vy: ang.sin() * 2.35,
                    age: 0.0,
                    max_age: self.rng.range_f32(440.0, 780.0),
                    strain: child_strain,
                    alive: true,
                });
            }
        }

        if should_divide {
            pending_mitoses.push(PendingMitosis {
                parent_idx: idx,
                parent_id: c_id,
                child_a_mode: mode.child_a_mode,
                child_b_mode: mode.child_b_mode,
                split_angle: mode.split_angle,
                keep_adhesion_parent: mode.keep_adhesion_parent,
                bond_children: mode.bond_children,
                adhesion_stiffness: mode.adhesion_stiffness,
            });
        }
    }

    fn execute_mitoses(&mut self, pending_mitoses: Vec<PendingMitosis>) {
        let rules_snapshot = self.mutation_rules.clone();
        for pm in pending_mitoses {
            if self.cells.len() >= MAX_CELLS {
                break;
            }
            if !self.cells[pm.parent_idx].alive {
                continue;
            }

            let active_modes = self.cells[pm.parent_idx].genome.active_modes.max(1);
            let mode_a = pm.child_a_mode % active_modes;
            let mode_b = pm.child_b_mode % active_modes;

            // Daughter A (parent transitions in-place to child_a_mode)
            let parent_atp = self.cells[pm.parent_idx].atp * 0.52;
            self.cells[pm.parent_idx].atp = parent_atp;
            self.cells[pm.parent_idx].mode_idx = mode_a;
            let role_a = self.cells[pm.parent_idx].role();
            self.cells[pm.parent_idx].radius = role_a.base_radius();
            self.cells[pm.parent_idx].max_atp = role_a.base_max_atp();
            if !pm.keep_adhesion_parent {
                let pid = pm.parent_id;
                self.bonds.retain(|b| b.a_id != pid && b.b_id != pid);
            }

            // Daughter B (new cell placed along angle + split_angle in child_b_mode)
            let parent = self.cells[pm.parent_idx].clone();
            let mut child_genome = parent.genome.clone();
            let local_mut = self.fields.sample_mutagen(parent.x, parent.y);
            let drift_scale = 1.0 + local_mut * 1.5;
            child_genome.mutate_natural(&mut self.rng, drift_scale);

            let mut child_brain = parent.brain.clone();
            let mut_rate = child_genome.mode(mode_b).mutation_rate * drift_scale;
            child_brain.mutate(&mut self.rng, mut_rate, 0.22);

            let child_org_id = if pm.keep_adhesion_parent || pm.bond_children {
                parent.organism_id
            } else {
                self.alloc_organism_id()
            };

            // Apply OnMitosis custom mutation rules if enabled
            if !rules_snapshot.is_empty() {
                let outcome = apply_mutation_rules(
                    &mut child_genome,
                    &mut child_brain,
                    mode_b,
                    &rules_snapshot,
                    MUT_TRIGGER_ON_MITOSIS,
                    drift_scale,
                    &mut self.rng,
                );
                if outcome.mutated {
                    let r = child_genome.mode(mode_b).role;
                    self.record_mutation_outcome(child_org_id, r, &outcome);
                }
            }

            let role_b = child_genome.mode(mode_b).role;
            let rad_b = role_b.base_radius();
            let cleave_ang =
                wrap_angle(parent.angle + pm.split_angle + self.rng.normal_approx() * 0.12);
            let sep = parent.radius + rad_b + 2.0;
            let raw_x = (parent.x + cleave_ang.cos() * sep).clamp(rad_b, WORLD_W - rad_b);
            let raw_y = (parent.y + cleave_ang.sin() * sep).clamp(rad_b, WORLD_H - rad_b);
            let (cx, cy, cvx, cvy) = self.fields.resolve_wall_collision(
                raw_x,
                raw_y,
                cleave_ang.cos() * 0.85,
                cleave_ang.sin() * 0.85,
                rad_b,
            );

            let child_id = self.alloc_cell_id();
            let max_hp = 85.0 + child_genome.mode(mode_b).membrane_armor * 80.0;
            let max_atp = role_b.base_max_atp();

            // Lysogenic provirus inheritance across mitosis!
            let (child_infected, child_dormant, child_vload) = if parent.lysogenic_dormant {
                (parent.infected_by.clone(), true, 0.12)
            } else {
                (None, false, 0.0)
            };

            let child = Cell {
                id: child_id,
                organism_id: child_org_id,
                lineage_id: parent.lineage_id,
                parent_id: pm.parent_id,
                generation: parent.generation.saturating_add(1),
                x: cx.clamp(rad_b, WORLD_W - rad_b),
                y: cy.clamp(rad_b, WORLD_H - rad_b),
                vx: cvx,
                vy: cvy,
                angle: cleave_ang,
                radius: rad_b,
                hp: max_hp,
                max_hp,
                atp: parent_atp.min(max_atp),
                max_atp,
                age: 0,
                divide_cooldown: 55,
                mode_idx: mode_b,
                genome: child_genome,
                brain: child_brain,
                synaptic_pulse: 0.0,
                incoming_synaptic: 0.0,
                myocyte_flex: 0.0,
                crispr_mask: parent.crispr_mask,
                viral_load: child_vload,
                incubation_progress: 0.0,
                infected_by: child_infected,
                lysogenic_dormant: child_dormant,
                temp_stress: parent.temp_stress,
                q10_rate: parent.q10_rate,
                signal_flags: SIGNAL_DIVIDE,
                bonded_count: 0,
                keratin_shield: 0.0,
                alive: true,
            };
            self.cells.push(child);
            self.total_births = self.total_births.saturating_add(1);

            if pm.keep_adhesion_parent || pm.bond_children {
                let stiff = pm.adhesion_stiffness.max(0.55);
                self.push_bond_pair(pm.parent_id, child_id, sep, stiff, pm.split_angle);
            }
        }
    }

    fn step_cell_interactions(&mut self, dt: f32) {
        let n = self.cells.len();
        for i in 0..n {
            if !self.cells[i].alive {
                continue;
            }
            let (ax, ay, ar, a_role, a_inf, a_lin, a_org) = {
                let c = &self.cells[i];
                (
                    c.x,
                    c.y,
                    c.radius,
                    c.role(),
                    c.is_actively_infected(),
                    c.lineage_id,
                    c.organism_id,
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
                        let (bx, by, br, b_role, b_inf, b_lin, b_org) = {
                            let o = &self.cells[j];
                            (
                                o.x,
                                o.y,
                                o.radius,
                                o.role(),
                                o.is_actively_infected(),
                                o.lineage_id,
                                o.organism_id,
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

                        // Immunocyte antiviral purging, CRISPR epitope acquisition & ally healing (30 px)
                        if d2 < 30.0 * 30.0 {
                            if a_role == CellRole::Immunocyte && !a_inf {
                                if let Some(strain) = self.cells[j].infected_by.clone() {
                                    self.cells[i].crispr_mask |= strain.crispr_bit();
                                    self.cells[j].crispr_mask |= strain.crispr_bit();
                                    self.cells[j].viral_load =
                                        (self.cells[j].viral_load - 0.08 * dt).max(0.0);
                                    if self.cells[j].viral_load <= 0.02 {
                                        self.cells[j].infected_by = None;
                                        self.cells[j].lysogenic_dormant = false;
                                        self.cells[j].incubation_progress = 0.0;
                                        self.total_neutralized =
                                            self.total_neutralized.saturating_add(1);
                                    }
                                } else if b_role != CellRole::Phagocyte
                                    && self.cells[j].hp < self.cells[j].max_hp
                                {
                                    self.cells[j].hp =
                                        (self.cells[j].hp + 0.42 * dt).min(self.cells[j].max_hp);
                                }
                            }
                            if b_role == CellRole::Immunocyte && !b_inf {
                                if let Some(strain) = self.cells[i].infected_by.clone() {
                                    self.cells[j].crispr_mask |= strain.crispr_bit();
                                    self.cells[i].crispr_mask |= strain.crispr_bit();
                                    self.cells[i].viral_load =
                                        (self.cells[i].viral_load - 0.08 * dt).max(0.0);
                                    if self.cells[i].viral_load <= 0.02 {
                                        self.cells[i].infected_by = None;
                                        self.cells[i].lysogenic_dormant = false;
                                        self.cells[i].incubation_progress = 0.0;
                                        self.total_neutralized =
                                            self.total_neutralized.saturating_add(1);
                                    }
                                } else if a_role != CellRole::Phagocyte
                                    && self.cells[i].hp < self.cells[i].max_hp
                                {
                                    self.cells[i].hp =
                                        (self.cells[i].hp + 0.42 * dt).min(self.cells[i].max_hp);
                                }
                            }
                        }

                        // Phagocyte predation / jaws on contact (mitigated by Keratinocyte & armor)
                        if d2 < (min_d + 4.5) * (min_d + 4.5) {
                            if a_role == CellRole::Phagocyte
                                && !a_inf
                                && b_lin != a_lin
                                && b_org != a_org
                            {
                                let b_arm = (self.cells[j].mode().membrane_armor
                                    + self.cells[j].keratin_shield)
                                    .clamp(0.0, 0.95);
                                let dmg = 5.6 * (1.0 - b_arm * 0.75) * dt;
                                self.cells[j].hp -= dmg;
                                self.cells[i].atp =
                                    (self.cells[i].atp + dmg * 0.98).min(self.cells[i].max_atp);
                                if self.cells[j].hp <= 0.0 {
                                    self.cells[j].alive = false;
                                    self.cells[i].atp =
                                        (self.cells[i].atp + 28.0).min(self.cells[i].max_atp);
                                }
                            } else if b_role == CellRole::Phagocyte
                                && !b_inf
                                && a_lin != b_lin
                                && a_org != b_org
                            {
                                let a_arm = (self.cells[i].mode().membrane_armor
                                    + self.cells[i].keratin_shield)
                                    .clamp(0.0, 0.95);
                                let dmg = 5.6 * (1.0 - a_arm * 0.75) * dt;
                                self.cells[i].hp -= dmg;
                                self.cells[j].atp =
                                    (self.cells[j].atp + dmg * 0.98).min(self.cells[j].max_atp);
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
            let local_temp = self.fields.sample_temp(vx, vy);
            let heat_inactivation =
                local_temp > 55.0 + strain.thermal_resistance * 35.0 && self.rng.chance(0.08 * dt);

            if age >= max_age || local_toxin > 0.24 || heat_inactivation {
                self.virions[v_idx].alive = false;
                if local_toxin > 0.24 {
                    self.total_neutralized = self.total_neutralized.saturating_add(1);
                }
                continue;
            }

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
                        let role = c.role();

                        // Immunocyte destroys adjacent virion & records CRISPR epitope!
                        if role == CellRole::Immunocyte
                            && !c.is_actively_infected()
                            && d2 < (c.radius + 14.0).powi(2)
                        {
                            c.crispr_mask |= strain.crispr_bit();
                            self.virions[v_idx].alive = false;
                            self.total_neutralized = self.total_neutralized.saturating_add(1);
                            self.fields.add_alarm(c.x, c.y, 0.22);
                            attached = true;
                            break 'bucket_loop;
                        }

                        if c.infected_by.is_none()
                            && strain.can_infect_role(role)
                            && d2 < (c.radius + 5.5).powi(2)
                        {
                            let has_crispr = (c.crispr_mask & strain.crispr_bit()) != 0;
                            let crispr_shield = if has_crispr { 0.72 } else { 0.0 };
                            let armor_shield =
                                (c.mode().membrane_armor + c.keratin_shield).min(0.90) * 0.25;
                            let effective_res =
                                (c.mode().immune_resistance + crispr_shield + armor_shield)
                                    .min(0.97)
                                    * (1.0 - strain.immune_evasion * 0.65);
                            let p_infect =
                                (strain.infectivity * (1.0 - effective_res)).clamp(0.02, 0.98);

                            if self.rng.chance(p_infect) {
                                c.infected_by = Some(strain.clone());
                                c.lysogenic_dormant = strain.strategy == ViralStrategy::Lysogenic;
                                c.viral_load = if c.lysogenic_dormant { 0.12 } else { 0.20 };
                                c.incubation_progress = 0.05;
                                self.fields.add_alarm(c.x, c.y, 0.28);
                            } else {
                                // Surviving viral contact teaches CRISPR adaptive immunity!
                                c.crispr_mask |= strain.crispr_bit();
                                self.total_neutralized = self.total_neutralized.saturating_add(1);
                            }
                            self.virions[v_idx].alive = false;
                            attached = true;
                            break 'bucket_loop;
                        } else if c.infected_by.is_none()
                            && strain.can_infect_role(role)
                            && d2 < 3400.0
                            && d2 > 1.0
                        {
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
                let nvx = (v.vx + attract_x * 0.18 + self.rng.normal_approx() * 0.34) * 0.90;
                let nvy = (v.vy + attract_y * 0.18 + self.rng.normal_approx() * 0.34) * 0.90;
                let rx = (v.x + nvx * dt).clamp(6.0, WORLD_W - 6.0);
                let ry = (v.y + nvy * dt).clamp(6.0, WORLD_H - 6.0);
                let (cx, cy, cvx, cvy) = self.fields.resolve_wall_collision(rx, ry, nvx, nvy, 4.0);
                v.x = cx.clamp(6.0, WORLD_W - 6.0);
                v.y = cy.clamp(6.0, WORLD_H - 6.0);
                v.vx = cvx;
                v.vy = cvy;
            }
        }
    }

    fn update_adhesion_bonds(&mut self, dt: f32) {
        self.id_to_idx.clear();
        for (i, c) in self.cells.iter_mut().enumerate() {
            c.bonded_count = 0;
            c.keratin_shield = 0.0;
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
            if dist > bond.rest_len * 2.55 || dist < 0.001 {
                continue;
            }

            let nx = dx / dist;
            let ny = dy / dist;
            let stretch = dist - bond.rest_len;
            let force = stretch * 0.095 * bond.stiffness * dt;
            let fx = nx * force;
            let fy = ny * force;
            self.cells[ia].vx += fx;
            self.cells[ia].vy += fy;
            self.cells[ib].vx -= fx;
            self.cells[ib].vy -= fy;

            // Myocyte contractile muscle flexing perpendicular to the bond axis!
            let role_a = self.cells[ia].role();
            let role_b = self.cells[ib].role();
            let flex_a = if role_a == CellRole::Myocyte {
                self.cells[ia].myocyte_flex
            } else {
                self.cells[ia].myocyte_flex * 0.25
            };
            let flex_b = if role_b == CellRole::Myocyte {
                self.cells[ib].myocyte_flex
            } else {
                self.cells[ib].myocyte_flex * 0.25
            };
            let net_flex = (flex_a - flex_b) * 0.36 * bond.stiffness * dt;
            let tx = -ny * net_flex;
            let ty = nx * net_flex;
            self.cells[ia].vx += tx;
            self.cells[ia].vy += ty;
            self.cells[ib].vx -= tx;
            self.cells[ib].vy -= ty;

            // Vascular ATP sharing across tissue junctions (amplified by Vascularocytes)
            let vasc_a = self.cells[ia].mode().vascular_share;
            let vasc_b = self.cells[ib].mode().vascular_share;
            let vasc_mult =
                if role_a == CellRole::Vascularocyte || role_b == CellRole::Vascularocyte {
                    0.14
                } else {
                    0.065
                };
            let share_rate = ((vasc_a + vasc_b) * 0.5).max(0.25) * vasc_mult * dt;
            let atp_diff = self.cells[ia].atp - self.cells[ib].atp;
            let transfer = atp_diff * share_rate;
            let a_max = self.cells[ia].max_atp;
            let b_max = self.cells[ib].max_atp;
            self.cells[ia].atp = (self.cells[ia].atp - transfer).clamp(0.0, a_max);
            self.cells[ib].atp = (self.cells[ib].atp + transfer).clamp(0.0, b_max);

            // Synaptic Signal Impulse propagation (Neurocyte / Senseocyte -> Flagellocyte / Myocyte / Secreocyte)
            let pulse_a = self.cells[ia].synaptic_pulse;
            let pulse_b = self.cells[ib].synaptic_pulse;
            self.cells[ib].incoming_synaptic =
                (self.cells[ib].incoming_synaptic + pulse_a * 0.65).clamp(-1.0, 1.0);
            self.cells[ia].incoming_synaptic =
                (self.cells[ia].incoming_synaptic + pulse_b * 0.65).clamp(-1.0, 1.0);
            let syn_flow = (pulse_a.abs() + pulse_b.abs()) * 0.5;

            // Share CRISPR adaptive immunity & Keratinocyte armor shielding across bonded tissues!
            let combined_crispr = self.cells[ia].crispr_mask | self.cells[ib].crispr_bit_mask();
            self.cells[ia].crispr_mask = combined_crispr;
            self.cells[ib].crispr_mask = combined_crispr;
            if role_a == CellRole::Keratinocyte {
                self.cells[ib].keratin_shield = (self.cells[ib].keratin_shield + 0.30).min(0.45);
            }
            if role_b == CellRole::Keratinocyte {
                self.cells[ia].keratin_shield = (self.cells[ia].keratin_shield + 0.30).min(0.45);
            }

            self.cells[ia].bonded_count = self.cells[ia].bonded_count.saturating_add(1);
            self.cells[ib].bonded_count = self.cells[ib].bonded_count.saturating_add(1);

            let mut updated = *bond;
            updated.last_vascular_flow = transfer;
            updated.last_synaptic_flow = syn_flow;
            kept_bonds.push(updated);
        }
        self.bonds = kept_bonds;

        // Form new adhesion bonds between nearby compatible adhesive cells.
        let n = self.cells.len();
        for i in 0..n {
            if self.bonds.len() >= MAX_BONDS {
                break;
            }
            let (ca_id, ca_x, ca_y, ca_r, ca_lin, ca_stiff, ca_max, ca_b) = {
                let ca = &self.cells[i];
                if !ca.alive {
                    continue;
                }
                let m = ca.mode();
                (
                    ca.id,
                    ca.x,
                    ca.y,
                    ca.radius,
                    ca.lineage_id,
                    m.adhesion_stiffness,
                    m.max_bonds,
                    ca.bonded_count,
                )
            };
            if ca_stiff <= 0.42 || ca_b >= ca_max {
                continue;
            }
            let gx = ((ca_x / HASH_CELL) as isize).clamp(0, HASH_W as isize - 1) as usize;
            let gy = ((ca_y / HASH_CELL) as isize).clamp(0, HASH_H as isize - 1) as usize;
            let mut local_links = ca_b;

            for ny in gy.saturating_sub(1)..=(gy + 1).min(HASH_H - 1) {
                for nx in gx.saturating_sub(1)..=(gx + 1).min(HASH_W - 1) {
                    if self.bonds.len() >= MAX_BONDS || local_links >= ca_max {
                        break;
                    }
                    let b_idx = ny * HASH_W + nx;
                    for &j in &self.cell_buckets[b_idx] {
                        if j <= i {
                            continue;
                        }
                        let cb = &self.cells[j];
                        let mb = cb.mode();
                        if !cb.alive
                            || cb.lineage_id != ca_lin
                            || mb.adhesion_stiffness <= 0.42
                            || cb.bonded_count >= mb.max_bonds
                        {
                            continue;
                        }
                        let dx = cb.x - ca_x;
                        let dy = cb.y - ca_y;
                        let max_bind = ca_r + cb.radius + 12.0;
                        if dx * dx + dy * dy <= max_bind * max_bind {
                            let exists = self.bonds.iter().any(|b| {
                                (b.a_id == ca_id && b.b_id == cb.id)
                                    || (b.a_id == cb.id && b.b_id == ca_id)
                            });
                            if !exists {
                                let r_len = (ca_r + cb.radius + 3.5).clamp(10.0, 34.0);
                                let stiff =
                                    ((ca_stiff + mb.adhesion_stiffness) * 0.5).clamp(0.15, 1.0);
                                self.bonds.push(AdhesionBond {
                                    a_id: ca_id,
                                    b_id: cb.id,
                                    rest_len: r_len,
                                    stiffness: stiff,
                                    preferred_angle: 0.0,
                                    last_vascular_flow: 0.0,
                                    last_synaptic_flow: 0.0,
                                });
                                local_links += 1;
                                if self.bonds.len() >= MAX_BONDS || local_links >= ca_max {
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
        self.mutation_log_buf.clear();

        let mut role_counts = [0_u32; NUM_CELL_ROLES];
        let mut sum_t_opt = 0.0_f32;
        let mut sum_immune = 0.0_f32;
        let mut sum_adhesion = 0.0_f32;
        let mut sum_atp = 0.0_f32;
        let mut max_gen = 0_u32;
        let mut infected_count = 0_u32;
        let mut crispr_count = 0_u32;
        let mut selected_idx: Option<usize> = None;

        for (i, c) in self.cells.iter().enumerate() {
            if !c.alive {
                continue;
            }
            let role = c.role();
            let mode = c.mode();
            role_counts[role.as_u8() as usize] += 1;
            sum_t_opt += mode.t_opt;
            let crispr_bonus = if c.crispr_mask != 0 { 0.20 } else { 0.0 };
            sum_immune += (mode.immune_resistance + crispr_bonus).min(1.0);
            sum_adhesion += mode.adhesion_stiffness;
            sum_atp += (c.atp / c.max_atp.max(1.0)).clamp(0.0, 1.0);
            if c.generation > max_gen {
                max_gen = c.generation;
            }
            if c.infected_by.is_some() {
                infected_count += 1;
            }
            if c.crispr_mask != 0 {
                crispr_count += 1;
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
                role.as_u8() as f32,
                (c.hp / c.max_hp.max(1.0)).clamp(0.0, 1.0),
                (c.atp / c.max_atp.max(1.0)).clamp(0.0, 1.0),
                c.viral_load,
                c.temp_stress,
                mode.adhesion_stiffness,
                mode.hue_shift,
                c.generation as f32,
                c.signal_flags as f32,
                mode.immune_resistance,
                c.lineage_id as f32,
                c.organism_id as f32,
                c.mode_idx as f32,
                c.brain.cognitive_state.as_u8() as f32,
                c.synaptic_pulse,
            ]);
        }

        // If selected cell died, auto-select the first living cell so Brain X-Ray stays live.
        if selected_idx.is_none() && !self.cells.is_empty() {
            if let Some((i, c)) = self.cells.iter().enumerate().find(|(_, c)| c.alive) {
                self.selected_cell_id = c.id;
                selected_idx = Some(i);
            }
        }

        for v in &self.virions {
            if !v.alive {
                continue;
            }
            self.virions_buf.extend_from_slice(&[
                v.x,
                v.y,
                v.vx,
                v.vy,
                v.strain.hue,
                v.strain.strategy.as_u8() as f32,
            ]);
        }

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
                self.bonds_buf.extend_from_slice(&[
                    ca.x,
                    ca.y,
                    cb.x,
                    cb.y,
                    b.stiffness,
                    b.last_vascular_flow,
                    b.last_synaptic_flow,
                ]);
            }
        }

        for entry in &self.mutation_log {
            self.mutation_log_buf.extend_from_slice(&[
                entry.tick as f32,
                entry.organism_id as f32,
                entry.role as f32,
                entry.target_kind as f32,
                entry.delta,
                entry.mode_idx as f32,
            ]);
        }

        let pop = (self.cells_buf.len() / CELL_STRIDE) as f32;
        let inv_pop = if pop > 0.0 { 1.0 / pop } else { 0.0 };
        self.stats_buf = [
            self.tick as f32,
            pop,
            (self.virions_buf.len() / VIRION_STRIDE) as f32,
            (self.bonds_buf.len() / BOND_STRIDE) as f32,
            self.total_mutations as f32,
            self.total_births as f32,
            self.total_bursts as f32,
            self.total_neutralized as f32,
            sum_t_opt * inv_pop,
            sum_immune * inv_pop,
            sum_adhesion * inv_pop,
            sum_atp * inv_pop,
            self.fields.mean_temp(),
            self.fields.mean_light(),
            self.fields.mean_nutrient(),
            self.fields.mean_mutagen(),
            max_gen as f32,
            infected_count as f32,
            crispr_count as f32,
            if selected_idx.is_some() { 1.0 } else { 0.0 },
            role_counts[0] as f32,
            role_counts[1] as f32,
            role_counts[2] as f32,
            role_counts[3] as f32,
            role_counts[4] as f32,
            role_counts[5] as f32,
            role_counts[6] as f32,
            role_counts[7] as f32,
            role_counts[8] as f32,
            role_counts[9] as f32,
            role_counts[10] as f32,
            role_counts[11] as f32,
        ];

        self.inspector_buf.fill(0.0);
        if let Some(s_idx) = selected_idx {
            let c = &self.cells[s_idx];
            let mode = c.mode();
            let local_temp = self.fields.sample_temp(c.x, c.y);
            let inf_strat = match &c.infected_by {
                Some(s) => s.strategy.as_u8() as f32,
                None => -1.0,
            };
            let header: [f32; 32] = [
                1.0,
                c.id as f32,
                c.role().as_u8() as f32,
                c.mode_idx as f32,
                c.organism_id as f32,
                c.generation as f32,
                c.lineage_id as f32,
                c.hp,
                c.max_hp,
                c.atp,
                c.max_atp,
                c.age as f32,
                local_temp,
                mode.t_opt,
                mode.t_tol,
                mode.metabolism,
                mode.membrane_armor,
                mode.adhesion_stiffness,
                mode.immune_resistance,
                mode.mutation_rate,
                mode.hebbian_rate,
                c.viral_load,
                c.incubation_progress,
                c.temp_stress,
                c.q10_rate,
                c.brain.cognitive_state.as_u8() as f32,
                c.synaptic_pulse,
                c.crispr_mask as f32,
                c.bonded_count as f32,
                c.brain.hebbian_delta,
                inf_strat,
                mode.hue_shift,
            ];
            self.inspector_buf[..32].copy_from_slice(&header);

            let mut off = 32;
            self.inspector_buf[off..off + NN_INPUTS].copy_from_slice(&c.brain.last_inputs);
            off += NN_INPUTS;
            self.inspector_buf[off..off + NN_HIDDEN].copy_from_slice(&c.brain.last_hidden);
            off += NN_HIDDEN;
            self.inspector_buf[off..off + NN_MEMORY].copy_from_slice(&c.brain.memory);
            off += NN_MEMORY;
            self.inspector_buf[off..off + NN_OUTPUTS].copy_from_slice(&c.brain.last_outputs);
            off += NN_OUTPUTS;
            self.inspector_buf[off..off + NN_W1_LEN].copy_from_slice(&c.brain.w1);
            off += NN_W1_LEN;
            self.inspector_buf[off..off + NN_W_MEM_LEN].copy_from_slice(&c.brain.w_mem);
            off += NN_W_MEM_LEN;
            self.inspector_buf[off..off + NN_W2_LEN].copy_from_slice(&c.brain.w2);
        }
    }
}

impl Cell {
    #[inline]
    fn crispr_bit_mask(&self) -> u16 {
        self.crispr_mask
    }
}

#[inline]
fn wrap_angle(mut a: f32) -> f32 {
    while a > PI {
        a -= TAU;
    }
    while a < -PI {
        a += TAU;
    }
    a
}

#[allow(dead_code)]
fn _cognitive_state_marker(s: CognitiveState) -> u8 {
    s.as_u8()
}
