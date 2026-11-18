//! Cell lineages, evolvable genome traits, Q10 thermal physiology, and viral strains.

use crate::rng::Rng;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellRole {
    Producer = 0,
    Protector = 1,
    Predator = 2,
    Swimmer = 3,
    Replicator = 4,
    Infected = 5,
}

impl CellRole {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Producer,
            1 => Self::Protector,
            2 => Self::Predator,
            3 => Self::Swimmer,
            4 => Self::Replicator,
            _ => Self::Infected,
        }
    }

    pub fn base_radius(self) -> f32 {
        match self {
            Self::Producer => 7.5,
            Self::Protector => 8.2,
            Self::Predator => 9.0,
            Self::Swimmer => 6.2,
            Self::Replicator => 7.0,
            Self::Infected => 8.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Genome {
    pub role: CellRole,
    pub t_opt: f32,
    pub t_tol: f32,
    pub metabolism: f32,
    pub membrane_armor: f32,
    pub adhesion: f32,
    pub immune_resistance: f32,
    pub mutation_rate: f32,
    pub neural_bias: f32,
    pub hue_shift: f32,
}

impl Genome {
    pub fn default_for_role(role: CellRole, rng: &mut Rng) -> Self {
        let mut g = match role {
            CellRole::Producer => Self {
                role,
                t_opt: 25.0,
                t_tol: 18.0,
                metabolism: 0.90,
                membrane_armor: 0.25,
                adhesion: 0.35,
                immune_resistance: 0.28,
                mutation_rate: 0.08,
                neural_bias: 0.0,
                hue_shift: 0.0,
            },
            CellRole::Protector => Self {
                role,
                t_opt: 27.0,
                t_tol: 20.0,
                metabolism: 1.10,
                membrane_armor: 0.45,
                adhesion: 0.30,
                immune_resistance: 0.72,
                mutation_rate: 0.07,
                neural_bias: 0.15,
                hue_shift: 0.0,
            },
            CellRole::Predator => Self {
                role,
                t_opt: 26.0,
                t_tol: 18.0,
                metabolism: 1.25,
                membrane_armor: 0.40,
                adhesion: 0.12,
                immune_resistance: 0.35,
                mutation_rate: 0.09,
                neural_bias: 0.20,
                hue_shift: 0.0,
            },
            CellRole::Swimmer => Self {
                role,
                t_opt: 24.0,
                t_tol: 21.0,
                metabolism: 1.15,
                membrane_armor: 0.18,
                adhesion: 0.15,
                immune_resistance: 0.30,
                mutation_rate: 0.10,
                neural_bias: 0.25,
                hue_shift: 0.0,
            },
            CellRole::Replicator => Self {
                role,
                t_opt: 25.0,
                t_tol: 17.0,
                metabolism: 1.30,
                membrane_armor: 0.20,
                adhesion: 0.32,
                immune_resistance: 0.25,
                mutation_rate: 0.16,
                neural_bias: 0.10,
                hue_shift: 0.0,
            },
            CellRole::Infected => Self {
                role: CellRole::Producer,
                t_opt: 25.0,
                t_tol: 18.0,
                metabolism: 1.0,
                membrane_armor: 0.20,
                adhesion: 0.20,
                immune_resistance: 0.15,
                mutation_rate: 0.10,
                neural_bias: 0.0,
                hue_shift: 0.0,
            },
        };
        g.t_opt = (g.t_opt + rng.normal_approx() * 2.5).clamp(-25.0, 95.0);
        g.hue_shift = rng.range_f32(-0.08, 0.08);
        g
    }

    /// Classic Q10 metabolic scaling: rate = 2^((T - T_opt) / 10), clamped for stability.
    #[inline]
    pub fn q10_rate(&self, temp: f32) -> f32 {
        let exp = ((temp - self.t_opt) / 10.0).clamp(-2.0, 1.7);
        2.0_f32.powf(exp).clamp(0.25, 3.2)
    }

    /// Signed thermal stress outside tolerance `[T_opt - T_tol, T_opt + T_tol]`.
    /// Positive => heat shock; negative => frostbite; 0 => inside thermal tolerance window.
    #[inline]
    pub fn thermal_stress(&self, temp: f32) -> f32 {
        let tol = self.t_tol.max(2.0);
        let diff = temp - self.t_opt;
        if diff > tol {
            (diff - tol) / tol
        } else if diff < -tol {
            (diff + tol) / tol
        } else {
            0.0
        }
    }

    /// Mutate genomic traits during mitosis.
    pub fn mutate(&mut self, rng: &mut Rng) {
        let m = self.mutation_rate.clamp(0.01, 0.45);
        if rng.chance(m * 1.5 + 0.15) {
            self.t_opt = (self.t_opt + rng.normal_approx() * 8.0 * m).clamp(-25.0, 95.0);
        }
        if rng.chance(m + 0.10) {
            self.t_tol = (self.t_tol + rng.normal_approx() * 4.0 * m).clamp(6.0, 38.0);
        }
        if rng.chance(m + 0.10) {
            self.metabolism = (self.metabolism + rng.normal_approx() * 0.25 * m).clamp(0.45, 2.2);
        }
        if rng.chance(m + 0.10) {
            self.membrane_armor =
                (self.membrane_armor + rng.normal_approx() * 0.20 * m).clamp(0.02, 0.90);
        }
        if rng.chance(m + 0.10) {
            self.adhesion = (self.adhesion + rng.normal_approx() * 0.28 * m).clamp(0.0, 1.0);
        }
        if rng.chance(m + 0.12) {
            self.immune_resistance =
                (self.immune_resistance + rng.normal_approx() * 0.26 * m).clamp(0.02, 0.95);
        }
        if rng.chance(m * 0.8) {
            self.mutation_rate =
                (self.mutation_rate + rng.normal_approx() * 0.06 * m).clamp(0.01, 0.40);
        }
        if rng.chance(m + 0.10) {
            self.neural_bias = (self.neural_bias + rng.normal_approx() * 0.35 * m).clamp(-1.0, 1.0);
        }
        self.hue_shift = (self.hue_shift + rng.normal_approx() * 0.04 * m).clamp(-0.35, 0.35);
    }
}

#[derive(Clone, Debug)]
pub struct VirusStrain {
    pub id: u32,
    pub generation: u32,
    pub infectivity: f32,
    pub incubation_ticks: f32,
    pub burst_count: u8,
    pub lethality: f32,
    pub mutation_rate: f32,
    pub immune_evasion: f32,
    pub strain_hue: f32,
}

impl VirusStrain {
    pub fn clamp_fields(&mut self) {
        self.infectivity = self.infectivity.clamp(0.05, 1.0);
        self.incubation_ticks = self.incubation_ticks.clamp(25.0, 450.0);
        self.burst_count = self.burst_count.clamp(2, 12);
        self.lethality = self.lethality.clamp(0.05, 1.0);
        self.mutation_rate = self.mutation_rate.clamp(0.0, 0.6);
        self.immune_evasion = self.immune_evasion.clamp(0.0, 0.95);
        self.strain_hue = self.strain_hue.rem_euclid(1.0);
    }

    pub fn mutated_offspring(&self, next_id: u32, rng: &mut Rng) -> Self {
        let mut child = self.clone();
        child.id = next_id;
        child.generation = self.generation.saturating_add(1);
        let m = self.mutation_rate;
        if m > 0.0 && rng.chance(0.35 + m) {
            child.infectivity += rng.normal_approx() * 0.16 * m;
            child.incubation_ticks += rng.normal_approx() * 35.0 * m;
            child.lethality += rng.normal_approx() * 0.15 * m;
            child.immune_evasion += rng.normal_approx() * 0.16 * m;
            child.strain_hue += rng.normal_approx() * 0.08 * m;
            if rng.chance(m * 0.4) {
                let delta = if rng.chance(0.5) { 1_i32 } else { -1_i32 };
                child.burst_count = (child.burst_count as i32 + delta).clamp(2, 12) as u8;
            }
            child.clamp_fields();
        }
        child
    }
}

#[derive(Clone, Debug)]
pub struct Virion {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub age: f32,
    pub max_age: f32,
    pub strain: VirusStrain,
    pub alive: bool,
}
