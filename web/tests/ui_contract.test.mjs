import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { ROLE_META, OVERLAY_META } from "../renderer.js";
import { INPUT_LABELS, OUTPUT_LABELS } from "../inspector.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const webDir = path.resolve(__dirname, "..");

test("renderer metadata covers 6 cell roles and 5 field overlays", () => {
  assert.equal(ROLE_META.length, 6);
  assert.deepEqual(
    ROLE_META.map((r) => r.name),
    ["Producer", "Protector", "Predator", "Swimmer", "Replicator", "Infected"]
  );
  assert.deepEqual(Object.keys(OVERLAY_META), [
    "biology",
    "temperature",
    "nutrients",
    "pheromones",
    "stress",
  ]);
});

test("inspector neural labels match 10-input 6-output architecture", () => {
  assert.equal(INPUT_LABELS.length, 10);
  assert.equal(OUTPUT_LABELS.length, 6);
});

test("index.html uses relative ./ paths and contains all required DOM IDs", () => {
  const html = fs.readFileSync(path.join(webDir, "index.html"), "utf8");
  assert.ok(html.includes('href="./styles.css"'));
  assert.ok(html.includes('src="./app.js"'));
  assert.ok(!html.includes('src="/') && !html.includes('href="/'), "no root-relative paths");

  const requiredIds = [
    "error-banner",
    "sim-canvas",
    "nn-canvas",
    "pop-sparkline",
    "evo-sparkline",
    "btn-play",
    "btn-step",
    "btn-reset",
    "sim-speed",
    "btn-spawn-center",
    "btn-virus-center",
    "hud-cursor",
  ];
  for (const id of requiredIds) {
    assert.ok(html.includes(`id="${id}"`), `missing element id="${id}"`);
  }
});
