//! Recurrent plastic neural brain (12 inputs -> 10 tanh hidden + 4 working-memory cells -> 8 outputs)
//! with online reward-modulated Hebbian learning and emergent cognitive goal states.

use crate::genome::CellRole;
use crate::rng::Rng;

pub const NN_INPUTS: usize = 12;
pub const NN_HIDDEN: usize = 10;
pub const NN_MEMORY: usize = 4;
pub const NN_OUTPUTS: usize = 8;
pub const NN_W1_LEN: usize = NN_INPUTS * NN_HIDDEN;
pub const NN_W_MEM_LEN: usize = NN_MEMORY * NN_HIDDEN;
pub const NN_W2_LEN: usize = NN_HIDDEN * NN_OUTPUTS;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CognitiveState {
    Forage = 0,
    Hunt = 1,
    Flee = 2,
    DefendHeal = 3,
    Reproduce = 4,
    CooperateSwarm = 5,
}

impl CognitiveState {
    #[inline]
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Forage,
            1 => Self::Hunt,
            2 => Self::Flee,
            3 => Self::DefendHeal,
            4 => Self::Reproduce,
            _ => Self::CooperateSwarm,
        }
    }

    #[inline]
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    #[inline]
    pub fn name(self) -> &'static str {
        match self {
            Self::Forage => "Forage",
            Self::Hunt => "Hunt",
            Self::Flee => "Flee",
            Self::DefendHeal => "Defend/Heal",
            Self::Reproduce => "Reproduce",
            Self::CooperateSwarm => "Cooperate",
        }
    }
}

#[derive(Clone, Debug)]
pub struct NeuralNet {
    pub w1: [f32; NN_W1_LEN],
    pub w_mem: [f32; NN_W_MEM_LEN],
    pub b1: [f32; NN_HIDDEN],
    pub w2: [f32; NN_W2_LEN],
    pub b2: [f32; NN_OUTPUTS],

    pub memory: [f32; NN_MEMORY],
    pub last_inputs: [f32; NN_INPUTS],
    pub last_hidden: [f32; NN_HIDDEN],
    pub last_outputs: [f32; NN_OUTPUTS],
    pub hebbian_delta: f32,
    pub cognitive_state: CognitiveState,
}

