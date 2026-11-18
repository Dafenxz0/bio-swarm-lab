// Live Conscious Cell Brain X-Ray (12 -> 10 + 4 Memory -> 8), Hebbian plasticity,
// DNA/CRISPR readout, and Population/Adaptation Sparkline renderers.

import { COGNITIVE_META, ROLE_META, STRATEGY_META, syncCanvasDPR } from "./renderer.js";

export const INPUT_LABELS = [
  "Nutr",
  "Light",
  "dTemp",
  "Alarm",
  "FoodP",
  "Enemy",
  "Ally",
  "ATP",
  "Viral",
  "Mutag",
  "SynIn",
  "Crowd",
];

export const HIDDEN_LABELS = ["H0", "H1", "H2", "H3", "H4", "H5", "H6", "H7", "H8", "H9"];
export const MEMORY_LABELS = ["Mem0", "Mem1", "Mem2", "Mem3"];

export const OUTPUT_LABELS = [
  "Thrust",
  "Turn",
  "Mitosis",
  "S.Alrm",
  "S.Food",
  "S.AbTx",
  "SynOut",
  "MyoFlx",
];

function drawNeuron(ctx, x, y, r, act, basePos = "143, 166, 142", baseNeg = "212, 101, 82") {
  const intensity = Math.min(1, Math.abs(act));
  ctx.fillStyle =
    act >= 0
      ? `rgba(${basePos}, ${(0.22 + intensity * 0.78).toFixed(2)})`
      : `rgba(${baseNeg}, ${(0.22 + intensity * 0.78).toFixed(2)})`;
  ctx.beginPath();
  ctx.arc(x, y, r, 0, Math.PI * 2);
  ctx.fill();
  ctx.strokeStyle = intensity > 0.45 ? "#f1f1ef" : "#52544f";
  ctx.lineWidth = 1.1;
  ctx.stroke();
}

