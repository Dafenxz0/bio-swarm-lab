// High-DPI Bio-Microscopy Canvas2D renderer:
// 7 environmental diffusion fields, chamber walls, vascular/synaptic bonds,
// 12 differentiated cell morphologies, 3 viral strategies, cognitive rings, and camera pan/zoom.

export const ROLE_META = [
  { id: 0, name: "Photocyte", short: "Solar ATP", color: "#63b884", glow: "rgba(99, 184, 132, 0.28)" },
  { id: 1, name: "Phagocyte", short: "Predator", color: "#d96254", glow: "rgba(217, 98, 84, 0.30)" },
  { id: 2, name: "Flagellocyte", short: "Motile", color: "#e0b64a", glow: "rgba(224, 182, 74, 0.28)" },
  { id: 3, name: "Myocyte", short: "Muscle", color: "#d98a4a", glow: "rgba(217, 138, 74, 0.28)" },
  { id: 4, name: "Neurocyte", short: "Neuron", color: "#5bbcd6", glow: "rgba(91, 188, 214, 0.34)" },
  { id: 5, name: "Senseocyte", short: "Sensor", color: "#86d4c4", glow: "rgba(134, 212, 196, 0.30)" },
  { id: 6, name: "Immunocyte", short: "Immune", color: "#5c9cd9", glow: "rgba(92, 156, 217, 0.32)" },
  { id: 7, name: "Keratinocyte", short: "Armor", color: "#a8a69a", glow: "rgba(168, 166, 154, 0.26)" },
  { id: 8, name: "Secreocyte", short: "Gland", color: "#9db863", glow: "rgba(157, 184, 99, 0.28)" },
  { id: 9, name: "Vascularocyte", short: "Vascular", color: "#e07b6c", glow: "rgba(224, 123, 108, 0.30)" },
  { id: 10, name: "Stemocyte", short: "Stem", color: "#70c7b4", glow: "rgba(112, 199, 180, 0.30)" },
  { id: 11, name: "Scavenger", short: "Scavenger", color: "#8fa68e", glow: "rgba(143, 166, 142, 0.28)" },
];

export const COGNITIVE_META = [
  { id: 0, name: "Forage", color: "#8fa68e" },
  { id: 1, name: "Hunt", color: "#d96254" },
  { id: 2, name: "Flee", color: "#e0b64a" },
  { id: 3, name: "Defend/Heal", color: "#5c9cd9" },
  { id: 4, name: "Reproduce", color: "#63b884" },
  { id: 5, name: "Cooperate", color: "#5bbcd6" },
];

export const STRATEGY_META = [
  { id: 0, name: "Lytic" },
  { id: 1, name: "Lysogenic" },
  { id: 2, name: "Oncogenic Mutator" },
];

export const OVERLAY_META = {
  biology: {
    label: "Biology",
    low: "Cold / low nutrient",
    high: "Warm / illuminated",
    gradient: "linear-gradient(90deg, #182220, #3d5949, #8fa68e)",
  },
  temperature: {
    label: "Temperature",
    low: "-30°C Cryo",
    high: "95°C Hydrothermal",
    gradient: "linear-gradient(90deg, #2d72b8, #242929, #d49b4b, #d45242)",
  },
  light: {
    label: "Light Field",
    low: "Deep shade (0.0)",
    high: "Peak solar (1.4+)",
    gradient: "linear-gradient(90deg, #121413, #5b5536, #e8cf78)",
  },
  nutrients: {
    label: "Nutrients",
    low: "Depleted (0.0)",
    high: "Glucose-rich (1.3+)",
    gradient: "linear-gradient(90deg, #121413, #2c5c43, #63b884)",
  },
  pheromones: {
    label: "Pheromones",
    low: "Food/Rally (Teal)",
    high: "Alarm/Threat (Coral)",
    gradient: "linear-gradient(90deg, #121413, #4aa6a1, #d96254)",
  },
  toxin: {
    label: "Toxin / Ab",
    low: "Clear (0.0)",
    high: "Antibody shield (1.2+)",
    gradient: "linear-gradient(90deg, #121413, #2f5b78, #5c9cd9)",
  },
  mutagen: {
    label: "Mutagen Radiation",
    low: "Baseline (0.0)",
    high: "High flux (1.2+)",
    gradient: "linear-gradient(90deg, #121413, #5d7327, #c8e05a)",
  },
};

const BRUSH_COLORS = {
  heat: "rgba(212, 155, 75, 0.90)",
  cryo: "rgba(92, 156, 217, 0.90)",
  light: "rgba(232, 207, 120, 0.90)",
  nutrient: "rgba(99, 184, 132, 0.90)",
  toxin: "rgba(99, 166, 157, 0.90)",
  mutagen: "rgba(200, 224, 90, 0.90)",
  virus: "rgba(217, 98, 84, 0.90)",
  wall: "rgba(241, 241, 239, 0.85)",
  erase: "rgba(224, 182, 74, 0.85)",
  spawn: "rgba(143, 166, 142, 0.90)",
};