impl NeuralNet {
    pub fn for_role(role: CellRole, rng: &mut Rng) -> Self {
        let mut net = Self {
            w1: [0.0; NN_W1_LEN],
            w_mem: [0.0; NN_W_MEM_LEN],
            b1: [0.0; NN_HIDDEN],
            w2: [0.0; NN_W2_LEN],
            b2: [0.0; NN_OUTPUTS],
            memory: [
                0.15 + rng.range_f32(-0.04, 0.04),
                -0.10 + rng.range_f32(-0.04, 0.04),
                0.22 + rng.range_f32(-0.05, 0.05),
                -0.18 + rng.range_f32(-0.05, 0.05),
            ],
            last_inputs: [0.0; NN_INPUTS],
            last_hidden: [0.0; NN_HIDDEN],
            last_outputs: [0.0; NN_OUTPUTS],
            hebbian_delta: 0.0,
            cognitive_state: CognitiveState::Forage,
        };

        for w in &mut net.w1 {
            *w = rng.normal_approx() * 0.20;
        }
        for wm in &mut net.w_mem {
            *wm = rng.normal_approx() * 0.24;
        }
        for w in &mut net.w2 {
            *w = rng.normal_approx() * 0.22;
        }
        for b in &mut net.b1 {
            *b = rng.normal_approx() * 0.12;
        }

        // Input indices:
        // 0: nutrient, 1: light, 2: temp_stress, 3: alarm, 4: food_pheromone,
        // 5: prey_enemy_bearing, 6: ally_bearing, 7: atp_ratio, 8: viral_load,
        // 9: mutagen, 10: incoming_synaptic_signal, 11: crowding
        //
        // Hidden interneurons:
        // H0: Metabolic / Nutrient & Food chemotaxis drive
        net.set_w1(0, 0, 1.35);
        net.set_w1(4, 0, 0.95);
        net.set_w1(11, 0, -0.45);
        // H1: Phototropic / Solar energy drive
        net.set_w1(1, 1, 1.45);
        net.set_w1(7, 1, -0.35);
        // H2: Threat, Pathogen & Alarm detector
        net.set_w1(3, 2, 1.15);
        net.set_w1(5, 2, 1.35);
        net.set_w1(8, 2, 1.25);
        // H3: Mitosis & Morphogenesis readiness (high ATP, low crowding, low viral load)
        net.set_w1(7, 3, 1.85);
        net.set_w1(8, 3, -1.20);
        net.set_w1(11, 3, -0.80);
        // H4: Multicellular Social & Ally cohesion
        net.set_w1(6, 4, 1.25);
        net.set_w1(4, 4, 0.65);
        net.set_w1(10, 4, 0.85);
        // H5: Thermal & Mutagenic avoidance stress detector
        net.set_w1(2, 5, 1.30);
        net.set_w1(9, 5, 1.10);
        // H6: Immune & CRISPR defense drive
        net.set_w1(8, 6, 1.55);
        net.set_w1(3, 6, 1.10);
        net.set_w1(5, 6, 0.85);
        // H7: Synaptic & Pacemaker relay integrator
        net.set_w1(10, 7, 1.65);
        net.set_w1(5, 7, 0.70);
        net.set_w_mem(2, 7, 0.95);
        // H8: Locomotor & Contractile CPG oscillatorA
        net.set_w_mem(2, 8, 1.15);
        net.set_w_mem(3, 8, -0.85);
        net.set_w1(10, 8, 0.90);
        // H9: Locomotor & Contractile CPG oscillatorB
        net.set_w_mem(2, 9, -0.90);
        net.set_w_mem(3, 9, 1.15);
        net.set_w1(0, 9, 0.55);

        // Recurrent working-memory feedback into cognitive interneurons:
        net.set_w_mem(0, 0, 0.55);
        net.set_w_mem(1, 2, 0.65);
        net.set_w_mem(1, 6, 0.50);

        // Output indices:
        // 0: thrust, 1: turn, 2: divide_urge, 3: alarm_out,
        // 4: food_out, 5: antibody_toxin_out, 6: synaptic_signal_out, 7: myocyte_flex

        // Shared baseline wiring:
        net.set_w2(3, 2, 1.55); // H3 -> divide_urge
        net.set_w2(2, 3, 1.10); // H2 -> alarm_out
        net.set_w2(0, 4, 0.85); // H0 -> food_out
        net.set_w2(6, 5, 1.25); // H6 -> antibody_toxin_out
        net.set_w2(7, 6, 1.15); // H7 -> synaptic_signal_out
        net.set_w2(8, 7, 1.35); // H8 -> myocyte_flex
        net.set_w2(9, 7, -0.95); // H9 -> myocyte_flex counter-phase

        match role {
            CellRole::Photocyte => {
                net.set_w2(1, 0, 0.55);
                net.set_w2(0, 0, 0.35);
                net.set_w2(1, 4, 1.25);
                net.set_w2(3, 2, 1.65);
                net.b2[0] = -0.10;
                net.b2[2] = 0.22;
            }
            CellRole::Phagocyte => {
                net.set_w2(2, 0, 1.35);
                net.set_w2(0, 0, 0.75);
                net.set_w2(2, 1, 0.85);
                net.set_w2(2, 5, 0.90);
                net.b2[0] = 0.50;
            }
            CellRole::Flagellocyte => {
                net.set_w2(7, 0, 1.45);
                net.set_w2(8, 0, 0.95);
                net.set_w2(0, 0, 1.10);
                net.set_w2(7, 1, 0.90);
                net.b2[0] = 0.72;
            }
            CellRole::Myocyte => {
                net.set_w2(7, 7, 1.55);
                net.set_w2(8, 7, 1.65);
                net.set_w2(9, 7, -1.35);
                net.set_w2(7, 6, 0.95);
                net.b2[0] = 0.35;
                net.b2[7] = 0.25;
            }
            CellRole::Neurocyte => {
                net.set_w2(2, 6, 1.45);
                net.set_w2(0, 6, 1.15);
                net.set_w2(7, 6, 1.65);
                net.set_w2(8, 6, 1.20);
                net.b2[6] = 0.40;
            }
            CellRole::Senseocyte => {
                net.set_w2(2, 6, 1.60);
                net.set_w2(0, 6, 1.35);
                net.set_w2(1, 6, 1.10);
                net.set_w2(2, 3, 1.45);
                net.b2[6] = 0.35;
            }
            CellRole::Immunocyte => {
                net.set_w2(6, 0, 1.30);
                net.set_w2(2, 0, 1.15);
                net.set_w2(6, 5, 1.85);
                net.set_w2(2, 3, 1.35);
                net.b2[0] = 0.35;
                net.b2[5] = 0.30;
            }
            CellRole::Keratinocyte => {
                net.set_w2(4, 0, 0.40);
                net.set_w2(6, 5, 0.85);
                net.b2[0] = -0.25;
            }
            CellRole::Secreocyte => {
                net.set_w2(0, 4, 1.45);
                net.set_w2(2, 3, 1.45);
                net.set_w2(6, 5, 1.75);
                net.b2[4] = 0.35;
                net.b2[5] = 0.45;
            }
            CellRole::Vascularocyte => {
                net.set_w2(4, 6, 1.25);
                net.set_w2(0, 4, 1.10);
                net.b2[0] = 0.05;
                net.b2[2] = 0.25;
            }
            CellRole::Stemocyte => {
                net.set_w2(3, 2, 2.10);
                net.set_w2(0, 2, 0.85);
                net.set_w2(1, 2, 0.75);
                net.b2[2] = 0.48;
                net.b2[0] = 0.15;
            }
            CellRole::Scavenger => {
                net.set_w2(0, 0, 1.25);
                net.set_w2(5, 0, 0.90);
                net.set_w2(6, 5, 1.30);
                net.set_w2(0, 4, 1.15);
                net.b2[0] = 0.42;
            }
        }
        net
    }