export function renderNeuralDiagram(canvas, insp) {
  if (!canvas) return;
  const { dpr, cssW, cssH } = syncCanvasDPR(canvas, 320, 225, 2);
  const ctx = canvas.getContext("2d");
  ctx.save();
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  const w = cssW;
  const h = cssH;

  ctx.fillStyle = "#141414";
  ctx.fillRect(0, 0, w, h);

  if (!insp || insp[0] < 0.5) {
    ctx.fillStyle = "#9a9b96";
    ctx.font = '11px "Geist Mono", monospace';
    ctx.textAlign = "center";
    ctx.fillText("Click any cell to X-Ray its live Neural Brain", w / 2, h / 2);
    ctx.restore();
    return;
  }

  const inputs = insp.subarray(32, 44);
  const hidden = insp.subarray(44, 54);
  const memory = insp.subarray(54, 58);
  const outputs = insp.subarray(58, 66);
  const w1 = insp.subarray(66, 186);
  const wMem = insp.subarray(186, 226);
  const w2 = insp.subarray(226, 306);

  const xIn = 52;
  const xMem = w * 0.34;
  const xHid = w * 0.58;
  const xOut = w - 58;

  const inY = (i) => 14 + ((h - 28) * i) / 11;
  const memY = (i) => 36 + ((h - 72) * i) / 3;
  const hidY = (i) => 18 + ((h - 36) * i) / 9;
  const outY = (i) => 22 + ((h - 44) * i) / 7;

  // Input (12) -> Hidden (10) synapses
  for (let hi = 0; hi < 10; hi++) {
    const y2 = hidY(hi);
    for (let ii = 0; ii < 12; ii++) {
      const weight = w1[hi * 12 + ii];
      const mag = Math.abs(weight);
      if (mag < 0.18) continue;
      const fire = Math.min(1, Math.abs(inputs[ii]) * 0.8 + 0.2);
      const alpha = Math.min(0.8, mag * 0.32 * fire + 0.05);
      ctx.strokeStyle =
        weight >= 0
          ? `rgba(143, 166, 142, ${alpha.toFixed(2)})`
          : `rgba(212, 101, 82, ${alpha.toFixed(2)})`;
      ctx.lineWidth = Math.min(2.2, 0.45 + mag * 0.72);
      ctx.beginPath();
      ctx.moveTo(xIn, inY(ii));
      ctx.lineTo(xHid, y2);
      ctx.stroke();
    }
  }

  // Recurrent Working Memory (4) -> Hidden (10) synapses
  for (let hi = 0; hi < 10; hi++) {
    const y2 = hidY(hi);
    for (let mi = 0; mi < 4; mi++) {
      const weight = wMem[hi * 4 + mi];
      const mag = Math.abs(weight);
      if (mag < 0.16) continue;
      const fire = Math.min(1, Math.abs(memory[mi]) * 0.85 + 0.18);
      const alpha = Math.min(0.82, mag * 0.36 * fire + 0.06);
      ctx.strokeStyle =
        weight >= 0
          ? `rgba(99, 166, 157, ${alpha.toFixed(2)})`
          : `rgba(212, 155, 75, ${alpha.toFixed(2)})`;
      ctx.lineWidth = Math.min(2.2, 0.5 + mag * 0.75);
      ctx.beginPath();
      ctx.moveTo(xMem, memY(mi));
      ctx.lineTo(xHid, y2);
      ctx.stroke();
    }
  }

  // Hidden (10) -> Output (8) synapses
  for (let oi = 0; oi < 8; oi++) {
    const y2 = outY(oi);
    for (let hi = 0; hi < 10; hi++) {
      const weight = w2[oi * 10 + hi];
      const mag = Math.abs(weight);
      if (mag < 0.16) continue;
      const fire = Math.min(1, Math.abs(hidden[hi]) * 0.8 + 0.2);
      const alpha = Math.min(0.88, mag * 0.36 * fire + 0.08);
      ctx.strokeStyle =
        weight >= 0
          ? `rgba(91, 188, 214, ${alpha.toFixed(2)})`
          : `rgba(212, 155, 75, ${alpha.toFixed(2)})`;
      ctx.lineWidth = Math.min(2.4, 0.5 + mag * 0.82);
      ctx.beginPath();
      ctx.moveTo(xHid, hidY(hi));
      ctx.lineTo(xOut, y2);
      ctx.stroke();
    }
  }

  ctx.font = '9.5px "Geist Mono", monospace';
  ctx.textBaseline = "middle";

  for (let i = 0; i < 12; i++) {
    const y = inY(i);
    drawNeuron(ctx, xIn, y, 4.4, Math.max(-1, Math.min(1, inputs[i])));
    ctx.fillStyle = "#9a9b96";
    ctx.textAlign = "right";
    ctx.fillText(INPUT_LABELS[i], xIn - 8, y);
  }

  // Working memory nodes
  for (let i = 0; i < 4; i++) {
    const y = memY(i);
    drawNeuron(ctx, xMem, y, 5.2, memory[i], "99, 166, 157", "212, 155, 75");
    ctx.fillStyle = "#86d4c4";
    ctx.textAlign = "center";
    ctx.fillText(`M${i}`, xMem, y - 10);
  }

  for (let i = 0; i < 10; i++) {
    drawNeuron(ctx, xHid, hidY(i), 5.0, hidden[i]);
  }

  for (let i = 0; i < 8; i++) {
    const y = outY(i);
    const act = outputs[i];
    drawNeuron(ctx, xOut, y, 5.2, act);
    ctx.fillStyle = Math.abs(act) > 0.22 ? "#f1f1ef" : "#9a9b96";
    ctx.textAlign = "left";
    ctx.fillText(OUTPUT_LABELS[i], xOut + 9, y);
  }

  ctx.restore();
}

