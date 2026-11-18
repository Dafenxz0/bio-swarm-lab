// Live Cell X-Ray Neural Network diagram & Population/Evolution Sparkline renderers.

import { ROLE_META } from "./renderer.js";

export const INPUT_LABELS = [
  "Nutr",
  "dTemp",
  "Alarm",
  "FoodP",
  "Enemy",
  "Ally",
  "ATP",
  "Viral",
  "Crowd",
  "Bias",
];

export const OUTPUT_LABELS = [
  "Thrust",
  "Turn",
  "Mitosis",
  "S.Alrm",
  "S.Food",
  "Ab/Adh",
];

export function renderNeuralDiagram(canvas, insp) {
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  const w = canvas.width;
  const h = canvas.height;
  ctx.clearRect(0, 0, w, h);

  ctx.fillStyle = "#060a0f";
  ctx.fillRect(0, 0, w, h);

  if (!insp || insp[0] < 0.5) {
    ctx.fillStyle = "#667b90";
    ctx.font = "11px monospace";
    ctx.textAlign = "center";
    ctx.fillText("Click any cell to X-Ray its live Neural Brain", w / 2, h / 2);
    return;
  }

  const inputs = insp.subarray(24, 34);
  const hidden = insp.subarray(34, 42);
  const outputs = insp.subarray(42, 48);
  const w1 = insp.subarray(48, 128);
  const w2 = insp.subarray(128, 176);

  const xIn = 52;
  const xHid = w * 0.5;
  const xOut = w - 58;

  const inY = (i) => 15 + ((h - 30) * i) / 9;
  const hidY = (i) => 22 + ((h - 44) * i) / 7;
  const outY = (i) => 28 + ((h - 56) * i) / 5;

  // Draw Input -> Hidden weights modulated by input signal
  for (let hi = 0; hi < 8; hi++) {
    const y2 = hidY(hi);
    for (let ii = 0; ii < 10; ii++) {
      const weight = w1[hi * 10 + ii];
      const mag = Math.abs(weight);
      if (mag < 0.14) continue;
      const fire = Math.min(1, Math.abs(inputs[ii]) * 0.8 + 0.2);
      const alpha = Math.min(0.85, mag * 0.35 * fire + 0.06);
      ctx.strokeStyle =
        weight >= 0
          ? `rgba(56, 217, 169, ${alpha.toFixed(2)})`
          : `rgba(255, 107, 107, ${alpha.toFixed(2)})`;
      ctx.lineWidth = Math.min(2.6, 0.5 + mag * 0.85);
      ctx.beginPath();
      ctx.moveTo(xIn, inY(ii));
      ctx.lineTo(xHid, y2);
      ctx.stroke();
    }
  }

  // Draw Hidden -> Output weights modulated by hidden activation
  for (let oi = 0; oi < 6; oi++) {
    const y2 = outY(oi);
    for (let hi = 0; hi < 8; hi++) {
      const weight = w2[oi * 8 + hi];
      const mag = Math.abs(weight);
      if (mag < 0.14) continue;
      const fire = Math.min(1, Math.abs(hidden[hi]) * 0.8 + 0.2);
      const alpha = Math.min(0.9, mag * 0.4 * fire + 0.08);
      ctx.strokeStyle =
        weight >= 0
          ? `rgba(59, 201, 219, ${alpha.toFixed(2)})`
          : `rgba(245, 159, 0, ${alpha.toFixed(2)})`;
      ctx.lineWidth = Math.min(2.8, 0.5 + mag * 0.9);
      ctx.beginPath();
      ctx.moveTo(xHid, hidY(hi));
      ctx.lineTo(xOut, y2);
      ctx.stroke();
    }
  }

  // Draw nodes + labels
  ctx.font = "10px monospace";
  ctx.textBaseline = "middle";

  for (let i = 0; i < 10; i++) {
    const y = inY(i);
    const act = Math.max(-1, Math.min(1, inputs[i]));
    drawNeuron(ctx, xIn, y, 5.2, act);
    ctx.fillStyle = "#9eb0c3";
    ctx.textAlign = "right";
    ctx.fillText(INPUT_LABELS[i], xIn - 9, y);
  }

  for (let i = 0; i < 8; i++) {
    const y = hidY(i);
    drawNeuron(ctx, xHid, y, 6.2, hidden[i]);
  }

  for (let i = 0; i < 6; i++) {
    const y = outY(i);
    const act = outputs[i];
    drawNeuron(ctx, xOut, y, 6.2, act);
    ctx.fillStyle = Math.abs(act) > 0.2 ? "#e7eef6" : "#9eb0c3";
    ctx.textAlign = "left";
    ctx.fillText(OUTPUT_LABELS[i], xOut + 10, y);
  }
}

function drawNeuron(ctx, x, y, r, act) {
  const intensity = Math.min(1, Math.abs(act));
  ctx.fillStyle =
    act >= 0
      ? `rgba(56, 217, 169, ${(0.18 + intensity * 0.82).toFixed(2)})`
      : `rgba(255, 107, 107, ${(0.18 + intensity * 0.82).toFixed(2)})`;
  ctx.beginPath();
  ctx.arc(x, y, r, 0, Math.PI * 2);
  ctx.fill();
  ctx.strokeStyle = intensity > 0.45 ? "#e7eef6" : "#334a62";
  ctx.lineWidth = 1.1;
  ctx.stroke();
}

export function renderPopulationSparkline(canvas, history) {
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  const w = canvas.width;
  const h = canvas.height;
  ctx.clearRect(0, 0, w, h);
  ctx.fillStyle = "#060a0f";
  ctx.fillRect(0, 0, w, h);

  if (history.length < 2) return;

  let maxVal = 20;
  for (const pt of history) {
    for (let i = 0; i < 6; i++) {
      if (pt.roles[i] > maxVal) maxVal = pt.roles[i];
    }
    if (pt.virions > maxVal) maxVal = pt.virions;
  }

  const drawSeries = (getter, color) => {
    ctx.strokeStyle = color;
    ctx.lineWidth = 1.4;
    ctx.beginPath();
    for (let i = 0; i < history.length; i++) {
      const x = (i / (history.length - 1)) * (w - 4) + 2;
      const y = h - 4 - (getter(history[i]) / maxVal) * (h - 10);
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();
  };

  for (let r = 0; r < 6; r++) {
    drawSeries((pt) => pt.roles[r], ROLE_META[r].color);
  }
  drawSeries((pt) => pt.virions, "#f783ac");
}

export function renderEvolutionSparkline(canvas, history) {
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  const w = canvas.width;
  const h = canvas.height;
  ctx.clearRect(0, 0, w, h);
  ctx.fillStyle = "#060a0f";
  ctx.fillRect(0, 0, w, h);

  if (history.length < 2) return;

  const drawNorm = (getter, color) => {
    ctx.strokeStyle = color;
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    for (let i = 0; i < history.length; i++) {
      const x = (i / (history.length - 1)) * (w - 4) + 2;
      const val = Math.max(0, Math.min(1, getter(history[i])));
      const y = h - 4 - val * (h - 10);
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();
  };

  // Mean T_opt mapped from [-20..90C] -> [0..1]
  drawNorm((pt) => (pt.meanTOpt + 20) / 110, "#f59f00");
  // Mean CRISPR Immune Resistance [0..1]
  drawNorm((pt) => pt.meanImmune, "#4dabf7");
  // Mean Adhesion [0..1]
  drawNorm((pt) => pt.meanAdhesion, "#38d9a9");
}