    #[inline]
    pub fn set_w1(&mut self, inp: usize, hid: usize, val: f32) {
        if inp < NN_INPUTS && hid < NN_HIDDEN {
            self.w1[hid * NN_INPUTS + inp] = val;
        }
    }

    #[inline]
    pub fn set_w_mem(&mut self, mem: usize, hid: usize, val: f32) {
        if mem < NN_MEMORY && hid < NN_HIDDEN {
            self.w_mem[hid * NN_MEMORY + mem] = val;
        }
    }

    #[inline]
    pub fn set_w2(&mut self, hid: usize, out: usize, val: f32) {
        if hid < NN_HIDDEN && out < NN_OUTPUTS {
            self.w2[out * NN_HIDDEN + hid] = val;
        }
    }

    pub fn forward(&mut self, inputs: [f32; NN_INPUTS], activity_bias: f32) -> [f32; NN_OUTPUTS] {
        self.last_inputs = inputs;

        for h in 0..NN_HIDDEN {
            let mut sum = self.b1[h] + activity_bias * 0.18;
            let row_in = h * NN_INPUTS;
            for (i, &inp) in inputs.iter().enumerate() {
                sum += self.w1[row_in + i] * inp;
            }
            let row_m = h * NN_MEMORY;
            for (m, &mem_v) in self.memory.iter().enumerate() {
                sum += self.w_mem[row_m + m] * mem_v;
            }
            self.last_hidden[h] = sum.tanh();
        }

        // Update 4 recurrent working-memory cells (leaky integrator + rhythmic coupled oscillator):
        let d0 = (self.last_hidden[0] + 0.6 * self.last_hidden[1] + 0.08).tanh();
        let d1 = (self.last_hidden[2] + 0.6 * self.last_hidden[5] - 0.06).tanh();
        let osc_in = self.last_hidden[7] + 0.55 * self.last_hidden[8] - 0.45 * self.memory[3];
        let d2 = (osc_in + 0.12).tanh();
        let m0 = 0.76 * self.memory[0] + 0.24 * d0;
        let m1 = 0.78 * self.memory[1] + 0.22 * d1;
        let m2 = 0.70 * self.memory[2] + 0.30 * d2;
        let d3 = (0.60 * m2 + self.last_hidden[9] + self.last_hidden[4]).tanh();
        let m3 = 0.72 * self.memory[3] + 0.28 * d3;
        self.memory = [
            m0.clamp(-1.0, 1.0),
            m1.clamp(-1.0, 1.0),
            m2.clamp(-1.0, 1.0),
            m3.clamp(-1.0, 1.0),
        ];

        for o in 0..NN_OUTPUTS {
            let mut sum = self.b2[o];
            let row = o * NN_HIDDEN;
            for (h, &hid) in self.last_hidden.iter().enumerate() {
                sum += self.w2[row + h] * hid;
            }
            self.last_outputs[o] = sum.tanh();
        }

        self.hebbian_delta *= 0.94;
        self.last_outputs
    }

