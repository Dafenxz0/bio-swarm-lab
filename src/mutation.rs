//! Configurable mutagenesis rules, targeted genomic & neural operators, and mutation event logging.

use crate::genome::{CellRole, OrganismGenome, MAX_MODES, NUM_CELL_ROLES};
use crate::neural::NeuralNet;
use crate::rng::Rng;
use core::f32::consts::PI;

pub const MUT_TARGET_SPLIT_GEOMETRY: u32 = 1 << 0;
pub const MUT_TARGET_ADHESION: u32 = 1 << 1;
pub const MUT_TARGET_DIFFERENTIATION: u32 = 1 << 2;
pub const MUT_TARGET_PHYSIOLOGY: u32 = 1 << 3;
pub const MUT_TARGET_IMMUNITY_ARMOR: u32 = 1 << 4;
pub const MUT_TARGET_NEURAL: u32 = 1 << 5;
pub const MUT_TARGET_SECRETION_HUE: u32 = 1 << 6;
pub const MUT_TARGET_ALL: u32 = (1 << 7) - 1;

pub const MUT_TRIGGER_ON_MITOSIS: u8 = 0;
pub const MUT_TRIGGER_ON_MUTAGEN_FIELD: u8 = 1;
pub const MUT_TRIGGER_ON_THERMAL_STRESS: u8 = 2;
pub const MUT_TRIGGER_DIRECT_ONLY: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MutationRule {
    pub id: u32,
    pub enabled: bool,
    pub target_mask: u32,
    pub bias: f32,
    pub strength: f32,
    pub probability: f32,
    pub trigger: u8,
}

impl MutationRule {
    /// Build one of 4 pre-configured mutation rules (`0..=3`) for initial lab registries.
    pub fn preset(id: u32, preset_idx: u8) -> Self {
        match preset_idx {
            // 0: Mitotic Morphogenesis & Adhesion Drift
            0 => Self {
                id,
                enabled: true,
                target_mask: MUT_TARGET_SPLIT_GEOMETRY
                    | MUT_TARGET_ADHESION
                    | MUT_TARGET_DIFFERENTIATION,
                bias: 0.15,
                strength: 0.25,
                probability: 0.35,
                trigger: MUT_TRIGGER_ON_MITOSIS,
            },
            // 1: Radiation Field Neural & Role Hyper-Mutator
            1 => Self {
                id,
                enabled: true,
                target_mask: MUT_TARGET_DIFFERENTIATION
                    | MUT_TARGET_NEURAL
                    | MUT_TARGET_SECRETION_HUE,
                bias: 0.10,
                strength: 0.45,
                probability: 0.55,
                trigger: MUT_TRIGGER_ON_MUTAGEN_FIELD,
            },
            // 2: Thermal Extremophile Physiological Adaptation
            2 => Self {
                id,
                enabled: true,
                target_mask: MUT_TARGET_PHYSIOLOGY | MUT_TARGET_IMMUNITY_ARMOR,
                bias: 0.20,
                strength: 0.32,
                probability: 0.45,
                trigger: MUT_TRIGGER_ON_THERMAL_STRESS,
            },
            // 3: Direct CRISPR & Synaptic Overdrive
            _ => Self {
                id,
                enabled: true,
                target_mask: MUT_TARGET_ALL,
                bias: 0.25,
                strength: 0.50,
                probability: 1.0,
                trigger: MUT_TRIGGER_DIRECT_ONLY,
            },
        }
    }