/** Clamp normalized hue (0..1) within the warm-to-cyan microscopy range (0..225 deg). */
export function safeBioHueDeg(hue01) {
  const deg = (((hue01 % 1) + 1) % 1) * 360;
  if (deg >= 230 && deg <= 335) {
    return deg < 282 ? 205 : 18;
  }
  return deg;
}

export function createFieldSurface(gridW, gridH) {
  if (typeof document === "undefined") {
    return null;
  }
  const off = document.createElement("canvas");
  off.width = gridW;
  off.height = gridH;
  const octx = off.getContext("2d");
  const img = octx.createImageData(gridW, gridH);
  return { off, octx, img, gridW, gridH };
}

export function renderFieldSurface(surface, overlay, fields) {
  if (!surface) return;
  const { octx, img, gridW, gridH } = surface;
  const data = img.data;
  const len = gridW * gridH;
  const { temp, light, nutrients, alarm, food, toxin, mutagen, walls } = fields;

  for (let i = 0; i < len; i++) {
    const p = i * 4;
    if (walls && walls[i] !== 0) {
      data[p] = 36;
      data[p + 1] = 38;
      data[p + 2] = 35;
      data[p + 3] = 255;
      continue;
    }

    let r = 18;
    let g = 20;
    let b = 19;

    if (overlay === "temperature") {
      const t = temp[i];
      if (t < 24) {
        const k = Math.min(1, (24 - t) / 54);
        r = 24 - k * 8;
        g = 28 + k * 76;
        b = 34 + k * 165;
      } else {
        const k = Math.min(1, (t - 24) / 68);
        r = 24 + k * 200;
        g = 26 + Math.sin(k * Math.PI) * 110;
        b = 30 - k * 14;
      }
    } else if (overlay === "light") {
      const lt = Math.min(1, (light ? light[i] : 0.6) / 1.35);
      r = 16 + lt * 212;
      g = 18 + lt * 188;
      b = 18 + lt * 94;
    } else if (overlay === "nutrients") {
      const n = Math.min(1, nutrients[i] / 1.15);
      r = 16 + n * 48;
      g = 20 + n * 155;
      b = 18 + n * 96;
    } else if (overlay === "pheromones") {
      const al = Math.min(1, alarm[i] * 1.65);
      const fd = Math.min(1, food[i] * 1.65);
      r = 18 + al * 195 + fd * 24;
      g = 20 + fd * 142 + al * 46;
      b = 20 + fd * 136 + al * 36;
    } else if (overlay === "toxin") {
      const tx = Math.min(1, toxin[i] * 1.55);
      r = 16 + tx * 58;
      g = 20 + tx * 132;
      b = 22 + tx * 190;
    } else if (overlay === "mutagen") {
      const mt = Math.min(1, (mutagen ? mutagen[i] : 0) * 1.45);
      r = 17 + mt * 174;
      g = 20 + mt * 200;
      b = 18 + mt * 64;
    } else {
      // Normal Biology: subtle translucent tint from temperature, nutrients, light, toxin, mutagen
      const n = Math.min(1, nutrients[i] / 1.2);
      const lt = light ? Math.min(1, light[i] / 1.4) : 0.4;
      const t = temp[i];
      const hot = Math.max(0, (t - 36) / 60);
      const cold = Math.max(0, (14 - t) / 45);
      const tx = Math.min(1, toxin[i]);
      const mt = mutagen ? Math.min(1, mutagen[i]) : 0;
      const al = Math.min(1, alarm[i]);
      r = 17 + lt * 16 + hot * 72 + al * 54 + mt * 38;
      g = 20 + lt * 14 + n * 24 + mt * 46 + tx * 24 + hot * 18;
      b = 19 + lt * 6 + n * 16 + cold * 78 + tx * 48;
    }

    data[p] = Math.max(0, Math.min(255, r | 0));
    data[p + 1] = Math.max(0, Math.min(255, g | 0));
    data[p + 2] = Math.max(0, Math.min(255, b | 0));
    data[p + 3] = 255;
  }

  octx.putImageData(img, 0, 0);
}