    /// Online reward-modulated Hebbian / Oja synaptic plasticity.
    pub fn adapt(&mut self, reward: f32, hebbian_rate: f32) {
        let lr = (hebbian_rate * 0.08).clamp(0.0, 0.06);
        let r = reward.clamp(-1.5, 1.5);
        if lr <= 1e-6 || r.abs() <= 1e-4 {
            return;
        }
        let mut total_dw = 0.0_f32;
        for o in 0..NN_OUTPUTS {
            let out = self.last_outputs[o];
            let row = o * NN_HIDDEN;
            for h in 0..NN_HIDDEN {
                let hid = self.last_hidden[h];
                let idx = row + h;
                let dw = lr * r * (hid * out - 0.08 * out * out * self.w2[idx]);
                self.w2[idx] = (self.w2[idx] + dw).clamp(-4.0, 4.0);
                total_dw += dw.abs();
            }
        }
        let lr_in = lr * 0.45;
        for h in 0..NN_HIDDEN {
            let hid = self.last_hidden[h];
            let row = h * NN_INPUTS;
            for i in 0..NN_INPUTS {
                let inp = self.last_inputs[i];
                let idx = row + i;
                let dw = lr_in * r * (inp * hid - 0.08 * hid * hid * self.w1[idx]);
                self.w1[idx] = (self.w1[idx] + dw).clamp(-4.0, 4.0);
                total_dw += dw.abs();
            }
        }
        let mean_dw = (total_dw / (NN_W2_LEN as f32)) * 18.0;
        self.hebbian_delta = (self.hebbian_delta * 0.75 + mean_dw).clamp(0.0, 1.0);
    }

    /// Classify the cell's emergent cognitive goal from internal state and neural activations.
    pub fn classify_cognitive_state(
        &mut self,
        role: CellRole,
        atp_ratio: f32,
        viral_load: f32,
        bonded_count: u8,
    ) -> CognitiveState {
        let threat_in = self.last_inputs[5].abs().max(self.last_inputs[3]);
        let temp_mut_stress = self.last_inputs[2].abs().max(self.last_inputs[9]);
        let divide_out = self.last_outputs[2];
        let defense_out = self.last_outputs[5];
        let synaptic_out = self.last_outputs[6].abs();
        let is_combat_role = role == CellRole::Phagocyte || role == CellRole::Immunocyte;
        let is_tissue_role = role == CellRole::Neurocyte
            || role == CellRole::Vascularocyte
            || role == CellRole::Myocyte;

        let is_defending = viral_load > 0.15
            || (role == CellRole::Immunocyte && (threat_in > 0.25 || defense_out > 0.35));
        let is_hunting =
            role == CellRole::Phagocyte && (threat_in > 0.18 || self.last_outputs[0] > 0.35);

        let state = if is_defending {
            CognitiveState::DefendHeal
        } else if is_hunting {
            CognitiveState::Hunt
        } else if (threat_in > 0.48 && !is_combat_role) || temp_mut_stress > 0.75 {
            CognitiveState::Flee
        } else if atp_ratio > 0.68 && divide_out > 0.25 {
            CognitiveState::Reproduce
        } else if bonded_count > 0 && (synaptic_out > 0.30 || is_tissue_role) {
            CognitiveState::CooperateSwarm
        } else {
            CognitiveState::Forage
        };

        self.cognitive_state = state;
        state
    }

    pub fn mutate(&mut self, rng: &mut Rng, rate: f32, strength: f32) {
        let p = rate.clamp(0.01, 0.65);
        let s = strength.clamp(0.02, 0.90);
        for w in &mut self.w1 {
            if rng.chance(p) {
                *w = (*w + rng.normal_approx() * s).clamp(-4.0, 4.0);
            }
        }
        for wm in &mut self.w_mem {
            if rng.chance(p) {
                *wm = (*wm + rng.normal_approx() * s * 0.8).clamp(-3.5, 3.5);
            }
        }
        for b in &mut self.b1 {
            if rng.chance(p) {
                *b = (*b + rng.normal_approx() * s * 0.6).clamp(-3.0, 3.0);
            }
        }
        for w in &mut self.w2 {
            if rng.chance(p) {
                *w = (*w + rng.normal_approx() * s).clamp(-4.0, 4.0);
            }
        }
        for b in &mut self.b2 {
            if rng.chance(p) {
                *b = (*b + rng.normal_approx() * s * 0.6).clamp(-3.0, 3.0);
            }
        }
    }
}
