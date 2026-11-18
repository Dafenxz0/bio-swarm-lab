//! Cell Lab 12-type differentiation roles, 8-mode organism genomes, Q10 thermal physiology, and 5 preset conscious creatures.

use crate::rng::Rng;
use core::f32::consts::PI;

pub const NUM_CELL_ROLES: usize = 12;
pub const MAX_MODES: usize = 8;
pub const PRESET_CREATURE_COUNT: u8 = 5;

pub const SECRETION_NONE: u8 = 0;
pub const SECRETION_FOOD_PHEROMONE: u8 = 1;
pub const SECRETION_ALARM: u8 = 2;
pub const SECRETION_ANTIBODY: u8 = 3;
pub const SECRETION_TOXIN: u8 = 4;
pub const SECRETION_NUTRIENT_MUCUS: u8 = 5;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellRole {
    /// 0: Chloroplast solar collector converting ambient `light` + `nutrients` into ATP.
    Photocyte = 0,
    /// 1: Predator jaw cell capable of biting/engulfing foreign lineages for ATP & biomass.
    Phagocyte = 1,
    /// 2: High-thrust flagellar tail propelling attached multicellular organisms.
    Flagellocyte = 2,
    /// 3: Contractile muscle cell flexing spring adhesion bond angles via neural `myocyte_flex`.
    Myocyte = 3,
    /// 4: Deep neural pacemaker & synaptic relay broadcasting impulses across adhesion bonds.
    Neurocyte = 4,
    /// 5: Long-range optical & chemotactic sensory receptor feeding bonded neural relays.
    Senseocyte = 5,
    /// 6: T-cell / macrophage neutralizing virions, curing allies, and recording CRISPR immunity.
    Immunocyte = 6,
    /// 7: Heavily armored epidermal shield blocking predator bites and viral attachment.
    Keratinocyte = 7,
    /// 8: Specialized exocrine gland secreting configurable chemical payloads (`secretion_kind`).
    Secreocyte = 8,
    /// 9: High-throughput vascular pump equalizing and distributing ATP across adhesion bonds.
    Vascularocyte = 9,
    /// 10: Pluripotent stem / gamete cell optimized for rapid differentiated mitosis.
    Stemocyte = 10,
    /// 11: Detritivore & detoxifier recycling dead organic matter into nutrients and clearing toxins.
    Scavenger = 11,
}