    pub fn clamp_fields(&mut self) {
        if self.target_mask & MUT_TARGET_ALL == 0 {
            self.target_mask = MUT_TARGET_ALL;
        } else {
            self.target_mask &= MUT_TARGET_ALL;
        }
        self.bias = self.bias.clamp(-1.0, 1.0);
        self.strength = self.strength.clamp(0.01, 1.0);
        self.probability = self.probability.clamp(0.0, 1.0);
        self.trigger = self.trigger.min(3);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MutationEvent {
    pub rule_id: u32,
    pub target_kind: u8,
    pub mode_idx: u8,
    pub delta: f32,
}

#[derive(Clone, Debug, Default)]
pub struct MutationOutcome {
    pub mutated: bool,
    pub applied_mask: u32,
    pub primary_target: u8,
    pub total_delta: f32,
    pub events: Vec<MutationEvent>,
}

/// Apply all matching `MutationRule`s for a given `trigger` (`0=OnMitosis`, `1=OnMutagenField`,
/// `2=OnThermalStress`, `3=DirectOnly`) to `genome` and `brain`.
pub fn apply_mutation_rules(
    genome: &mut OrganismGenome,
    brain: &mut NeuralNet,
    mode_idx: u8,
    rules: &[MutationRule],
    trigger: u8,
    env_intensity: f32,
    rng: &mut Rng,
) -> MutationOutcome {
    let mut outcome = MutationOutcome::default();
    let intensity = env_intensity.clamp(0.1, 2.0);

    for rule in rules {
        if !rule.enabled || rule.trigger != trigger {
            continue;
        }
        let eff_prob = if trigger == MUT_TRIGGER_DIRECT_ONLY {
            rule.probability.max(0.95)
        } else {
            (rule.probability * intensity).clamp(0.0, 1.0)
        };
        if !rng.chance(eff_prob) {
            continue;
        }
        apply_rule_into(genome, brain, mode_idx, rule, intensity, rng, &mut outcome);
    }
    outcome
}

/// Directly execute a single `MutationRule` (e.g. from FFI `sim_trigger_mutation`).
pub fn apply_single_mutation_rule(
    genome: &mut OrganismGenome,
    brain: &mut NeuralNet,
    mode_idx: u8,
    rule: &MutationRule,
    env_intensity: f32,
    rng: &mut Rng,
) -> MutationOutcome {
    let mut outcome = MutationOutcome::default();
    let intensity = env_intensity.clamp(0.2, 2.0);
    apply_rule_into(genome, brain, mode_idx, rule, intensity, rng, &mut outcome);
    outcome
}

fn apply_rule_into(
    genome: &mut OrganismGenome,
    brain: &mut NeuralNet,
    mode_idx: u8,
    rule: &MutationRule,
    intensity: f32,
    rng: &mut Rng,
    outcome: &mut MutationOutcome,
) {
    let m_idx = (mode_idx as usize).min(MAX_MODES - 1);
    let s = (rule.strength * intensity).clamp(0.02, 1.25);
    let b = rule.bias.clamp(-1.0, 1.0);
    let mask = if rule.target_mask & MUT_TARGET_ALL == 0 {
        MUT_TARGET_ALL
    } else {
        rule.target_mask & MUT_TARGET_ALL
    };

    // Bit 0: SplitGeometry / Angles (`split_mass`, `split_angle`)
    if (mask & MUT_TARGET_SPLIT_GEOMETRY) != 0 {
        let m = &mut genome.modes[m_idx];
        let d_ang = (rng.normal_approx() + b * 0.7) * 0.65 * s;
        let d_mass = (rng.normal_approx() - b * 0.4) * 0.14 * s;
        m.split_angle = (m.split_angle + d_ang).clamp(-PI, PI);
        m.split_mass = (m.split_mass + d_mass).clamp(0.45, 1.35);
        record_event(outcome, rule.id, 0, m_idx as u8, d_ang);
    }

    // Bit 1: Adhesion / Multicellularity (`keep_adhesion_parent`, `bond_children`, `adhesion_stiffness`, `max_bonds`)
    if (mask & MUT_TARGET_ADHESION) != 0 {
        let m = &mut genome.modes[m_idx];
        let d_stiff = (rng.normal_approx() + b * 0.8) * 0.28 * s;
        m.adhesion_stiffness = (m.adhesion_stiffness + d_stiff).clamp(0.0, 1.0);
        if b > 0.15 || rng.chance(0.45 * s) {
            m.keep_adhesion_parent = b >= -0.15 || !m.keep_adhesion_parent;
            m.bond_children = b >= -0.05 || m.adhesion_stiffness > 0.45;
        }
        if rng.chance(0.40 * s) {
            let delta_b = if b >= 0.0 { 1_i32 } else { -1_i32 };
            m.max_bonds = (m.max_bonds as i32 + delta_b).clamp(1, 8) as u8;
        }
        record_event(outcome, rule.id, 1, m_idx as u8, d_stiff);
    }

    // Bit 2: Mode Differentiation (`role`, `child_a_mode`, `child_b_mode`, `active_modes`)
    if (mask & MUT_TARGET_DIFFERENTIATION) != 0 {
        let mut delta_val = 0.25 * s;
        if genome.active_modes < MAX_MODES as u8 && (b >= 0.0 || rng.chance(0.5)) {
            genome.active_modes = (genome.active_modes + 1).min(MAX_MODES as u8);
        }
        let max_m = genome.active_modes.max(2);
        let m = &mut genome.modes[m_idx];
        if rng.chance(0.65) {
            let step_r = 1 + rng.range_u32(3) as u8;
            let next_u8 = (m.role.as_u8() + step_r) % (NUM_CELL_ROLES as u8);
            let next_role = CellRole::from_u8(next_u8);
            delta_val = (next_role.as_u8() as f32 - m.role.as_u8() as f32) * 0.1;
            m.role = next_role;
        }
        m.child_a_mode = (m.child_a_mode.wrapping_add(1)) % max_m;
        if rng.chance(0.65) {
            m.child_b_mode = (m.child_b_mode.wrapping_add(2)) % max_m;
        }
        record_event(outcome, rule.id, 2, m_idx as u8, delta_val);
    }

    // Bit 3: Physiology / Thermal / Metabolism (`t_opt`, `t_tol`, `metabolism`, `light_eff`, `vascular_share`)
    if (mask & MUT_TARGET_PHYSIOLOGY) != 0 {
        let m = &mut genome.modes[m_idx];
        let d_temp = (rng.normal_approx() + b * 0.9) * 9.5 * s;
        let d_metab = (rng.normal_approx() + b * 0.5) * 0.26 * s;
        let d_tol = (rng.normal_approx() + b * 0.4) * 4.0 * s;
        let d_light = (rng.normal_approx() + b * 0.5) * 0.25 * s;
        let d_vasc = (rng.normal_approx() + b * 0.4) * 0.20 * s;
        m.t_opt = (m.t_opt + d_temp).clamp(-25.0, 95.0);
        m.t_tol = (m.t_tol + d_tol).clamp(5.0, 40.0);
        m.metabolism = (m.metabolism + d_metab).clamp(0.40, 2.40);
        m.light_eff = (m.light_eff + d_light).clamp(0.0, 2.0);
        m.vascular_share = (m.vascular_share + d_vasc).clamp(0.0, 1.0);
        record_event(outcome, rule.id, 3, m_idx as u8, d_temp * 0.1 + d_metab);
    }

    // Bit 4: Armor / Immunity / Sensory (`membrane_armor`, `immune_resistance`, `sensory_range`)
    if (mask & MUT_TARGET_IMMUNITY_ARMOR) != 0 {
        let m = &mut genome.modes[m_idx];
        let d_imm = (rng.normal_approx() + b * 0.85) * 0.25 * s;
        let d_arm = (rng.normal_approx() + b * 0.70) * 0.22 * s;
        let d_sens = (rng.normal_approx() + b * 0.5) * 18.0 * s;
        m.immune_resistance = (m.immune_resistance + d_imm).clamp(0.02, 0.98);
        m.membrane_armor = (m.membrane_armor + d_arm).clamp(0.02, 0.95);
        m.sensory_range = (m.sensory_range + d_sens).clamp(40.0, 220.0);
        record_event(outcome, rule.id, 4, m_idx as u8, d_imm + d_arm);
    }

    // Bit 5: Neural Weights & Hebbian Plasticity (`brain` synapses + `hebbian_rate`)
    if (mask & MUT_TARGET_NEURAL) != 0 {
        let m = &mut genome.modes[m_idx];
        let d_heb = (rng.normal_approx() + b * 0.6) * 0.08 * s;
        m.hebbian_rate = (m.hebbian_rate + d_heb).clamp(0.0, 0.50);
        let mut_rate = (0.22 + 0.35 * s).clamp(0.05, 0.65);
        let mut_str = (0.28 * s).clamp(0.04, 0.85);
        brain.mutate(rng, mut_rate, mut_str);
        for out_b in &mut brain.b2 {
            *out_b = (*out_b + b * 0.12 * s).clamp(-3.0, 3.0);
        }
        record_event(outcome, rule.id, 5, m_idx as u8, d_heb + 0.25 * s);
    }

    // Bit 6: Secretion Kind & Hue Shift (`secretion_kind`, `hue_shift`, `mutation_rate`)
    if (mask & MUT_TARGET_SECRETION_HUE) != 0 {
        let m = &mut genome.modes[m_idx];
        let d_hue = (rng.normal_approx() + b * 0.5) * 0.14 * s;
        let d_mrate = (rng.normal_approx() + b * 0.4) * 0.06 * s;
        m.hue_shift = (m.hue_shift + d_hue).clamp(-0.5, 0.5);
        m.mutation_rate = (m.mutation_rate + d_mrate).clamp(0.005, 0.45);
        if rng.chance(0.55 * s) {
            m.secretion_kind = (m.secretion_kind + 1 + (rng.range_u32(4) as u8)) % 6;
        }
        record_event(outcome, rule.id, 6, m_idx as u8, d_hue);
    }

    genome.modes[m_idx].clamp_fields();
}

fn record_event(
    outcome: &mut MutationOutcome,
    rule_id: u32,
    target_kind: u8,
    mode_idx: u8,
    delta: f32,
) {
    let clean_delta = if delta.abs() < 0.01 { 0.05 } else { delta };
    if !outcome.mutated {
        outcome.mutated = true;
        outcome.primary_target = target_kind;
    }
    outcome.applied_mask |= 1u32 << target_kind;
    outcome.total_delta += clean_delta;
    outcome.events.push(MutationEvent {
        rule_id,
        target_kind,
        mode_idx,
        delta: clean_delta,
    });
}