export function updateInspectorDOM(insp) {
  const idEl = document.getElementById("insp-id");
  const roleEl = document.getElementById("insp-role");
  const modeEl = document.getElementById("insp-mode");
  const orgEl = document.getElementById("insp-org");
  const genEl = document.getElementById("insp-gen");
  const lineageEl = document.getElementById("insp-lineage");
  const thermalEl = document.getElementById("insp-thermal");
  const goalEl = document.getElementById("insp-goal-badge");
  const synEl = document.getElementById("insp-synapse");
  const crisprEl = document.getElementById("insp-crispr");
  const bondsEl = document.getElementById("insp-bonds");

  if (!insp || insp[0] < 0.5) {
    if (idEl) idEl.textContent = "None";
    if (roleEl) {
      roleEl.textContent = "No live selection";
      roleEl.style.color = "";
    }
    if (modeEl) modeEl.textContent = "-";
    if (orgEl) orgEl.textContent = "-";
    if (genEl) genEl.textContent = "-";
    if (lineageEl) lineageEl.textContent = "-";
    if (thermalEl) thermalEl.textContent = "-";
    if (goalEl) goalEl.textContent = "Idle";
    const hpBar = document.getElementById("insp-hp-bar");
    const atpBar = document.getElementById("insp-atp-bar");
    const viralBar = document.getElementById("insp-viral-bar");
    if (hpBar) hpBar.style.width = "0%";
    if (atpBar) atpBar.style.width = "0%";
    if (viralBar) viralBar.style.width = "0%";
    return;
  }

  const roleIdx = Math.max(0, Math.min(11, insp[2] | 0));
  const roleMeta = ROLE_META[roleIdx];
  const cogIdx = Math.max(0, Math.min(5, insp[25] | 0));
  const cogMeta = COGNITIVE_META[cogIdx];
  const infStrat = insp[30] | 0;
  const viralLoad = insp[21];

  if (idEl) idEl.textContent = `#${insp[1] | 0}`;
  if (roleEl) {
    if (viralLoad > 0.05 && infStrat >= 0) {
      const sName = STRATEGY_META[Math.min(2, infStrat)].name;
      roleEl.textContent = `${roleMeta.name} (${sName})`;
    } else {
      roleEl.textContent = roleMeta.name;
    }
    roleEl.style.color = roleMeta.color;
  }
  if (modeEl) modeEl.textContent = `M${insp[3] | 0}`;
  if (orgEl) orgEl.textContent = `#${insp[4] | 0}`;
  if (genEl) genEl.textContent = `${insp[5] | 0}`;
  if (lineageEl) lineageEl.textContent = `Lineage ${insp[6] | 0} · Age ${insp[11] | 0} ticks`;

  if (goalEl) {
    goalEl.textContent = cogMeta.name;
    goalEl.style.borderColor = cogMeta.color;
  }

  if (synEl) {
    const hebbDelta = insp[29];
    synEl.textContent = `ΔHebbian ${hebbDelta.toFixed(3)} · Pulse ${insp[26].toFixed(2)}`;
  }

  const hpPct = Math.max(0, Math.min(100, (insp[7] / Math.max(1, insp[8])) * 100));
  const atpPct = Math.max(0, Math.min(100, (insp[9] / Math.max(1, insp[10])) * 100));
  const viralPct = Math.max(0, Math.min(100, viralLoad * 100));
  const hpVal = document.getElementById("insp-hp-val");
  const hpBar = document.getElementById("insp-hp-bar");
  const atpVal = document.getElementById("insp-atp-val");
  const atpBar = document.getElementById("insp-atp-bar");
  const viralVal = document.getElementById("insp-viral-val");
  const viralBar = document.getElementById("insp-viral-bar");

  if (hpVal) hpVal.textContent = `${insp[7].toFixed(0)}/${insp[8].toFixed(0)}`;
  if (hpBar) hpBar.style.width = `${hpPct.toFixed(1)}%`;
  if (atpVal) atpVal.textContent = `${insp[9].toFixed(0)}/${insp[10].toFixed(0)}`;
  if (atpBar) atpBar.style.width = `${atpPct.toFixed(1)}%`;
  if (viralVal) viralVal.textContent = `${viralPct.toFixed(0)}% (Inc ${(insp[22] * 100).toFixed(0)}%)`;
  if (viralBar) viralBar.style.width = `${viralPct.toFixed(1)}%`;

  const stress = insp[23];
  const stressLabel = stress > 0.05 ? "HEAT-SHOCK" : stress < -0.05 ? "FROSTBITE" : "Optimal";
  if (thermalEl) {
    thermalEl.textContent = `${insp[12].toFixed(1)}°C (Q10 ${insp[24].toFixed(2)}x · ${stressLabel})`;
  }

  // 4 Recurrent Working-Memory bars (memory[0..4] at insp[54..58])
  for (let m = 0; m < 4; m++) {
    const mv = insp[54 + m];
    const vEl = document.getElementById(`mem-val-${m}`);
    const bEl = document.getElementById(`mem-bar-${m}`);
    if (vEl) vEl.textContent = mv.toFixed(2);
    if (bEl) {
      const pct = Math.max(2, Math.min(100, ((mv + 1) * 50)));
      bEl.style.width = `${pct.toFixed(1)}%`;
      bEl.style.background = mv >= 0 ? "var(--p-teal)" : "var(--p-amber)";
    }
  }

  const crisprMask = insp[27] | 0;
  if (crisprEl) {
    crisprEl.textContent = `CRISPR: 0x${crisprMask.toString(16).toUpperCase().padStart(4, "0")}`;
  }
  if (bondsEl) bondsEl.textContent = String(insp[28] | 0);

  const setTxt = (id, txt) => {
    const el = document.getElementById(id);
    if (el) el.textContent = txt;
  };
  setTxt("dna-topt", `${insp[13].toFixed(1)}°C ±${insp[14].toFixed(0)}`);
  setTxt("dna-met", `${insp[15].toFixed(2)}x`);
  setTxt("dna-armor", `${(insp[16] * 100).toFixed(0)}%`);
  setTxt("dna-adh", `${(insp[17] * 100).toFixed(0)}%`);
  setTxt("dna-imm", `${(insp[18] * 100).toFixed(0)}%`);
  setTxt("dna-mut", `${(insp[19] * 100).toFixed(1)}%`);
  setTxt("dna-hebb", insp[20].toFixed(2));
  setTxt("dna-bias", insp[31].toFixed(2));
}