/** Sync backing pixel resolution of a canvas to CSS box * devicePixelRatio. */
export function syncCanvasDPR(canvas, fallbackW = 1200, fallbackH = 800, maxDpr = 2.5) {
  if (!canvas || typeof window === "undefined") {
    return { dpr: 1, cssW: fallbackW, cssH: fallbackH, pixelW: fallbackW, pixelH: fallbackH };
  }
  const dpr = Math.min(window.devicePixelRatio || 1, maxDpr);
  const rect = canvas.getBoundingClientRect();
  const cssW = rect.width > 16 ? rect.width : Number(canvas.getAttribute("width")) || fallbackW;
  const cssH = rect.height > 16 ? rect.height : Number(canvas.getAttribute("height")) || fallbackH;
  const pixelW = Math.max(16, Math.round(cssW * dpr));
  const pixelH = Math.max(16, Math.round(cssH * dpr));
  if (canvas.width !== pixelW || canvas.height !== pixelH) {
    canvas.width = pixelW;
    canvas.height = pixelH;
  }
  return { dpr, cssW, cssH, pixelW, pixelH };
}

/** Draw one of the 12 biological cell morphologies (shared by live world & blueprint preview). */
export function drawCellGlyph(ctx, x, y, r, ang, roleIdx, opts = {}) {
  const {
    tick = 0,
    id = 1,
    adhesion = 0,
    viralLoad = 0,
    tempStress = 0,
    cognitiveState = 0,
    synapticPulse = 0,
    signals = 0,
    hueShift = 0,
    highlight = false,
  } = opts;

  const meta = ROLE_META[Math.max(0, Math.min(11, roleIdx | 0))];
  const cos = Math.cos(ang);
  const sin = Math.sin(ang);

  // Subtle cognitive goal indicator outer ring
  if (cognitiveState > 0) {
    const cog = COGNITIVE_META[Math.min(5, cognitiveState | 0)];
    ctx.strokeStyle = cog.color;
    ctx.globalAlpha = 0.42;
    ctx.lineWidth = 1.25;
    ctx.setLineDash([r * 0.55, r * 0.35]);
    ctx.beginPath();
    ctx.arc(x, y, r * 1.44, tick * 0.03 + id * 0.4, tick * 0.03 + id * 0.4 + Math.PI * 1.55);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.globalAlpha = 1;
  }

  // Signal halos (Antibody, Alarm, Synapse, Mutation flash)
  if (signals & 32) {
    ctx.strokeStyle = "rgba(200, 224, 90, 0.72)";
    ctx.lineWidth = 1.8;
    ctx.beginPath();
    ctx.arc(x, y, r * 1.68, 0, Math.PI * 2);
    ctx.stroke();
  } else if (signals & 16 || Math.abs(synapticPulse) > 0.32) {
    ctx.strokeStyle = "rgba(91, 188, 214, 0.52)";
    ctx.lineWidth = 1.4;
    ctx.beginPath();
    ctx.arc(x, y, r * 1.6, 0, Math.PI * 2);
    ctx.stroke();
  } else if (signals & 4) {
    ctx.strokeStyle = "rgba(92, 156, 217, 0.5)";
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.arc(x, y, r * 1.62, 0, Math.PI * 2);
    ctx.stroke();
  } else if (signals & 1) {
    ctx.strokeStyle = "rgba(217, 98, 84, 0.52)";
    ctx.lineWidth = 1.3;
    ctx.beginPath();
    ctx.arc(x, y, r * 1.52, 0, Math.PI * 2);
    ctx.stroke();
  }

  // Thermal stress aura (heat-shock coral-amber vs frostbite azure)
  if (Math.abs(tempStress) > 0.05) {
    ctx.strokeStyle =
      tempStress > 0
        ? `rgba(212, 95, 66, ${Math.min(0.85, tempStress * 0.65).toFixed(2)})`
        : `rgba(92, 156, 217, ${Math.min(0.85, -tempStress * 0.65).toFixed(2)})`;
    ctx.lineWidth = 1.9;
    ctx.beginPath();
    ctx.arc(x, y, r * 1.28, 0, Math.PI * 2);
    ctx.stroke();
  }

  // Pre-body appendages: Flagellocyte tail, Senseocyte cone rays, Scavenger cilia, Immunocyte Y-spikes
  if (roleIdx === 2) {
    // Flagellocyte: multi-segment undulating whip tail
    ctx.strokeStyle = "rgba(224, 182, 74, 0.85)";
    ctx.lineWidth = Math.max(1.2, r * 0.24);
    ctx.lineCap = "round";
    ctx.beginPath();
    const bx = x - cos * r * 0.85;
    const by = y - sin * r * 0.85;
    ctx.moveTo(bx, by);
    const segs = 4;
    for (let s = 1; s <= segs; s++) {
      const frac = s / segs;
      const wag = Math.sin(tick * 0.45 + id - frac * 2.4) * 0.48 * frac;
      const px = bx - Math.cos(ang + wag) * (r * 2.1 * frac);
      const py = by - Math.sin(ang + wag) * (r * 2.1 * frac);
      ctx.lineTo(px, py);
    }
    ctx.stroke();
  } else if (roleIdx === 5) {
    // Senseocyte: optical / chemosensory forward reception cone + antennae
    ctx.fillStyle = "rgba(134, 212, 196, 0.13)";
    ctx.beginPath();
    ctx.moveTo(x + cos * r * 0.5, y + sin * r * 0.5);
    ctx.arc(x, y, r * 3.1, ang - 0.38, ang + 0.38);
    ctx.closePath();
    ctx.fill();

    ctx.strokeStyle = "rgba(134, 212, 196, 0.65)";
    ctx.lineWidth = 1.0;
    for (const side of [-0.34, 0.34]) {
      ctx.beginPath();
      ctx.moveTo(x + Math.cos(ang + side) * r * 0.85, y + Math.sin(ang + side) * r * 0.85);
      ctx.lineTo(x + Math.cos(ang + side) * r * 1.65, y + Math.sin(ang + side) * r * 1.65);
      ctx.stroke();
    }
  } else if (roleIdx === 6) {
    // Immunocyte: antibody Y-spikes
    ctx.strokeStyle = "rgba(135, 192, 245, 0.82)";
    ctx.lineWidth = 1.1;
    for (let k = 0; k < 6; k++) {
      const sa = ang + (k * Math.PI) / 3;
      const sx = x + Math.cos(sa) * r * 0.96;
      const sy = y + Math.sin(sa) * r * 0.96;
      const mx = x + Math.cos(sa) * r * 1.32;
      const my = y + Math.sin(sa) * r * 1.32;
      ctx.beginPath();
      ctx.moveTo(sx, sy);
      ctx.lineTo(mx, my);
      ctx.lineTo(mx + Math.cos(sa + 0.55) * r * 0.3, my + Math.sin(sa + 0.55) * r * 0.3);
      ctx.moveTo(mx, my);
      ctx.lineTo(mx + Math.cos(sa - 0.55) * r * 0.3, my + Math.sin(sa - 0.55) * r * 0.3);
      ctx.stroke();
    }
  } else if (roleIdx === 11) {
    // Scavenger: rhythmic radial cilia fringe
    ctx.strokeStyle = "rgba(143, 166, 142, 0.78)";
    ctx.lineWidth = 1.0;
    ctx.beginPath();
    for (let k = 0; k < 10; k++) {
      const ca = ang + (k * Math.PI * 2) / 10;
      const sway = Math.sin(tick * 0.35 + k + id) * 0.22;
      ctx.moveTo(x + Math.cos(ca) * r * 0.95, y + Math.sin(ca) * r * 0.95);
      ctx.lineTo(x + Math.cos(ca + sway) * r * 1.42, y + Math.sin(ca + sway) * r * 1.42);
    }
    ctx.stroke();
  }

  // Outer membrane glow
  ctx.fillStyle = meta.glow;
  ctx.beginPath();
  ctx.arc(x, y, r * 1.16, 0, Math.PI * 2);
  ctx.fill();

  // Main cell body silhouette
  ctx.fillStyle = meta.color;
  ctx.beginPath();
  if (roleIdx === 1) {
    // Phagocyte: open predator jaws cleft
    const jaw = 0.44 + Math.sin(tick * 0.25 + id) * 0.08;
    ctx.arc(x, y, r, ang + jaw, ang + Math.PI * 2 - jaw);
    ctx.lineTo(x + cos * r * 0.24, y + sin * r * 0.24);
    ctx.closePath();
  } else if (roleIdx === 3) {
    // Myocyte: contractile spindle ellipse
    const contract = 1 - Math.sin(tick * 0.32 + id) * 0.14;
    ctx.ellipse(x, y, r * 1.24 * contract, r * (0.78 / contract), ang, 0, Math.PI * 2);
  } else if (roleIdx === 7) {
    // Keratinocyte: hexagonal armor plate
    for (let v = 0; v < 6; v++) {
      const va = ang + (v * Math.PI) / 3;
      const vx = x + Math.cos(va) * r * 1.04;
      const vy = y + Math.sin(va) * r * 1.04;
      if (v === 0) ctx.moveTo(vx, vy);
      else ctx.lineTo(vx, vy);
    }
    ctx.closePath();
  } else {
    ctx.arc(x, y, r, 0, Math.PI * 2);
  }
  ctx.fill();

  // Optional subtle hue-shift ring for mutated/lineage color differentiation
  if (Math.abs(hueShift) > 0.03) {
    ctx.strokeStyle = `hsla(${safeBioHueDeg(0.35 + hueShift).toFixed(0)}, 55%, 64%, 0.48)`;
    ctx.lineWidth = 1.2;
    ctx.stroke();
  }

  // Adhesion rim marker
  if (adhesion > 0.45) {
    ctx.strokeStyle = "rgba(182, 212, 180, 0.72)";
    ctx.lineWidth = 1.25;
    ctx.stroke();
  }

  // Internal organelle details per cell role
  if (roleIdx === 0) {
    // Photocyte: concentric chlorophyll thylakoid rings
    ctx.strokeStyle = "rgba(21, 52, 34, 0.75)";
    ctx.lineWidth = 1.3;
    ctx.beginPath();
    ctx.arc(x, y, r * 0.62, 0, Math.PI * 2);
    ctx.stroke();
    ctx.fillStyle = "#b5e6c5";
    ctx.beginPath();
    ctx.arc(x, y, r * 0.26, 0, Math.PI * 2);
    ctx.fill();
  } else if (roleIdx === 1) {
    // Phagocyte: twin mandible tooth highlights + dark nucleus
    ctx.fillStyle = "#f1f1ef";
    for (const side of [-0.42, 0.42]) {
      ctx.beginPath();
      ctx.arc(x + Math.cos(ang + side) * r * 0.78, y + Math.sin(ang + side) * r * 0.78, r * 0.14, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.fillStyle = "rgba(22, 18, 18, 0.65)";
    ctx.beginPath();
    ctx.arc(x - cos * r * 0.25, y - sin * r * 0.25, r * 0.3, 0, Math.PI * 2);
    ctx.fill();
  } else if (roleIdx === 3) {
    // Myocyte: contractile sarcomere Z-disc bands
    ctx.strokeStyle = "rgba(38, 22, 12, 0.7)";
    ctx.lineWidth = 1.2;
    const perpX = -sin * r * 0.56;
    const perpY = cos * r * 0.56;
    for (const offset of [-0.45, 0, 0.45]) {
      const cx = x + cos * r * offset;
      const cy = y + sin * r * offset;
      ctx.beginPath();
      ctx.moveTo(cx - perpX, cy - perpY);
      ctx.lineTo(cx + perpX, cy + perpY);
      ctx.stroke();
    }
  } else if (roleIdx === 4) {
    // Neurocyte: glowing soma + radiating dendritic branches
    ctx.strokeStyle = "rgba(218, 245, 252, 0.75)";
    ctx.lineWidth = 1.05;
    for (let d = 0; d < 5; d++) {
      const da = ang + (d * Math.PI * 2) / 5;
      ctx.beginPath();
      ctx.moveTo(x, y);
      ctx.lineTo(x + Math.cos(da) * r * 1.32, y + Math.sin(da) * r * 1.32);
      ctx.stroke();
    }
    const pulseAlpha = 0.65 + Math.min(0.35, Math.abs(synapticPulse) * 0.45);
    ctx.fillStyle = `rgba(235, 250, 255, ${pulseAlpha.toFixed(2)})`;
    ctx.beginPath();
    ctx.arc(x, y, r * 0.38, 0, Math.PI * 2);
    ctx.fill();
  } else if (roleIdx === 7) {
    // Keratinocyte: inner hexagonal armor plate
    ctx.strokeStyle = "rgba(241, 241, 239, 0.45)";
    ctx.lineWidth = 1.1;
    ctx.beginPath();
    for (let v = 0; v < 6; v++) {
      const va = ang + (v * Math.PI) / 3;
      const vx = x + Math.cos(va) * r * 0.64;
      const vy = y + Math.sin(va) * r * 0.64;
      if (v === 0) ctx.moveTo(vx, vy);
      else ctx.lineTo(vx, vy);
    }
    ctx.closePath();
    ctx.stroke();
  } else if (roleIdx === 8) {
    // Secreocyte: glowing exocrine vesicles
    ctx.fillStyle = "rgba(235, 245, 198, 0.85)";
    for (let v = 0; v < 4; v++) {
      const va = ang + (v * Math.PI) / 2 + tick * 0.04;
      ctx.beginPath();
      ctx.arc(x + Math.cos(va) * r * 0.42, y + Math.sin(va) * r * 0.42, r * 0.18, 0, Math.PI * 2);
      ctx.fill();
    }
  } else if (roleIdx === 9) {
    // Vascularocyte: pulsing ATP pump core
    const beat = 0.34 + Math.abs(Math.sin(tick * 0.18 + id)) * 0.22;
    ctx.strokeStyle = "rgba(248, 224, 185, 0.85)";
    ctx.lineWidth = 1.4;
    ctx.beginPath();
    ctx.arc(x, y, r * beat, 0, Math.PI * 2);
    ctx.stroke();
    ctx.fillStyle = "#f2d680";
    ctx.beginPath();
    ctx.arc(x, y, r * 0.18, 0, Math.PI * 2);
    ctx.fill();
  } else if (roleIdx === 10) {
    // Stemocyte: twin mitotic nuclei
    ctx.fillStyle = "rgba(20, 32, 30, 0.68)";
    ctx.beginPath();
    ctx.arc(x - cos * r * 0.28, y - sin * r * 0.28, r * 0.25, 0, Math.PI * 2);
    ctx.arc(x + cos * r * 0.28, y + sin * r * 0.28, r * 0.25, 0, Math.PI * 2);
    ctx.fill();
  } else {
    // Default eccentric nucleus (Flagellocyte, Senseocyte, Immunocyte, Scavenger)
    ctx.fillStyle = "rgba(18, 22, 20, 0.58)";
    ctx.beginPath();
    ctx.arc(x + cos * r * 0.22, y + sin * r * 0.22, r * 0.32, 0, Math.PI * 2);
    ctx.fill();
  }

  // Infected pulsing viral capsids inside host cytoplasm
  if (viralLoad > 0.02) {
    const pulse = 0.65 + 0.35 * Math.sin(tick * 0.32 + id);
    ctx.strokeStyle = `rgba(217, 98, 84, ${(0.55 + viralLoad * 0.4 * pulse).toFixed(2)})`;
    ctx.lineWidth = 1.6;
    ctx.beginPath();
    ctx.arc(x, y, r * (1.06 + 0.12 * pulse), 0, Math.PI * 2);
    ctx.stroke();

    ctx.fillStyle = "#ff6b52";
    const specks = Math.max(1, Math.min(6, (viralLoad * 7) | 0));
    for (let s = 0; s < specks; s++) {
      const sa = (s * Math.PI * 2) / specks + tick * 0.1;
      ctx.beginPath();
      ctx.arc(x + Math.cos(sa) * r * 0.48, y + Math.sin(sa) * r * 0.48, Math.max(1.2, r * 0.19), 0, Math.PI * 2);
      ctx.fill();
    }
  }

  if (highlight) {
    ctx.strokeStyle = "#f1f1ef";
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.arc(x, y, r * 1.48, 0, Math.PI * 2);
    ctx.stroke();
  }
}

export function renderWorld(ctx, pixelW, pixelH, worldW, worldH, state) {
  const {
    surface,
    walls,
    gridW = 120,
    gridH = 80,
    cells,
    cellCount,
    cellStride,
    virions,
    virionCount,
    virionStride,
    bonds,
    bondCount,
    bondStride,
    selectedCellId,
    hoveredCellId = 0,
    tick,
    pointer,
    activeBrush,
    brushRadius,
    camera = { zoom: 1, x: worldW * 0.5, y: worldH * 0.5 },
  } = state;

  const zoom = Math.max(0.5, Math.min(3.5, camera.zoom || 1));
  const baseScale = Math.min(pixelW / worldW, pixelH / worldH);
  const scale = baseScale * zoom;
  const tx = pixelW * 0.5 - camera.x * scale;
  const ty = pixelH * 0.5 - camera.y * scale;

  ctx.save();
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.fillStyle = "#121413";
  ctx.fillRect(0, 0, pixelW, pixelH);

  ctx.setTransform(scale, 0, 0, scale, tx, ty);

  // 1. Environmental field surface over Petri dish bounds
  if (surface) {
    ctx.imageSmoothingEnabled = true;
    ctx.drawImage(surface.off, 0, 0, worldW, worldH);
  } else {
    ctx.fillStyle = "#141615";
    ctx.fillRect(0, 0, worldW, worldH);
  }

  // Subtle microscopy reference grid
  ctx.strokeStyle = "rgba(154, 155, 150, 0.06)";
  ctx.lineWidth = 1 / scale;
  ctx.beginPath();
  for (let gx = 100; gx < worldW; gx += 100) {
    ctx.moveTo(gx, 0);
    ctx.lineTo(gx, worldH);
  }
  for (let gy = 100; gy < worldH; gy += 100) {
    ctx.moveTo(0, gy);
    ctx.lineTo(worldW, gy);
  }
  ctx.stroke();

  // Crisp Petri Dish Walls
  if (walls && walls.length === gridW * gridH) {
    const cw = worldW / gridW;
    const ch = worldH / gridH;
    ctx.fillStyle = "#2d302c";
    ctx.strokeStyle = "#5a5d56";
    ctx.lineWidth = Math.max(0.8, 1.1 / scale);
    ctx.beginPath();
    for (let gy = 0; gy < gridH; gy++) {
      const row = gy * gridW;
      for (let gx = 0; gx < gridW; gx++) {
        if (walls[row + gx] !== 0) {
          ctx.rect(gx * cw, gy * ch, cw + 0.15, ch + 0.15);
        }
      }
    }
    ctx.fill();

    // Draw crisp exposed rim lines on wall edges adjacent to open water
    ctx.beginPath();
    for (let gy = 0; gy < gridH; gy++) {
      const row = gy * gridW;
      for (let gx = 0; gx < gridW; gx++) {
        if (walls[row + gx] === 0) continue;
        const wx = gx * cw;
        const wy = gy * ch;
        if (gy === 0 || walls[(gy - 1) * gridW + gx] === 0) {
          ctx.moveTo(wx, wy);
          ctx.lineTo(wx + cw, wy);
        }
        if (gy === gridH - 1 || walls[(gy + 1) * gridW + gx] === 0) {
          ctx.moveTo(wx, wy + ch);
          ctx.lineTo(wx + cw, wy + ch);
        }
        if (gx === 0 || walls[row + gx - 1] === 0) {
          ctx.moveTo(wx, wy);
          ctx.lineTo(wx, wy + ch);
        }
        if (gx === gridW - 1 || walls[row + gx + 1] === 0) {
          ctx.moveTo(wx + cw, wy);
          ctx.lineTo(wx + cw, wy + ch);
        }
      }
    }
    ctx.stroke();
  }

  // Petri dish outer boundary border
  ctx.strokeStyle = "#52544f";
  ctx.lineWidth = Math.max(1.2, 1.6 / scale);
  ctx.strokeRect(0, 0, worldW, worldH);

  // 2. Multicellular Tissue Bridges + Batched Adhesion Bonds + Vascular/Synaptic Pulses
  if (bondCount > 0) {
    ctx.lineCap = "round";

    // Outer organic tissue sleeve unifying each organism into a cohesive body
    ctx.strokeStyle = "rgba(122, 152, 124, 0.24)";
    ctx.lineWidth = 10.5;
    ctx.beginPath();
    for (let i = 0; i < bondCount; i++) {
      const o = i * bondStride;
      ctx.moveTo(bonds[o], bonds[o + 1]);
      ctx.lineTo(bonds[o + 2], bonds[o + 3]);
    }
    ctx.stroke();

    // Crisp inner cytoskeletal filaments (batched)
    ctx.strokeStyle = "rgba(156, 186, 154, 0.62)";
    ctx.lineWidth = 2.2;
    ctx.beginPath();
    for (let i = 0; i < bondCount; i++) {
      const o = i * bondStride;
      ctx.moveTo(bonds[o], bonds[o + 1]);
      ctx.lineTo(bonds[o + 2], bonds[o + 3]);
    }
    ctx.stroke();

    // Active synaptic pathways (batched)
    ctx.strokeStyle = "rgba(91, 188, 214, 0.68)";
    ctx.lineWidth = 1.6;
    ctx.beginPath();
    for (let i = 0; i < bondCount; i++) {
      const o = i * bondStride;
      if (bonds[o + 6] > 0.14) {
        ctx.moveTo(bonds[o], bonds[o + 1]);
        ctx.lineTo(bonds[o + 2], bonds[o + 3]);
      }
    }
    ctx.stroke();

    // Animated vascular ATP beads (batched path)
    ctx.fillStyle = "rgba(242, 214, 128, 0.85)";
    ctx.beginPath();
    for (let i = 0; i < bondCount; i++) {
      const o = i * bondStride;
      const vascFlow = bonds[o + 5];
      if (Math.abs(vascFlow) > 0.03) {
        const ax = bonds[o];
        const ay = bonds[o + 1];
        const dir = vascFlow >= 0 ? 1 : -1;
        const phase = (((tick * 0.09 * dir + i * 0.23) % 1) + 1) % 1;
        const px = ax + (bonds[o + 2] - ax) * phase;
        const py = ay + (bonds[o + 3] - ay) * phase;
        const rad = Math.min(2.4, 1.1 + Math.abs(vascFlow) * 2.2);
        ctx.moveTo(px + rad, py);
        ctx.arc(px, py, rad, 0, Math.PI * 2);
      }
    }
    ctx.fill();
  }

  // 3. Free Viral Particles colored by Strain Hue & Strategy (Lytic icosahedron, Lysogenic ring, Oncogenic spiked)
  for (let i = 0; i < virionCount; i++) {
    const o = i * virionStride;
    const x = virions[o];
    const y = virions[o + 1];
    const hue = safeBioHueDeg(virions[o + 4]) | 0;
    const strategy = virions[o + 5] | 0;
    const r = 2.6;
    const spin = (tick * 0.08 + i * 0.7) % (Math.PI * 2);

    ctx.fillStyle = `hsla(${hue}, 82%, 62%, 0.92)`;
    ctx.strokeStyle = `hsla(${hue}, 88%, 74%, 0.85)`;
    ctx.lineWidth = 0.95;

    if (strategy === 1) {
      // Lysogenic: episomal ring with tiny core
      ctx.beginPath();
      ctx.arc(x, y, r * 1.15, 0, Math.PI * 2);
      ctx.stroke();
      ctx.beginPath();
      ctx.arc(x, y, r * 0.42, 0, Math.PI * 2);
      ctx.fill();
    } else if (strategy === 2) {
      // Oncogenic Mutator: spiked 6-point capsid star
      ctx.beginPath();
      for (let k = 0; k < 12; k++) {
        const a = spin + (k * Math.PI) / 6;
        const rad = k % 2 === 0 ? r * 1.9 : r * 0.72;
        const vx = x + Math.cos(a) * rad;
        const vy = y + Math.sin(a) * rad;
        if (k === 0) ctx.moveTo(vx, vy);
        else ctx.lineTo(vx, vy);
      }
      ctx.closePath();
      ctx.fill();
      ctx.stroke();
    } else {
      // Lytic: crisp icosahedron polygon + capsid tail fibers
      ctx.beginPath();
      for (let k = 0; k < 6; k++) {
        const a = spin + (k * Math.PI) / 3;
        const vx = x + Math.cos(a) * r * 1.15;
        const vy = y + Math.sin(a) * r * 1.15;
        if (k === 0) ctx.moveTo(vx, vy);
        else ctx.lineTo(vx, vy);
      }
      ctx.closePath();
      ctx.fill();
      ctx.beginPath();
      for (let k = 0; k < 3; k++) {
        const a = spin + (k * Math.PI) / 3;
        ctx.moveTo(x - Math.cos(a) * r * 1.65, y - Math.sin(a) * r * 1.65);
        ctx.lineTo(x + Math.cos(a) * r * 1.65, y + Math.sin(a) * r * 1.65);
      }
      ctx.stroke();
    }
  }

  // 4. Living Cells (12 Morphologies + Cognitive Goal Rings + Selected Reticle)
  for (let i = 0; i < cellCount; i++) {
    const o = i * cellStride;
    const id = cells[o] | 0;
    const x = cells[o + 1];
    const y = cells[o + 2];
    const ang = cells[o + 3];
    const r = cells[o + 4];
    const roleIdx = Math.max(0, Math.min(11, cells[o + 5] | 0));
    const hpRatio = cells[o + 6];
    const atpRatio = cells[o + 7];
    const viralLoad = cells[o + 8];
    const tempStress = cells[o + 9];
    const adhesion = cells[o + 10];
    const hueShift = cells[o + 11];
    const signals = cells[o + 13] | 0;
    const modeIdx = cells[o + 17] | 0;
    const cognitiveState = cells[o + 18] | 0;
    const synapticPulse = cells[o + 19];

    drawCellGlyph(ctx, x, y, r, ang, roleIdx, {
      tick,
      id,
      adhesion,
      viralLoad,
      tempStress,
      cognitiveState,
      synapticPulse,
      signals,
      hueShift,
      highlight: id === hoveredCellId && id !== selectedCellId,
    });

    if (id === selectedCellId) {
      ctx.strokeStyle = "#f1f1ef";
      ctx.lineWidth = 1.6 / Math.sqrt(zoom);
      ctx.beginPath();
      ctx.arc(x, y, r * 1.82, 0, Math.PI * 2);
      ctx.stroke();

      // ATP arc (sage-emerald) & HP arc (coral)
      ctx.strokeStyle = "#8fa68e";
      ctx.lineWidth = 2.0 / Math.sqrt(zoom);
      ctx.beginPath();
      ctx.arc(x, y, r * 2.12, -Math.PI / 2, -Math.PI / 2 + atpRatio * Math.PI * 2);
      ctx.stroke();

      ctx.strokeStyle = "#d46552";
      ctx.beginPath();
      ctx.arc(x, y, r * 2.4, -Math.PI / 2, -Math.PI / 2 + hpRatio * Math.PI * 2);
      ctx.stroke();

      ctx.fillStyle = "#f1f1ef";
      ctx.font = `${Math.max(8, 10 / Math.sqrt(zoom))}px "Geist Mono", monospace`;
      ctx.textAlign = "center";
      ctx.fillText(`M${modeIdx}`, x, y - r * 2.75);
    }
  }

  // 5. Interactive Brush Reticle
  if (pointer && pointer.inside && activeBrush !== "inspect") {
    const pr =
      activeBrush === "wall" || activeBrush === "erase"
        ? Math.max(12, brushRadius * (activeBrush === "wall" ? 0.45 : 0.55))
        : brushRadius;
    ctx.strokeStyle = BRUSH_COLORS[activeBrush] || "rgba(143, 166, 142, 0.85)";
    ctx.lineWidth = 1.4 / scale;
    ctx.setLineDash([5 / scale, 4 / scale]);
    ctx.beginPath();
    if (activeBrush === "wall" || activeBrush === "erase") {
      ctx.rect(pointer.x - pr, pointer.y - pr, pr * 2, pr * 2);
    } else {
      ctx.arc(pointer.x, pointer.y, pr, 0, Math.PI * 2);
    }
    ctx.stroke();
    ctx.setLineDash([]);
  }

  ctx.restore();
}
