// Bio-microscopy Canvas2D renderer for environmental diffusion fields, adhesion bonds,
// 6 distinct cell morphologies, virions, and inspection reticles.

export const ROLE_META = [
  { id: 0, name: "Producer", label: "Producer (Photocyte)", color: "#38d9a9", glow: "rgba(56,217,169,0.28)" },
  { id: 1, name: "Protector", label: "Protector (Immunocyte)", color: "#4dabf7", glow: "rgba(77,171,247,0.32)" },
  { id: 2, name: "Predator", label: "Predator (Phagocyte)", color: "#ff6b6b", glow: "rgba(255,107,107,0.30)" },
  { id: 3, name: "Swimmer", label: "Swimmer (Myocyte)", color: "#ffd43b", glow: "rgba(255,212,59,0.28)" },
  { id: 4, name: "Replicator", label: "Replicator (Stem)", color: "#3bc9db", glow: "rgba(59,201,219,0.28)" },
  { id: 5, name: "Infected", label: "Infected (Viral Host)", color: "#e599f7", glow: "rgba(229,153,247,0.38)" },
];

export const OVERLAY_META = {
  biology: {
    label: "Normal Biology",
    low: "Cold / Nutrient-poor",
    high: "Warm / Nutrient-rich",
    gradient: "linear-gradient(90deg, #0b1824, #0c2c2c, #2f1b14)",
  },
  temperature: {
    label: "Temperature Heatmap",
    low: "-30°C Cryo",
    high: "95°C Hydrothermal",
    gradient: "linear-gradient(90deg, #1b6ef3, #1e2d3d, #f59f00, #ff3b30)",
  },
  nutrients: {
    label: "Nutrient / Glucose Field",
    low: "Depleted (0.0)",
    high: "Saturated (1.2+)",
    gradient: "linear-gradient(90deg, #060b10, #0d3b2e, #38d9a9)",
  },
  pheromones: {
    label: "Swarm Pheromones",
    low: "Food/Rally (Cyan)",
    high: "Alarm/Threat (Crimson)",
    gradient: "linear-gradient(90deg, #060b10, #15aabf, #ff4d6d)",
  },
  stress: {
    label: "Infection / Immune Stress",
    low: "Antibody Shield (Teal)",
    high: "Alarm / Outbreak (Violet)",
    gradient: "linear-gradient(90deg, #060b10, #22b8cf, #be4bdb)",
  },
};

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
  const { temp, nutrients, alarm, food, toxin } = fields;

  for (let i = 0; i < len; i++) {
    const p = i * 4;
    let r = 6;
    let g = 11;
    let b = 17;

    if (overlay === "temperature") {
      const t = temp[i];
      if (t < 24) {
        const k = Math.min(1, (24 - t) / 52);
        r = 14 - k * 8;
        g = 22 + k * 75;
        b = 34 + k * 205;
      } else {
        const k = Math.min(1, (t - 24) / 68);
        r = 16 + k * 235;
        g = 22 + Math.sin(k * Math.PI) * 125;
        b = 32 - k * 26;
      }
    } else if (overlay === "nutrients") {
      const n = Math.min(1, nutrients[i] / 1.05);
      r = 5 + n * 24;
      g = 11 + n * 165;
      b = 17 + n * 105;
    } else if (overlay === "pheromones") {
      const al = Math.min(1, alarm[i] * 1.6);
      const fd = Math.min(1, food[i] * 1.6);
      r = 7 + al * 230 + fd * 12;
      g = 11 + fd * 165 + al * 28;
      b = 18 + fd * 195 + al * 85;
    } else if (overlay === "stress") {
      const tx = Math.min(1, toxin[i] * 1.6);
      const al = Math.min(1, alarm[i] * 1.4);
      r = 7 + al * 185 + tx * 20;
      g = 11 + tx * 160 + al * 25;
      b = 18 + tx * 190 + al * 210;
    } else {
      // Normal Biology: subtle translucent ambient tint for nutrients, thermal pockets & antibodies
      const n = Math.min(1, nutrients[i] / 1.1);
      const t = temp[i];
      const hot = Math.max(0, (t - 36) / 60);
      const cold = Math.max(0, (14 - t) / 45);
      const tx = Math.min(1, toxin[i]);
      const al = Math.min(1, alarm[i]);
      r = 6 + hot * 85 + al * 52;
      g = 11 + n * 28 + tx * 34 + hot * 18;
      b = 17 + n * 22 + cold * 90 + tx * 58;
    }

    data[p] = Math.max(0, Math.min(255, r | 0));
    data[p + 1] = Math.max(0, Math.min(255, g | 0));
    data[p + 2] = Math.max(0, Math.min(255, b | 0));
    data[p + 3] = 255;
  }

  octx.putImageData(img, 0, 0);
}