export function renderPopulationSparkline(canvas, history) {
  if (!canvas) return;
  const { dpr, cssW, cssH } = syncCanvasDPR(canvas, 320, 104, 2);
  const ctx = canvas.getContext("2d");
  ctx.save();
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  const w = cssW;
  const h = cssH;

  ctx.fillStyle = "#141414";
  ctx.fillRect(0, 0, w, h);

  if (history.length < 2) {
    ctx.restore();
    return;
  }

  let maxVal = 20;
  for (const pt of history) {
    if (pt.pop > maxVal) maxVal = pt.pop;
    if (pt.virions > maxVal) maxVal = pt.virions;
  }

  // Subtle horizontal reference rule
  ctx.strokeStyle = "rgba(154, 155, 150, 0.12)";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(0, h * 0.5);
  ctx.lineTo(w, h * 0.5);
  ctx.stroke();

  const drawSeries = (getter, color, width = 1.4) => {
    ctx.strokeStyle = color;
    ctx.lineWidth = width;
    ctx.beginPath();
    for (let i = 0; i < history.length; i++) {
      const x = (i / (history.length - 1)) * (w - 6) + 3;
      const y = h - 5 - (getter(history[i]) / maxVal) * (h - 12);
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();
  };

  for (let r = 0; r < ROLE_META.length; r++) {
    drawSeries((pt) => pt.roles[r] || 0, ROLE_META[r].color, 1.0);
  }
  drawSeries((pt) => pt.pop, "#f1f1ef", 1.5);
  drawSeries((pt) => pt.virions, "#d46552", 1.4);
  ctx.restore();
}

export function renderEvolutionSparkline(canvas, history) {
  if (!canvas) return;
  const { dpr, cssW, cssH } = syncCanvasDPR(canvas, 320, 88, 2);
  const ctx = canvas.getContext("2d");
  ctx.save();
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  const w = cssW;
  const h = cssH;

  ctx.fillStyle = "#141414";
  ctx.fillRect(0, 0, w, h);

  if (history.length < 2) {
    ctx.restore();
    return;
  }

  const drawNorm = (getter, color) => {
    ctx.strokeStyle = color;
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    for (let i = 0; i < history.length; i++) {
      const x = (i / (history.length - 1)) * (w - 6) + 3;
      const val = Math.max(0, Math.min(1, getter(history[i])));
      const y = h - 5 - val * (h - 12);
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();
  };

  drawNorm((pt) => (pt.meanTOpt + 25) / 120, "#d49b4b");
  drawNorm((pt) => pt.meanImmune, "#5c9cd9");
  drawNorm((pt) => pt.meanAdhesion, "#8fa68e");
  ctx.restore();
}
