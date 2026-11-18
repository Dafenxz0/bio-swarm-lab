//! Per-cell evolvable Feedforward Neural Network (10 inputs -> 8 tanh hidden -> 6 outputs).

use crate::genome::CellRole;
use crate::rng::Rng;

pub const NN_INPUTS: usize = 10;
pub const NN_HIDDEN: usize = 8;
pub const NN_OUTPUTS: usize = 6;
pub const NN_W1_LEN: usize = NN_INPUTS * NN_HIDDEN;
pub const NN_W2_LEN: usize = NN_HIDDEN * NN_OUTPUTS;

#[derive(Clone, Debug)]
pub struct NeuralNet {
    pub w1: [f32; NN_W1_LEN],
    pub b1: [f32; NN_HIDDEN],
    pub w2: [f32; NN_W2_LEN],
    pub b2: [f32; NN_OUTPUTS],

    pub last_inputs: [f32; NN_INPUTS],
    pub last_hidden: [f32; NN_HIDDEN],
    pub last_outputs: [f32; NN_OUTPUTS],
}

impl NeuralNet {
    pub fn for_role(role: CellRole, rng: &mut Rng) -> Self {
        let mut net = Self {
            w1: [0.0; NN_W1_LEN],
            b1: [0.0; NN_HIDDEN],
            w2: [0.0; NN_W2_LEN],
            b2: [0.0; NN_OUTPUTS],
            last_inputs: [0.0; NN_INPUTS],
            last_hidden: [0.0; NN_HIDDEN],
            last_outputs: [0.0; NN_OUTPUTS],
        };

        // Small random initial wiring across all synapses so evolution has rich raw material.
        for w in &mut net.w1 {
            *w = rng.normal_approx() * 0.22;
        }
        for w in &mut net.w2 {
            *w = rng.normal_approx() * 0.25;
        }
        for b in &mut net.b1 {
            *b = rng.normal_approx() * 0.12;
        }

        // Hidden neurons act as interpretable biological sensory interneurons:
        // H0: Nutrient/food drive
        net.set_w1(0, 0, 1.3);
        net.set_w1(3, 0, 0.9);
        net.set_w1(8, 0, -0.5);
        // H1: Thermal discomfort drive
        net.set_w1(1, 1, 1.2);
        // H2: Threat / viral danger detector
        net.set_w1(2, 2, 1.1);
        net.set_w1(4, 2, 1.4);
        net.set_w1(7, 2, 1.0);
        // H3: Mitosis readiness (high ATP, low crowding, low infection)
        net.set_w1(6, 3, 1.8);
        net.set_w1(8, 3, -0.9);
        net.set_w1(7, 3, -1.2);
        // H4: Social / ally cohesion
        net.set_w1(5, 4, 1.2);
        net.set_w1(3, 4, 0.6);

        // Role-specific instinctive efferent wiring:
        match role {
            CellRole::Producer => {
                // Slow drift, strong mitosis when ATP high, gentle food pheromone when rich.
                net.set_w2(0, 0, 0.4);
                net.set_w2(1, 0, 0.7);
                net.set_w2(3, 2, 1.4);
                net.set_w2(0, 4, 0.9);
                net.set_w2(2, 3, 0.7);
                net.set_w2(4, 5, 0.6);
                net.b2[0] = -0.15;
                net.b2[2] = 0.1;
            }
            CellRole::Protector => {
                // Immunocyte: rallies on alarm/threat, secretes antibodies & alarm pheromone.
                net.set_w2(2, 0, 1.3);
                net.set_w2(0, 0, 0.5);
                net.set_w2(2, 3, 1.4);
                net.set_w2(2, 5, 1.6);
                net.set_w2(3, 2, 1.1);
                net.b2[0] = 0.25;
                net.b2[5] = 0.15;
            }
            CellRole::Predator => {
                // Phagocyte: aggressive swimming, turns toward prey/threat, secretes rally.
                net.set_w2(2, 0, 1.1);
                net.set_w2(0, 0, 0.7);
                net.set_w2(2, 1, 0.6);
                net.set_w2(3, 2, 1.1);
                net.set_w2(0, 4, 0.6);
                net.b2[0] = 0.45;
            }
            CellRole::Swimmer => {
                // Myocyte: fast chemotactic swimming, strong alarm & food relay.
                net.set_w2(0, 0, 1.2);
                net.set_w2(1, 1, 0.8);
                net.set_w2(0, 4, 1.2);
                net.set_w2(2, 3, 1.2);
                net.set_w2(3, 2, 1.2);
                net.b2[0] = 0.65;
            }
            CellRole::Replicator => {
                // Stem cell: prioritizes mitosis and adhesion tissue growth.
                net.set_w2(3, 2, 1.9);
                net.set_w2(0, 0, 0.6);
                net.set_w2(4, 5, 0.9);
                net.b2[2] = 0.35;
                net.b2[0] = 0.05;
            }
            CellRole::Infected => {
                net.b2[3] = 0.6;
            }
        }
        net
    }

    #[inline]
    fn set_w1(&mut self, inp: usize, hid: usize, val: f32) {
        self.w1[hid * NN_INPUTS + inp] = val;
    }

    #[inline]
    fn set_w2(&mut self, hid: usize, out: usize, val: f32) {
        self.w2[out * NN_HIDDEN + hid] = val;
    }

    pub fn forward(&mut self, inputs: [f32; NN_INPUTS], activity_bias: f32) -> [f32; NN_OUTPUTS] {
        self.last_inputs = inputs;

        for h in 0..NN_HIDDEN {
            let mut sum = self.b1[h] + activity_bias * 0.2;
            let row = h * NN_INPUTS;
            for (i, &inp) in inputs.iter().enumerate() {
                sum += self.w1[row + i] * inp;
            }
            self.last_hidden[h] = sum.tanh();
        }

        for o in 0..NN_OUTPUTS {
            let mut sum = self.b2[o];
            let row = o * NN_HIDDEN;
            for (h, &hid) in self.last_hidden.iter().enumerate() {
                sum += self.w2[row + h] * hid;
            }
            self.last_outputs[o] = sum.tanh();
        }

        self.last_outputs
    }

    pub fn mutate(&mut self, rng: &mut Rng, rate: f32, strength: f32) {
        let p = rate.clamp(0.01, 0.6);
        let s = strength.clamp(0.02, 0.8);
        for w in &mut self.w1 {
            if rng.chance(p) {
                *w = (*w + rng.normal_approx() * s).clamp(-4.0, 4.0);
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