export function renderWorld(ctx, canvasW, canvasH, worldW, worldH, state) {
  const {
    surface,
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
    tick,
    pointer,
    activeBrush,
    brushRadius,
  } = state;

  const sx = canvasW / worldW;
  const sy = canvasH / worldH;

  ctx.save();
  ctx.clearRect(0, 0, canvasW, canvasH);

  // 1. Environmental field layer
  if (surface) {
    ctx.imageSmoothingEnabled = true;
    ctx.drawImage(surface.off, 0, 0, canvasW, canvasH);
  } else {
    ctx.fillStyle = "#060b11";
    ctx.fillRect(0, 0, canvasW, canvasH);
  }

  // Subtle microscopy coordinate grid
  ctx.strokeStyle = "rgba(120, 160, 200, 0.05)";
  ctx.lineWidth = 1;
  const stepX = 100 * sx;
  const stepY = 100 * sy;
  ctx.beginPath();
  for (let gx = stepX; gx < canvasW; gx += stepX) {
    ctx.moveTo(gx, 0);
    ctx.lineTo(gx, canvasH);
  }
  for (let gy = stepY; gy < canvasH; gy += stepY) {
    ctx.moveTo(0, gy);
    ctx.lineTo(canvasW, gy);
  }
  ctx.stroke();

  // 2. Multicellular Adhesion Bonds
  if (bondCount > 0) {
    ctx.lineCap = "round";
    for (let i = 0; i < bondCount; i++) {
      const o = i * bondStride;
      const ax = bonds[o] * sx;
      const ay = bonds[o + 1] * sy;
      const bx = bonds[o + 2] * sx;
      const by = bonds[o + 3] * sy;
      const strength = bonds[o + 4];
      ctx.strokeStyle = `rgba(99, 230, 190, ${(0.22 + strength * 0.42).toFixed(2)})`;
      ctx.lineWidth = (1.5 + strength * 2.0) * Math.min(sx, sy);
      ctx.beginPath();
      ctx.moveTo(ax, ay);
      ctx.lineTo(bx, by);
      ctx.stroke();
    }
  }

  // 3. Free Viral Particles (Virions)
  for (let i = 0; i < virionCount; i++) {
    const o = i * virionStride;
    const x = virions[o] * sx;
    const y = virions[o + 1] * sy;
    const hue = (virions[o + 4] * 360) | 0;
    const r = 2.4 * Math.min(sx, sy);

    ctx.fillStyle = `hsla(${hue}, 92%, 68%, 0.9)`;
    ctx.beginPath();
    ctx.arc(x, y, r, 0, Math.PI * 2);
    ctx.fill();

    // Viral capsid spikes
    ctx.strokeStyle = `hsla(${hue}, 95%, 78%, 0.55)`;
    ctx.lineWidth = 0.9;
    const spin = (tick * 0.08 + i) % (Math.PI * 2);
    ctx.beginPath();
    for (let k = 0; k < 3; k++) {
      const a = spin + (k * Math.PI) / 3;
      const dx = Math.cos(a) * r * 1.8;
      const dy = Math.sin(a) * r * 1.8;
      ctx.moveTo(x - dx, y - dy);
      ctx.lineTo(x + dx, y + dy);
    }
    ctx.stroke();
  }

  // 4. Living Cells
  for (let i = 0; i < cellCount; i++) {
    const o = i * cellStride;
    const id = cells[o] | 0;
    const x = cells[o + 1] * sx;
    const y = cells[o + 2] * sy;
    const ang = cells[o + 3];
    const r = cells[o + 4] * Math.min(sx, sy);
    const roleIdx = Math.max(0, Math.min(5, cells[o + 5] | 0));
    const hpRatio = cells[o + 6];
    const atpRatio = cells[o + 7];
    const viralLoad = cells[o + 8];
    const tempStress = cells[o + 9];
    const adhesion = cells[o + 10];
    const signals = cells[o + 13] | 0;

    const meta = ROLE_META[roleIdx];

    // Neural signal halo (alarm / antibody / mitosis)
    if (signals & 4) {
      ctx.strokeStyle = "rgba(77, 171, 247, 0.45)";
      ctx.lineWidth = 1.6;
      ctx.beginPath();
      ctx.arc(x, y, r * 1.7, 0, Math.PI * 2);
      ctx.stroke();
    } else if (signals & 1) {
      ctx.strokeStyle = "rgba(255, 77, 109, 0.5)";
      ctx.lineWidth = 1.3;
      ctx.beginPath();
      ctx.arc(x, y, r * 1.55, 0, Math.PI * 2);
      ctx.stroke();
    }

    // Thermal stress aura (heat-shock amber/red vs frostbite icy cyan)
    if (Math.abs(tempStress) > 0.05) {
      ctx.strokeStyle =
        tempStress > 0
          ? `rgba(255, 92, 51, ${Math.min(0.85, tempStress * 0.65).toFixed(2)})`
          : `rgba(116, 192, 252, ${Math.min(0.85, -tempStress * 0.65).toFixed(2)})`;
      ctx.lineWidth = 2.0;
      ctx.beginPath();
      ctx.arc(x, y, r * 1.32, 0, Math.PI * 2);
      ctx.stroke();
    }

    // Swimmer undulating flagella tail
    if (roleIdx === 3) {
      const tailLen = r * 1.9;
      const wag = Math.sin(tick * 0.45 + id) * 0.5;
      const bx = x - Math.cos(ang) * r;
      const by = y - Math.sin(ang) * r;
      const tx = x - Math.cos(ang + wag) * (r + tailLen);
      const ty = y - Math.sin(ang + wag) * (r + tailLen);
      ctx.strokeStyle = "rgba(255, 224, 102, 0.78)";
      ctx.lineWidth = 1.6;
      ctx.beginPath();
      ctx.moveTo(bx, by);
      ctx.lineTo(tx, ty);
      ctx.stroke();
    }

    // Cell outer membrane
    ctx.fillStyle = meta.glow;
    ctx.beginPath();
    ctx.arc(x, y, r * 1.12, 0, Math.PI * 2);
    ctx.fill();

    ctx.fillStyle = meta.color;
    ctx.beginPath();
    if (roleIdx === 2) {
      // Predator: pointed phagocyte wedge
      ctx.arc(x, y, r, ang + 0.42, ang + Math.PI * 2 - 0.42);
      ctx.lineTo(x + Math.cos(ang) * r * 0.35, y + Math.sin(ang) * r * 0.35);
      ctx.closePath();
    } else {
      ctx.arc(x, y, r, 0, Math.PI * 2);
    }
    ctx.fill();

    // Adhesion rim marker
    if (adhesion > 0.45) {
      ctx.strokeStyle = "rgba(150, 242, 215, 0.75)";
      ctx.lineWidth = 1.4;
      ctx.stroke();
    }

    // Organelle / nucleus detail
    ctx.fillStyle = "rgba(6, 12, 18, 0.55)";
    if (roleIdx === 4) {
      // Replicator twin mitotic nuclei
      ctx.beginPath();
      ctx.arc(x - Math.cos(ang) * r * 0.28, y - Math.sin(ang) * r * 0.28, r * 0.26, 0, Math.PI * 2);
      ctx.arc(x + Math.cos(ang) * r * 0.28, y + Math.sin(ang) * r * 0.28, r * 0.26, 0, Math.PI * 2);
      ctx.fill();
    } else {
      ctx.beginPath();
      ctx.arc(x + Math.cos(ang) * r * 0.22, y + Math.sin(ang) * r * 0.22, r * 0.34, 0, Math.PI * 2);
      ctx.fill();
    }

    // Viral capsid dots inside infected cell
    if (viralLoad > 0.05) {
      ctx.fillStyle = "#ff4d6d";
      const specks = Math.max(1, Math.min(5, (viralLoad * 6) | 0));
      for (let s = 0; s < specks; s++) {
        const sa = (s * Math.PI * 2) / specks + tick * 0.08;
        ctx.beginPath();
        ctx.arc(x + Math.cos(sa) * r * 0.45, y + Math.sin(sa) * r * 0.45, 1.5, 0, Math.PI * 2);
        ctx.fill();
      }
    }

    // Selected X-ray cell reticle + ATP/HP arcs
    if (id === selectedCellId) {
      ctx.strokeStyle = "#ffffff";
      ctx.lineWidth = 1.6;
      ctx.beginPath();
      ctx.arc(x, y, r * 1.85, 0, Math.PI * 2);
      ctx.stroke();

      // ATP arc (emerald) & HP arc (coral)
      ctx.strokeStyle = "#38d9a9";
      ctx.lineWidth = 2.0;
      ctx.beginPath();
      ctx.arc(x, y, r * 2.15, -Math.PI / 2, -Math.PI / 2 + atpRatio * Math.PI * 2);
      ctx.stroke();

      ctx.strokeStyle = "#ff8787";
      ctx.beginPath();
      ctx.arc(x, y, r * 2.42, -Math.PI / 2, -Math.PI / 2 + hpRatio * Math.PI * 2);
      ctx.stroke();
    }
  }

  // 5. Brush cursor reticle
  if (pointer && pointer.inside && activeBrush !== "inspect") {
    const px = pointer.x * sx;
    const py = pointer.y * sy;
    const pr = brushRadius * Math.min(sx, sy);
    ctx.strokeStyle =
      activeBrush === "heat"
        ? "rgba(255, 146, 43, 0.85)"
        : activeBrush === "cryo"
          ? "rgba(116, 192, 252, 0.85)"
          : activeBrush === "virus"
            ? "rgba(229, 153, 247, 0.85)"
            : activeBrush === "toxin"
              ? "rgba(77, 171, 247, 0.85)"
              : "rgba(56, 217, 169, 0.85)";
    ctx.lineWidth = 1.4;
    ctx.setLineDash([4, 3]);
    ctx.beginPath();
    ctx.arc(px, py, pr, 0, Math.PI * 2);
    ctx.stroke();
    ctx.setLineDash([]);
  }

  ctx.restore();
}
