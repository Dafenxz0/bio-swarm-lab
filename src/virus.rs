//! Viral replication strategies (Lytic, Lysogenic, OncogenicMutator), strain registry traits,
//! host cell-role tropism masks, CRISPR antigenic signatures, and antigenic drift.

use crate::genome::CellRole;
use crate::rng::Rng;

pub const ALL_ROLES_MASK: u16 = (1 << 12) - 1;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViralStrategy {
    /// 0: Replicates rapidly in cytoplasm and lyses host into `burst_count` virions.
    Lytic = 0,
    /// 1: Integrates as dormant provirus inherited across mitosis; activates into lytic under stress.
    Lysogenic = 1,
    /// 2: Hijacks host neural weights, forces unregulated mitosis, and mutates host `OrganismGenome`.
    OncogenicMutator = 2,
}

impl ViralStrategy {
    #[inline]
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Lytic,
            1 => Self::Lysogenic,
            _ => Self::OncogenicMutator,
        }
    }

    #[inline]
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    #[inline]
    pub fn name(self) -> &'static str {
        match self {
            Self::Lytic => "Lytic",
            Self::Lysogenic => "Lysogenic",
            Self::OncogenicMutator => "Oncogenic Mutator",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VirusStrain {
    pub id: u32,
    pub generation: u32,
    pub strategy: ViralStrategy,
    pub target_role_mask: u16,
    pub infectivity: f32,
    pub incubation_ticks: f32,
    pub burst_count: u8,
    pub lethality: f32,
    pub mutation_rate: f32,
    pub immune_evasion: f32,
    pub thermal_resistance: f32,
    pub enveloped: bool,
    pub hue: f32,
    pub antigenic_sig: u8,
}

impl VirusStrain {
    /// Build one of 3 distinct baseline viral strain archetypes (`0=Lytic`, `1=Lysogenic`, `2=Oncogenic`).
    pub fn preset(id: u32, preset_idx: u8) -> Self {
        match preset_idx {
            0 => Self {
                id,
                generation: 1,
                strategy: ViralStrategy::Lytic,
                target_role_mask: ALL_ROLES_MASK & !CellRole::Keratinocyte.role_bit(),
                infectivity: 0.72,
                incubation_ticks: 95.0,
                burst_count: 6,
                lethality: 0.68,
                mutation_rate: 0.22,
                immune_evasion: 0.25,
                thermal_resistance: 0.45,
                enveloped: false,
                hue: 0.04,
                antigenic_sig: (id as u8) & 15,
            },
            1 => Self {
                id,
                generation: 1,
                strategy: ViralStrategy::Lysogenic,
                target_role_mask: ALL_ROLES_MASK,
                infectivity: 0.65,
                incubation_ticks: 165.0,
                burst_count: 5,
                lethality: 0.45,
                mutation_rate: 0.18,
                immune_evasion: 0.42,
                thermal_resistance: 0.62,
                enveloped: true,
                hue: 0.48,
                antigenic_sig: ((id as u8).wrapping_add(5)) & 15,
            },
            _ => Self {
                id,
                generation: 1,
                strategy: ViralStrategy::OncogenicMutator,
                target_role_mask: ALL_ROLES_MASK,
                infectivity: 0.68,
                incubation_ticks: 130.0,
                burst_count: 4,
                lethality: 0.35,
                mutation_rate: 0.38,
                immune_evasion: 0.50,
                thermal_resistance: 0.55,
                enveloped: true,
                hue: 0.14,
                antigenic_sig: ((id as u8).wrapping_add(11)) & 15,
            },
        }
    }

    pub fn clamp_fields(&mut self) {
        if self.target_role_mask & ALL_ROLES_MASK == 0 {
            self.target_role_mask = ALL_ROLES_MASK;
        } else {
            self.target_role_mask &= ALL_ROLES_MASK;
        }
        self.infectivity = self.infectivity.clamp(0.05, 1.0);
        self.incubation_ticks = self.incubation_ticks.clamp(20.0, 500.0);
        self.burst_count = self.burst_count.clamp(2, 16);
        self.lethality = self.lethality.clamp(0.05, 1.0);
        self.mutation_rate = self.mutation_rate.clamp(0.0, 0.65);
        self.immune_evasion = self.immune_evasion.clamp(0.0, 0.95);
        self.thermal_resistance = self.thermal_resistance.clamp(0.0, 1.0);
        if self.hue > 1.0 {
            self.hue = (self.hue / 360.0).rem_euclid(1.0);
        } else {
            self.hue = self.hue.rem_euclid(1.0);
        }
        self.antigenic_sig &= 15;
    }

    #[inline]
    pub fn can_infect_role(&self, role: CellRole) -> bool {
        (self.target_role_mask & role.role_bit()) != 0
    }

    /// 16-bit CRISPR adaptive immunity epitope bit corresponding to this strain.
    #[inline]
    pub fn crispr_bit(&self) -> u16 {
        1u16 << (self.antigenic_sig & 15)
    }

    /// Antigenic drift & viral evolution helper creating a mutated variant sub-strain on burst/replication.
    pub fn mutated_offspring(&self, next_id: u32, rng: &mut Rng) -> Self {
        let mut child = self.clone();
        child.id = next_id;
        child.generation = self.generation.saturating_add(1);
        let m = self.mutation_rate;
        if m > 0.0 && rng.chance(0.35 + m) {
            child.infectivity += rng.normal_approx() * 0.15 * m;
            child.incubation_ticks += rng.normal_approx() * 34.0 * m;
            child.lethality += rng.normal_approx() * 0.14 * m;
            child.immune_evasion += rng.normal_approx() * 0.16 * m;
            child.thermal_resistance += rng.normal_approx() * 0.14 * m;
            child.hue += rng.normal_approx() * 0.07 * m;
            if rng.chance(m * 0.40) {
                let delta = if rng.chance(0.5) { 1_i32 } else { -1_i32 };
                child.burst_count = (child.burst_count as i32 + delta).clamp(2, 16) as u8;
            }
            // Host-range tropism shift
            if rng.chance(m * 0.25) {
                let bit = 1u16 << rng.range_u32(12);
                let toggled = child.target_role_mask ^ bit;
                if toggled & ALL_ROLES_MASK != 0 {
                    child.target_role_mask = toggled & ALL_ROLES_MASK;
                }
            }
            // Antigenic epitope drift allowing partial escape from existing CRISPR memory
            if rng.chance(m * 0.30) {
                child.antigenic_sig = (child.antigenic_sig.wrapping_add(1)) & 15;
            }
            if rng.chance(m * 0.15) {
                child.enveloped = !child.enveloped;
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