impl CellRole {
    #[inline]
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Photocyte,
            1 => Self::Phagocyte,
            2 => Self::Flagellocyte,
            3 => Self::Myocyte,
            4 => Self::Neurocyte,
            5 => Self::Senseocyte,
            6 => Self::Immunocyte,
            7 => Self::Keratinocyte,
            8 => Self::Secreocyte,
            9 => Self::Vascularocyte,
            10 => Self::Stemocyte,
            _ => Self::Scavenger,
        }
    }

    #[inline]
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    #[inline]
    pub fn role_bit(self) -> u16 {
        1u16 << (self as u8)
    }

    #[inline]
    pub fn name(self) -> &'static str {
        match self {
            Self::Photocyte => "Photocyte",
            Self::Phagocyte => "Phagocyte",
            Self::Flagellocyte => "Flagellocyte",
            Self::Myocyte => "Myocyte",
            Self::Neurocyte => "Neurocyte",
            Self::Senseocyte => "Senseocyte",
            Self::Immunocyte => "Immunocyte",
            Self::Keratinocyte => "Keratinocyte",
            Self::Secreocyte => "Secreocyte",
            Self::Vascularocyte => "Vascularocyte",
            Self::Stemocyte => "Stemocyte",
            Self::Scavenger => "Scavenger",
        }
    }

    #[inline]
    pub fn base_radius(self) -> f32 {
        match self {
            Self::Photocyte => 7.6,
            Self::Phagocyte => 8.8,
            Self::Flagellocyte => 6.2,
            Self::Myocyte => 7.2,
            Self::Neurocyte => 6.8,
            Self::Senseocyte => 6.6,
            Self::Immunocyte => 8.0,
            Self::Keratinocyte => 8.6,
            Self::Secreocyte => 7.4,
            Self::Vascularocyte => 7.2,
            Self::Stemocyte => 7.0,
            Self::Scavenger => 7.5,
        }
    }

    #[inline]
    pub fn base_speed(self) -> f32 {
        match self {
            Self::Flagellocyte => 2.85,
            Self::Myocyte => 1.85,
            Self::Phagocyte => 2.10,
            Self::Immunocyte => 1.95,
            Self::Senseocyte => 1.55,
            Self::Scavenger => 1.65,
            Self::Secreocyte => 1.25,
            Self::Stemocyte => 1.15,
            Self::Neurocyte => 1.05,
            Self::Vascularocyte => 0.95,
            Self::Photocyte => 0.85,
            Self::Keratinocyte => 0.70,
        }
    }

    #[inline]
    pub fn base_max_atp(self) -> f32 {
        match self {
            Self::Stemocyte => 95.0,
            Self::Vascularocyte => 150.0,
            Self::Phagocyte => 140.0,
            Self::Photocyte => 135.0,
            Self::Immunocyte => 125.0,
            Self::Myocyte => 125.0,
            Self::Flagellocyte => 120.0,
            _ => 115.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellMode {
    pub role: CellRole,
    pub split_mass: f32,
    pub split_angle: f32,
    pub child_a_mode: u8,
    pub child_b_mode: u8,
    pub keep_adhesion_parent: bool,
    pub bond_children: bool,
    pub adhesion_stiffness: f32,
    pub max_bonds: u8,

    pub t_opt: f32,
    pub t_tol: f32,
    pub metabolism: f32,
    pub light_eff: f32,
    pub membrane_armor: f32,
    pub immune_resistance: f32,
    pub vascular_share: f32,
    pub secretion_kind: u8,
    pub sensory_range: f32,
    pub hebbian_rate: f32,
    pub mutation_rate: f32,
    pub hue_shift: f32,
}

impl CellMode {
    pub fn default_for_role(role: CellRole, mode_idx: u8) -> Self {
        let idx = mode_idx.min((MAX_MODES - 1) as u8);
        let mut m = Self {
            role,
            split_mass: 0.76,
            split_angle: 0.0,
            child_a_mode: idx,
            child_b_mode: idx,
            keep_adhesion_parent: false,
            bond_children: false,
            adhesion_stiffness: 0.35,
            max_bonds: 3,
            t_opt: 25.0,
            t_tol: 19.0,
            metabolism: 1.0,
            light_eff: 0.25,
            membrane_armor: 0.25,
            immune_resistance: 0.30,
            vascular_share: 0.35,
            secretion_kind: SECRETION_NONE,
            sensory_range: 105.0,
            hebbian_rate: 0.08,
            mutation_rate: 0.08,
            hue_shift: 0.0,
        };

        match role {
            CellRole::Photocyte => {
                m.light_eff = 1.35;
                m.metabolism = 0.88;
                m.vascular_share = 0.65;
                m.secretion_kind = SECRETION_FOOD_PHEROMONE;
            }
            CellRole::Phagocyte => {
                m.metabolism = 1.28;
                m.membrane_armor = 0.44;
                m.sensory_range = 125.0;
                m.hebbian_rate = 0.12;
            }
            CellRole::Flagellocyte => {
                m.metabolism = 1.22;
                m.membrane_armor = 0.18;
                m.adhesion_stiffness = 0.55;
            }
            CellRole::Myocyte => {
                m.metabolism = 1.18;
                m.adhesion_stiffness = 0.75;
                m.keep_adhesion_parent = true;
                m.bond_children = true;
                m.vascular_share = 0.55;
            }
            CellRole::Neurocyte => {
                m.metabolism = 1.05;
                m.sensory_range = 145.0;
                m.hebbian_rate = 0.22;
                m.keep_adhesion_parent = true;
                m.bond_children = true;
                m.vascular_share = 0.60;
            }
            CellRole::Senseocyte => {
                m.sensory_range = 185.0;
                m.hebbian_rate = 0.16;
                m.light_eff = 0.45;
                m.secretion_kind = SECRETION_ALARM;
            }
            CellRole::Immunocyte => {
                m.membrane_armor = 0.46;
                m.immune_resistance = 0.78;
                m.secretion_kind = SECRETION_ANTIBODY;
                m.sensory_range = 135.0;
            }
            CellRole::Keratinocyte => {
                m.membrane_armor = 0.86;
                m.immune_resistance = 0.72;
                m.metabolism = 0.75;
                m.adhesion_stiffness = 0.82;
                m.max_bonds = 5;
            }
            CellRole::Secreocyte => {
                m.metabolism = 1.12;
                m.secretion_kind = SECRETION_TOXIN;
                m.immune_resistance = 0.52;
            }
            CellRole::Vascularocyte => {
                m.vascular_share = 0.92;
                m.light_eff = 0.55;
                m.adhesion_stiffness = 0.72;
                m.max_bonds = 6;
                m.keep_adhesion_parent = true;
                m.bond_children = true;
            }
            CellRole::Stemocyte => {
                m.split_mass = 0.62;
                m.metabolism = 1.32;
                m.mutation_rate = 0.14;
                m.vascular_share = 0.60;
                m.max_bonds = 5;
            }
            CellRole::Scavenger => {
                m.metabolism = 1.08;
                m.immune_resistance = 0.64;
                m.membrane_armor = 0.36;
                m.secretion_kind = SECRETION_NUTRIENT_MUCUS;
                m.sensory_range = 130.0;
            }
        }
        m
    }

    pub fn clamp_fields(&mut self) {
        self.split_mass = self.split_mass.clamp(0.45, 3.50);
        self.split_angle = self.split_angle.clamp(-PI, PI);
        self.child_a_mode = self.child_a_mode.min((MAX_MODES - 1) as u8);
        self.child_b_mode = self.child_b_mode.min((MAX_MODES - 1) as u8);
        self.adhesion_stiffness = self.adhesion_stiffness.clamp(0.0, 1.0);
        self.max_bonds = self.max_bonds.clamp(0, 8);
        self.t_opt = self.t_opt.clamp(-25.0, 95.0);
        self.t_tol = self.t_tol.clamp(5.0, 40.0);
        self.metabolism = self.metabolism.clamp(0.40, 2.40);
        self.light_eff = self.light_eff.clamp(0.0, 2.0);
        self.membrane_armor = self.membrane_armor.clamp(0.02, 0.95);
        self.immune_resistance = self.immune_resistance.clamp(0.02, 0.98);
        self.vascular_share = self.vascular_share.clamp(0.0, 1.0);
        self.secretion_kind = self.secretion_kind.min(5);
        self.sensory_range = self.sensory_range.clamp(25.0, 220.0);
        self.hebbian_rate = self.hebbian_rate.clamp(0.0, 0.50);
        self.mutation_rate = self.mutation_rate.clamp(0.0, 0.45);
        if self.hue_shift.abs() > 1.0 {
            self.hue_shift = (self.hue_shift / 360.0).clamp(-0.5, 0.5);
        } else {
            self.hue_shift = self.hue_shift.clamp(-0.5, 0.5);
        }
    }

    /// Classic Q10 metabolic scaling: `rate = 2^((T - T_opt) / 10)`, clamped for stability.
    #[inline]
    pub fn q10_rate(&self, temp: f32) -> f32 {
        let exp = ((temp - self.t_opt) / 10.0).clamp(-2.0, 1.7);
        2.0_f32.powf(exp).clamp(0.25, 3.2)
    }

    /// Signed thermal stress outside tolerance `[T_opt - T_tol, T_opt + T_tol]`.
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

    /// Convert `split_mass` (`0.45..1.0` ratio or `1.0..3.5` mass multiplier) into an ATP threshold.
    #[inline]
    pub fn split_atp_threshold(&self, max_atp: f32) -> f32 {
        let frac = if self.split_mass <= 1.05 {
            self.split_mass.clamp(0.45, 0.92)
        } else {
            (0.48 + (self.split_mass - 1.0) * 0.18).clamp(0.48, 0.92)
        };
        (max_atp * frac).max(38.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrganismGenome {
    pub id: u32,
    pub preset_id: u8,
    pub active_modes: u8,
    pub initial_mode: u8,
    pub modes: [CellMode; MAX_MODES],
}

impl OrganismGenome {
    pub fn single_role(role: CellRole, rng: &mut Rng) -> Self {
        let mut modes = [CellMode::default_for_role(role, 0); MAX_MODES];
        for (i, m) in modes.iter_mut().enumerate() {
            *m = CellMode::default_for_role(role, i as u8);
            m.t_opt = (m.t_opt + rng.normal_approx() * 2.2).clamp(-25.0, 95.0);
            m.hue_shift = rng.range_f32(-0.07, 0.07);
        }
        Self {
            id: 0,
            preset_id: 255,
            active_modes: 1,
            initial_mode: 0,
            modes,
        }
    }

    #[inline]
    pub fn preset_name(preset_id: u8) -> &'static str {
        match preset_id {
            0 => "Apex Leviathan",
            1 => "Solar Reef Symbiont",
            2 => "Swarm Valkyrie",
            3 => "Macrophage Sentinel",
            _ => "Segmented Muscle Worm",
        }
    }

    /// Build one of the 5 Conscious Multi-Cell Creature Presets (`0..=4`).
    pub fn preset(preset_id: u8) -> Self {
        let mut modes = [CellMode::default_for_role(CellRole::Stemocyte, 0); MAX_MODES];
        for (i, m) in modes.iter_mut().enumerate() {
            m.child_a_mode = i as u8;
            m.child_b_mode = i as u8;
        }

        match preset_id {
            // 0: Apex Leviathan — 11-cell streamlined bilateral predator with Senseocyte nose,
            // paired Phagocyte jaws, Neurocyte brain, Vascularocyte heart, Myocyte trunk,
            // Keratinocyte flank plates, and twin Flagellocyte propulsion tails.
            0 => {
                // M0: Neurocyte brain -> anterior Senseocyte head (M1) + posterior Vascularocyte heart (M2)
                modes[0] = CellMode::default_for_role(CellRole::Neurocyte, 0);
                modes[0].split_mass = 0.72;
                modes[0].split_angle = 0.0;
                modes[0].child_a_mode = 1;
                modes[0].child_b_mode = 2;
                modes[0].keep_adhesion_parent = true;
                modes[0].bond_children = false;
                modes[0].adhesion_stiffness = 0.88;
                modes[0].max_bonds = 5;

                // M1: Senseocyte head -> twin forward Phagocyte jaws (M3) at +/-0.56 rad
                modes[1] = CellMode::default_for_role(CellRole::Senseocyte, 1);
                modes[1].split_mass = 0.74;
                modes[1].split_angle = 0.56;
                modes[1].child_a_mode = 3;
                modes[1].child_b_mode = 3;
                modes[1].keep_adhesion_parent = true;
                modes[1].bond_children = true;
                modes[1].adhesion_stiffness = 0.86;
                modes[1].max_bonds = 4;

                // M2: Vascularocyte heart -> twin posterior Myocyte muscle flanks (M4) at +/-0.48 rad
                modes[2] = CellMode::default_for_role(CellRole::Vascularocyte, 2);
                modes[2].split_mass = 0.74;
                modes[2].split_angle = 0.48;
                modes[2].child_a_mode = 4;
                modes[2].child_b_mode = 4;
                modes[2].keep_adhesion_parent = true;
                modes[2].bond_children = true;
                modes[2].adhesion_stiffness = 0.88;
                modes[2].max_bonds = 5;

                // M3: Phagocyte raptorial jaw (terminal sensory-guided effector)
                modes[3] = CellMode::default_for_role(CellRole::Phagocyte, 3);
                modes[3].split_mass = 0.84;
                modes[3].split_angle = 0.0;
                modes[3].child_a_mode = 3;
                modes[3].child_b_mode = 3;
                modes[3].keep_adhesion_parent = true;
                modes[3].bond_children = false;
                modes[3].adhesion_stiffness = 0.82;
                modes[3].max_bonds = 4;

                // M4: Myocyte contractile trunk -> Flagellocyte tail (M5) + Keratinocyte armor flank (M6)
                modes[4] = CellMode::default_for_role(CellRole::Myocyte, 4);
                modes[4].split_mass = 0.78;
                modes[4].split_angle = 0.38;
                modes[4].child_a_mode = 5;
                modes[4].child_b_mode = 6;
                modes[4].keep_adhesion_parent = true;
                modes[4].bond_children = true;
                modes[4].adhesion_stiffness = 0.86;
                modes[4].max_bonds = 5;

                // M5: Flagellocyte twin propulsion tail (terminal thruster)
                modes[5] = CellMode::default_for_role(CellRole::Flagellocyte, 5);
                modes[5].split_mass = 0.88;
                modes[5].split_angle = 0.0;
                modes[5].child_a_mode = 5;
                modes[5].child_b_mode = 5;
                modes[5].keep_adhesion_parent = true;
                modes[5].bond_children = false;
                modes[5].adhesion_stiffness = 0.78;
                modes[5].max_bonds = 3;

                // M6: Keratinocyte dorsal armor plate (terminal shield)
                modes[6] = CellMode::default_for_role(CellRole::Keratinocyte, 6);
                modes[6].split_mass = 0.88;
                modes[6].split_angle = 0.0;
                modes[6].child_a_mode = 6;
                modes[6].child_b_mode = 6;
                modes[6].keep_adhesion_parent = true;
                modes[6].bond_children = false;
                modes[6].adhesion_stiffness = 0.88;
                modes[6].max_bonds = 4;

                Self {
                    id: 1,
                    preset_id: 0,
                    active_modes: 7,
                    initial_mode: 0,
                    modes,
                }
            }

            // 1: Solar Reef Symbiont — 13-cell radial photosynthetic coral rosette with Vascularocyte core,
            // Stemocyte branches, Photocyte solar fronds, Immunocyte guardians, and Keratinocyte bark.
            1 => {
                // M0: Vascularocyte central hub -> M1 (upper stalk) + M2 (lower stalk)
                modes[0] = CellMode::default_for_role(CellRole::Vascularocyte, 0);
                modes[0].split_mass = 0.72;
                modes[0].split_angle = 0.0;
                modes[0].child_a_mode = 1;
                modes[0].child_b_mode = 2;
                modes[0].keep_adhesion_parent = true;
                modes[0].bond_children = false;
                modes[0].adhesion_stiffness = 0.90;
                modes[0].vascular_share = 0.95;
                modes[0].max_bonds = 6;

                // M1 & M2: Stemocyte stalks -> symmetric Photocyte solar fronds (M3) at +/-0.92 rad
                modes[1] = CellMode::default_for_role(CellRole::Stemocyte, 1);
                modes[1].split_mass = 0.74;
                modes[1].split_angle = 0.92;
                modes[1].child_a_mode = 3;
                modes[1].child_b_mode = 3;
                modes[1].keep_adhesion_parent = true;
                modes[1].bond_children = true;
                modes[1].adhesion_stiffness = 0.86;
                modes[1].max_bonds = 5;

                modes[2] = CellMode::default_for_role(CellRole::Stemocyte, 2);
                modes[2].split_mass = 0.74;
                modes[2].split_angle = 0.92;
                modes[2].child_a_mode = 3;
                modes[2].child_b_mode = 3;
                modes[2].keep_adhesion_parent = true;
                modes[2].bond_children = true;
                modes[2].adhesion_stiffness = 0.86;
                modes[2].max_bonds = 5;

                // M3: Photocyte solar leaf -> Immunocyte polyp (M4) + Keratinocyte guard (M5)
                modes[3] = CellMode::default_for_role(CellRole::Photocyte, 3);
                modes[3].split_mass = 0.80;
                modes[3].split_angle = 0.62;
                modes[3].child_a_mode = 4;
                modes[3].child_b_mode = 5;
                modes[3].keep_adhesion_parent = true;
                modes[3].bond_children = true;
                modes[3].adhesion_stiffness = 0.84;
                modes[3].light_eff = 1.50;
                modes[3].vascular_share = 0.82;
                modes[3].max_bonds = 4;

                // M4: Immunocyte polyp guardian (terminal)
                modes[4] = CellMode::default_for_role(CellRole::Immunocyte, 4);
                modes[4].split_mass = 0.86;
                modes[4].child_a_mode = 4;
                modes[4].child_b_mode = 4;
                modes[4].keep_adhesion_parent = true;
                modes[4].bond_children = false;
                modes[4].adhesion_stiffness = 0.82;
                modes[4].max_bonds = 3;

                // M5: Keratinocyte calyx armor (terminal)
                modes[5] = CellMode::default_for_role(CellRole::Keratinocyte, 5);
                modes[5].split_mass = 0.88;
                modes[5].child_a_mode = 5;
                modes[5].child_b_mode = 5;
                modes[5].keep_adhesion_parent = true;
                modes[5].bond_children = false;
                modes[5].adhesion_stiffness = 0.90;
                modes[5].max_bonds = 4;

                Self {
                    id: 2,
                    preset_id: 1,
                    active_modes: 6,
                    initial_mode: 0,
                    modes,
                }
            }

            // 2: Swarm Valkyrie — 9-cell swept delta-wing hunter-interceptor with Senseocyte nose,
            // Neurocyte avionics core, Phagocyte wingtip lances, Immunocyte guards, and Flagellocyte drive.
            2 => {
                // M0: Neurocyte avionics core -> Senseocyte nose (M1) + Flagellocyte thruster (M2)
                modes[0] = CellMode::default_for_role(CellRole::Neurocyte, 0);
                modes[0].split_mass = 0.72;
                modes[0].split_angle = 0.0;
                modes[0].child_a_mode = 1;
                modes[0].child_b_mode = 2;
                modes[0].keep_adhesion_parent = true;
                modes[0].bond_children = false;
                modes[0].adhesion_stiffness = 0.88;
                modes[0].max_bonds = 5;

                // M1: Senseocyte nose cone -> swept-back delta wings of Phagocyte lances (M3) at +/-1.65 rad
                modes[1] = CellMode::default_for_role(CellRole::Senseocyte, 1);
                modes[1].split_mass = 0.74;
                modes[1].split_angle = 1.65;
                modes[1].child_a_mode = 3;
                modes[1].child_b_mode = 3;
                modes[1].keep_adhesion_parent = true;
                modes[1].bond_children = true;
                modes[1].adhesion_stiffness = 0.84;
                modes[1].max_bonds = 4;

                // M2: Flagellocyte core engine -> twin rear Secreocyte alarm/chaff pods (M5)
                modes[2] = CellMode::default_for_role(CellRole::Flagellocyte, 2);
                modes[2].split_mass = 0.78;
                modes[2].split_angle = 0.70;
                modes[2].child_a_mode = 5;
                modes[2].child_b_mode = 5;
                modes[2].keep_adhesion_parent = true;
                modes[2].bond_children = true;
                modes[2].adhesion_stiffness = 0.82;
                modes[2].max_bonds = 4;

                // M3: Phagocyte wingtip -> Immunocyte wing-guard (M4) single rear bud
                modes[3] = CellMode::default_for_role(CellRole::Phagocyte, 3);
                modes[3].split_mass = 0.80;
                modes[3].split_angle = 0.45;
                modes[3].child_a_mode = 4;
                modes[3].child_b_mode = 3;
                modes[3].keep_adhesion_parent = true;
                modes[3].bond_children = false;
                modes[3].adhesion_stiffness = 0.82;
                modes[3].max_bonds = 4;

                // M4: Immunocyte wing-guard (terminal)
                modes[4] = CellMode::default_for_role(CellRole::Immunocyte, 4);
                modes[4].split_mass = 0.86;
                modes[4].child_a_mode = 4;
                modes[4].child_b_mode = 4;
                modes[4].keep_adhesion_parent = true;
                modes[4].bond_children = false;
                modes[4].adhesion_stiffness = 0.82;
                modes[4].max_bonds = 3;

                // M5: Secreocyte pod (terminal)
                modes[5] = CellMode::default_for_role(CellRole::Secreocyte, 5);
                modes[5].secretion_kind = SECRETION_ALARM;
                modes[5].split_mass = 0.86;
                modes[5].child_a_mode = 5;
                modes[5].child_b_mode = 5;
                modes[5].keep_adhesion_parent = true;
                modes[5].bond_children = false;
                modes[5].adhesion_stiffness = 0.80;
                modes[5].max_bonds = 3;

                Self {
                    id: 3,
                    preset_id: 2,
                    active_modes: 6,
                    initial_mode: 0,
                    modes,
                }
            }

            // 3: Macrophage Sentinel — 11-cell star-armored immune patrol fortress with Immunocyte core,
            // Senseocyte scanner, Secreocyte antibody emitters, Scavenger arms, and Flagellocyte twin drive.
            3 => {
                // M0: Immunocyte central core -> Senseocyte bow (M1) + Keratinocyte stern bulkhead (M2)
                modes[0] = CellMode::default_for_role(CellRole::Immunocyte, 0);
                modes[0].split_mass = 0.72;
                modes[0].split_angle = 0.0;
                modes[0].child_a_mode = 1;
                modes[0].child_b_mode = 2;
                modes[0].keep_adhesion_parent = true;
                modes[0].bond_children = false;
                modes[0].adhesion_stiffness = 0.88;
                modes[0].max_bonds = 5;

                // M1: Senseocyte bow -> twin Secreocyte antibody arms (M3)
                modes[1] = CellMode::default_for_role(CellRole::Senseocyte, 1);
                modes[1].split_mass = 0.74;
                modes[1].split_angle = 1.05;
                modes[1].child_a_mode = 3;
                modes[1].child_b_mode = 3;
                modes[1].keep_adhesion_parent = true;
                modes[1].bond_children = true;
                modes[1].adhesion_stiffness = 0.84;
                modes[1].max_bonds = 4;

                // M2: Keratinocyte stern shield -> twin Flagellocyte patrol thrusters (M5)
                modes[2] = CellMode::default_for_role(CellRole::Keratinocyte, 2);
                modes[2].split_mass = 0.78;
                modes[2].split_angle = 0.65;
                modes[2].child_a_mode = 5;
                modes[2].child_b_mode = 5;
                modes[2].keep_adhesion_parent = true;
                modes[2].bond_children = true;
                modes[2].adhesion_stiffness = 0.88;
                modes[2].max_bonds = 5;

                // M3: Secreocyte antibody emitter -> Scavenger arm (M4) single outward bud
                modes[3] = CellMode::default_for_role(CellRole::Secreocyte, 3);
                modes[3].secretion_kind = SECRETION_ANTIBODY;
                modes[3].split_mass = 0.78;
                modes[3].split_angle = 0.55;
                modes[3].child_a_mode = 4;
                modes[3].child_b_mode = 3;
                modes[3].keep_adhesion_parent = true;
                modes[3].bond_children = false;
                modes[3].adhesion_stiffness = 0.82;
                modes[3].max_bonds = 4;

                // M4: Scavenger detoxifier (terminal)
                modes[4] = CellMode::default_for_role(CellRole::Scavenger, 4);
                modes[4].split_mass = 0.86;
                modes[4].child_a_mode = 4;
                modes[4].child_b_mode = 4;
                modes[4].keep_adhesion_parent = true;
                modes[4].bond_children = false;
                modes[4].adhesion_stiffness = 0.80;
                modes[4].max_bonds = 3;

                // M5: Flagellocyte thruster (terminal)
                modes[5] = CellMode::default_for_role(CellRole::Flagellocyte, 5);
                modes[5].split_mass = 0.86;
                modes[5].child_a_mode = 5;
                modes[5].child_b_mode = 5;
                modes[5].keep_adhesion_parent = true;
                modes[5].bond_children = false;
                modes[5].adhesion_stiffness = 0.78;
                modes[5].max_bonds = 3;

                Self {
                    id: 4,
                    preset_id: 3,
                    active_modes: 6,
                    initial_mode: 0,
                    modes,
                }
            }

            // 4: Segmented Muscle Worm — 9-cell articulated sinuous swimmer driven by Senseocyte nose,
            // Neurocyte ganglion, chained Myocyte contractile segments, Photocyte dorsal spine, Vascularocyte, & Flagellocyte tail.
            _ => {
                // M0: Neurocyte neck -> Senseocyte head forward (M1) + Myocyte seg1 backward (M2)
                modes[0] = CellMode::default_for_role(CellRole::Neurocyte, 0);
                modes[0].split_mass = 0.72;
                modes[0].split_angle = 0.0;
                modes[0].child_a_mode = 1;
                modes[0].child_b_mode = 2;
                modes[0].keep_adhesion_parent = true;
                modes[0].bond_children = false;
                modes[0].adhesion_stiffness = 0.90;
                modes[0].max_bonds = 3;

                // M1: Senseocyte snout (terminal head)
                modes[1] = CellMode::default_for_role(CellRole::Senseocyte, 1);
                modes[1].split_mass = 0.88;
                modes[1].split_angle = 0.0;
                modes[1].child_a_mode = 1;
                modes[1].child_b_mode = 1;
                modes[1].keep_adhesion_parent = true;
                modes[1].bond_children = false;
                modes[1].adhesion_stiffness = 0.88;
                modes[1].max_bonds = 3;

                // M2..M6: Linear axial spine chain along body axis (child_b_mode == self -> single segment!)
                modes[2] = CellMode::default_for_role(CellRole::Myocyte, 2);
                modes[2].split_mass = 0.74;
                modes[2].split_angle = 0.02;
                modes[2].child_a_mode = 3;
                modes[2].child_b_mode = 2;
                modes[2].keep_adhesion_parent = true;
                modes[2].bond_children = false;
                modes[2].adhesion_stiffness = 0.88;
                modes[2].max_bonds = 3;

                modes[3] = CellMode::default_for_role(CellRole::Myocyte, 3);
                modes[3].split_mass = 0.74;
                modes[3].split_angle = -0.02;
                modes[3].child_a_mode = 4;
                modes[3].child_b_mode = 3;
                modes[3].keep_adhesion_parent = true;
                modes[3].bond_children = false;
                modes[3].adhesion_stiffness = 0.88;
                modes[3].max_bonds = 3;

                modes[4] = CellMode::default_for_role(CellRole::Photocyte, 4);
                modes[4].light_eff = 1.45;
                modes[4].vascular_share = 0.85;
                modes[4].split_mass = 0.76;
                modes[4].split_angle = 0.02;
                modes[4].child_a_mode = 5;
                modes[4].child_b_mode = 4;
                modes[4].keep_adhesion_parent = true;
                modes[4].bond_children = false;
                modes[4].adhesion_stiffness = 0.86;
                modes[4].max_bonds = 3;

                modes[5] = CellMode::default_for_role(CellRole::Myocyte, 5);
                modes[5].split_mass = 0.76;
                modes[5].split_angle = -0.02;
                modes[5].child_a_mode = 6;
                modes[5].child_b_mode = 5;
                modes[5].keep_adhesion_parent = true;
                modes[5].bond_children = false;
                modes[5].adhesion_stiffness = 0.88;
                modes[5].max_bonds = 3;

                modes[6] = CellMode::default_for_role(CellRole::Vascularocyte, 6);
                modes[6].split_mass = 0.78;
                modes[6].split_angle = 0.0;
                modes[6].child_a_mode = 7;
                modes[6].child_b_mode = 6;
                modes[6].keep_adhesion_parent = true;
                modes[6].bond_children = false;
                modes[6].adhesion_stiffness = 0.88;
                modes[6].max_bonds = 3;

                // M7: Flagellocyte tail (terminal)
                modes[7] = CellMode::default_for_role(CellRole::Flagellocyte, 7);
                modes[7].split_mass = 0.88;
                modes[7].split_angle = 0.0;
                modes[7].child_a_mode = 7;
                modes[7].child_b_mode = 7;
                modes[7].keep_adhesion_parent = true;
                modes[7].bond_children = false;
                modes[7].adhesion_stiffness = 0.82;
                modes[7].max_bonds = 3;

                Self {
                    id: 5,
                    preset_id: 4,
                    active_modes: 8,
                    initial_mode: 0,
                    modes,
                }
            }
        }
    }

    #[inline]
    pub fn mode(&self, idx: u8) -> &CellMode {
        &self.modes[(idx as usize).min(MAX_MODES - 1)]
    }

    #[inline]
    pub fn mode_mut(&mut self, idx: u8) -> &mut CellMode {
        &mut self.modes[(idx as usize).min(MAX_MODES - 1)]
    }

    pub fn clamp_all(&mut self) {
        self.active_modes = self.active_modes.clamp(1, MAX_MODES as u8);
        self.initial_mode = self.initial_mode.min((MAX_MODES - 1) as u8);
        for m in &mut self.modes {
            m.clamp_fields();
        }
    }

    /// Background spontaneous genetic drift during mitosis.
    pub fn mutate_natural(&mut self, rng: &mut Rng, scale: f32) {
        let count = (self.active_modes as usize).clamp(1, MAX_MODES);
        for m in &mut self.modes[..count] {
            let rate = (m.mutation_rate * scale).clamp(0.005, 0.45);
            if rng.chance(rate * 1.4) {
                m.t_opt += rng.normal_approx() * 6.5 * rate;
            }
            if rng.chance(rate) {
                m.t_tol += rng.normal_approx() * 3.0 * rate;
            }
            if rng.chance(rate) {
                m.metabolism += rng.normal_approx() * 0.22 * rate;
            }
            if rng.chance(rate) {
                m.light_eff += rng.normal_approx() * 0.20 * rate;
            }
            if rng.chance(rate) {
                m.membrane_armor += rng.normal_approx() * 0.18 * rate;
            }
            if rng.chance(rate) {
                m.immune_resistance += rng.normal_approx() * 0.22 * rate;
            }
            if rng.chance(rate) {
                m.adhesion_stiffness += rng.normal_approx() * 0.20 * rate;
            }
            if rng.chance(rate * 0.85) {
                m.split_angle = wrap_rad(m.split_angle + rng.normal_approx() * 0.35 * rate);
            }
            if rng.chance(rate * 0.75) {
                m.split_mass += rng.normal_approx() * 0.12 * rate;
            }
            m.hue_shift += rng.normal_approx() * 0.04 * rate;
            m.clamp_fields();
        }
    }
}

#[inline]
fn wrap_rad(mut a: f32) -> f32 {
    while a > PI {
        a -= 2.0 * PI;
    }
    while a < -PI {
        a += 2.0 * PI;
    }
    a
}
