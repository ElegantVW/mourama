const $ = (s, el = document) => el.querySelector(s);
const $$ = (s, el = document) => [...el.querySelectorAll(s)];

const state = {
  token: localStorage.getItem("mourama_token") || "",
  me: null,
  map: null,
  selected: null,
};

function headers() {
  const h = { "content-type": "application/json" };
  if (state.token) h.authorization = "Bearer " + state.token;
  return h;
}

async function api(path, opt = {}) {
  const r = await fetch(path, {
    ...opt,
    headers: { ...headers(), ...(opt.headers || {}) },
  });
  const j = await r.json().catch(() => ({}));
  if (!r.ok) throw new Error(j.error || "mourama: the window could not reach the wood.");
  return j;
}

function showErr(id, e) {
  const el = $(id);
  el.hidden = false;
  el.textContent = e.message || String(e);
}

function fmt(n) {
  if (typeof n !== "number") return n;
  return Math.floor(n).toLocaleString("pt-PT");
}

function when(ts) {
  if (!ts) return "";
  return new Date(ts * 1000).toLocaleString();
}

async function boot() {
  if (state.token) {
    try {
      await refresh();
      showPlay();
      return;
    } catch {
      state.token = "";
      localStorage.removeItem("mourama_token");
    }
  }
  $("#gate").hidden = false;
  $("#play").hidden = true;
}

function showPlay() {
  $("#gate").hidden = true;
  $("#play").hidden = false;
}

$("#claim-form").addEventListener("submit", async (ev) => {
  ev.preventDefault();
  const fd = new FormData(ev.target);
  try {
    const j = await api("/api/claim", {
      method: "POST",
      body: JSON.stringify({
        invite: fd.get("invite"),
        name: fd.get("name"),
        password: fd.get("password"),
      }),
    });
    state.token = j.token;
    localStorage.setItem("mourama_token", j.token);
    await refresh();
    showPlay();
  } catch (e) {
    showErr("#gate-err", e);
  }
});

$("#login-form").addEventListener("submit", async (ev) => {
  ev.preventDefault();
  const fd = new FormData(ev.target);
  try {
    const j = await api("/api/login", {
      method: "POST",
      body: JSON.stringify({
        name: fd.get("name"),
        password: fd.get("password"),
      }),
    });
    state.token = j.token;
    localStorage.setItem("mourama_token", j.token);
    await refresh();
    showPlay();
  } catch (e) {
    showErr("#gate-err", e);
  }
});

$$(".tabs button").forEach((b) => {
  b.addEventListener("click", () => {
    $$(".tabs button").forEach((x) => x.classList.remove("on"));
    b.classList.add("on");
    $$(".tab").forEach((t) => (t.hidden = true));
    $("#tab-" + b.dataset.tab).hidden = false;
    if (b.dataset.tab === "map") drawMap();
    if (b.dataset.tab === "horn") drawHorn();
    if (b.dataset.tab === "reports") drawReports();
  });
});

async function refresh() {
  state.me = await api("/api/me");
  state.map = await api("/api/map");
  $("#clock").textContent = when(state.me.clock);
  await maybeGlitter();
  drawHill();
}

function drawHill() {
  const hills = state.me.hills || [];
  if (!hills.length) {
    $("#res").innerHTML = "";
    $("#hills").innerHTML = "<p class='panel'>Your court holds no hill.</p>";
    return;
  }
  const h = hills[0];
  const r = h.resources || {};
  $("#res").innerHTML = [
    ["Cobre", r.cobre],
    ["Estanho", r.estanho],
    ["Seara", r.seara],
    ["Orvalho", r.orvalho],
  ]
    .map(([k, v]) => `<div class="pile"><div class="k">${k}</div><div class="v">${fmt(v)}</div></div>`)
    .join("");
  $("#hills").innerHTML = (state.slip ? `<div class="slip">${state.slip}</div>` : "") + hills.map(renderHill).join("");
  $("#hills").querySelectorAll("[data-up]").forEach((b) => {
    b.addEventListener("click", () => actUpgrade(b.dataset.cid, b.dataset.up));
  });
  $("#hills").querySelectorAll("[data-train]").forEach((b) => {
    b.addEventListener("click", () => {
      const wrap = b.closest(".hill");
      actTrain(b.dataset.cid, wrap.querySelector("[name=unit]").value, wrap.querySelector("[name=count]").value);
    });
  });
  $("#hills").querySelectorAll("[data-veil]").forEach((b) => {
    b.addEventListener("click", () => actEncanto(b.dataset.cid));
  });
}

function renderHill(h) {
  const b = h.buildings || {};
  const g = h.garrison || {};
  const rows = Object.keys(b)
    .map((k) => {
      const x = b[k];
      const rising = x.upgrade_done ? ` · rising until ${when(x.upgrade_done)}` : "";
      return `<div class="row"><span>${x.title} L${x.level}${rising}</span><button data-up="${k}" data-cid="${h.id}">Raise</button></div>`;
    })
    .join("");
  const folk = Object.entries(g)
    .map(([k, n]) => `${k} ${n}`)
    .join(" · ") || "none";
  const units = ["pastor", "trasgo", "falcao", "javali", "guerreiro", "serpe"];
  return `<article class="hill">
    <h2>${h.name} <span class="tiny">(${h.q},${h.r})</span></h2>
    <p class="tiny">bronze ${fmt(h.bronze)} · encanto ${h.encanto_until > state.me.clock ? "until " + when(h.encanto_until) : "open"}</p>
    ${rows}
    <p>Garrison: ${folk}</p>
    <div class="inline">
      <select name="unit">${units.map((u) => `<option>${u}</option>`).join("")}</select>
      <input name="count" type="number" min="1" max="50" value="1">
      <button data-train data-cid="${h.id}">Train</button>
    </div>
    <button class="ghost" data-veil data-cid="${h.id}">Cast encanto</button>
  </article>`;
}

function hexPixel(q, r, size) {
  const x = size * Math.sqrt(3) * (q + r / 2);
  const y = size * 1.5 * r;
  return [x, y];
}

function drawMap() {
  if (!state.map) return;
  const size = 22;
  const tiles = state.map.tiles || [];
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  const placed = tiles.map((t) => {
    const [x, y] = hexPixel(t.q, t.r, size);
    minX = Math.min(minX, x);
    minY = Math.min(minY, y);
    maxX = Math.max(maxX, x);
    maxY = Math.max(maxY, y);
    return { t, x, y };
  });
  const pad = 28;
  const el = $("#hexmap");
  el.innerHTML = "";
  el.style.height = maxY - minY + pad * 2 + 48 + "px";
  el.style.width = maxX - minX + pad * 2 + 48 + "px";
  for (const p of placed) {
    const d = document.createElement("div");
    d.className = "hex " + p.t.kind + (p.t.mine ? " mine" : "") + (p.t.veiled ? " veiled" : "");
    d.style.left = p.x - minX + pad + "px";
    d.style.top = p.y - minY + pad + "px";
    d.textContent = p.t.kind === "outeiro" ? "" : (p.t.name || "").slice(0, 3);
    d.title = `${p.t.kind} (${p.t.q},${p.t.r}) ${p.t.name || ""}`;
    d.addEventListener("click", () => selectTile(p.t, d));
    el.appendChild(d);
  }
}

async function selectTile(t, d) {
  $$(".hex").forEach((x) => x.classList.remove("on"));
  d.classList.add("on");
  state.selected = t;
  const hill = await api(`/api/hill?q=${t.q}&r=${t.r}`);
  const home = (state.me.hills || [])[0];
  let send = "";
  if (home && !(hill.owner_id && hill.owner_id === state.me.account_id && hill.q === home.q)) {
    send = `<div class="inline">
      <select id="mission"><option>raid</option><option>scout</option><option>bind</option></select>
      <button id="send">Send</button>
    </div>
    <p class="tiny">Scout is falcão only. Bind needs a serpe and 100 orvalho.</p>`;
  }
  const body = hill.empty
    ? `<p>Empty outeiro (${t.q},${t.r}).</p>`
    : `<p><strong>${hill.name || t.kind}</strong> (${t.q},${t.r})</p>
       <p class="tiny">${hill.veiled ? "veiled by encanto" : JSON.stringify(hill.garrison)}</p>`;
  $("#map-detail").innerHTML = body + send;
  const btn = $("#send");
  if (btn) {
    btn.addEventListener("click", () => sendTo(t, $("#mission").value));
  }
}

async function sendTo(t, mission) {
  const home = (state.me.hills || [])[0];
  if (!home) return;
  const g = home.garrison || {};
  const units = {};
  if (mission === "scout") {
    const n = g.falcao || 0;
    if (!n) return alert("mourama: you need a falcão.");
    units.falcao = Math.min(1, n);
  } else if (mission === "bind") {
    if (!g.serpe) return alert("mourama: a bind needs a serpe.");
    units.serpe = 1;
    if (g.pastor) units.pastor = g.pastor;
    if (g.guerreiro) units.guerreiro = g.guerreiro;
    if (g.javali) units.javali = g.javali;
    if (g.trasgo) units.trasgo = g.trasgo;
  } else {
    for (const k of Object.keys(g)) {
      if (g[k] > 0) units[k] = g[k];
    }
  }
  try {
    await api("/api/send", {
      method: "POST",
      body: JSON.stringify({
        castro_id: home.id,
        q: t.q,
        r: t.r,
        units,
        mission,
      }),
    });
    await refresh();
    drawMap();
  } catch (e) {
    alert(e.message);
  }
}

function reportText(r) {
  if (r.body && typeof r.body.text === "string") return r.body.text;
  if (r.body && r.body.won === true) return r.title + " — the hill yielded.";
  if (r.body && r.body.won === false) return r.title + " — they held.";
  if (r.body && r.body.seen) return r.title + " — the falcão looked through.";
  return r.title || "";
}

async function drawReports() {
  const j = await api("/api/reports");
  const rows = j.reports || [];
  $("#reports").innerHTML = rows.length
    ? rows
        .map(
          (r) => `<article class="report"><div class="when">${when(r.created)}</div><strong>${r.title}</strong><p class="said">${reportText(r)}</p></article>`
        )
        .join("")
    : "<p class='panel'>No reports yet.</p>";
}

function landsIn(arrive) {
  const left = Math.max(0, arrive - Math.floor(Date.now() / 1000));
  if (left <= 0) return "landing…";
  const h = String(Math.floor(left / 3600)).padStart(2, "0");
  const m = String(Math.floor((left % 3600) / 60)).padStart(2, "0");
  const s = String(left % 60).padStart(2, "0");
  return `${h}:${m}:${s}`;
}

async function drawHorn() {
  const j = await api("/api/commands");
  const incoming = j.incoming || [];
  const mine = j.mine || [];
  const row = (a, hot) =>
    `<article class="report${hot ? " hot" : ""}"><strong>${a.mission || "march"} — ${(a.from_court || "?") + " → " + (a.to_hill || (a.to || []).join(","))}</strong><div class="when">${landsIn(a.arrive)}</div></article>`;
  $("#horn").innerHTML =
    `<h3>Incoming (${incoming.length})</h3>` +
    (incoming.length
      ? incoming.map((a) => row(a, true)).join("")
      : "<p class='panel'>The wood is quiet.</p>") +
    `<h3>Marching (${mine.length})</h3>` +
    (mine.length
      ? mine.map((a) => row(a, false)).join("")
      : "<p class='panel'>No folk on the road.</p>");
}
setInterval(() => {
  if (!$("#tab-horn").hidden) drawHorn();
}, 1000);

async function maybeGlitter() {
  try {
    const j = await api("/api/reports");
    const rows = j.reports || [];
    const note = rows.find((r) => r.body && typeof r.body.text === "string");
    if (!note) return;
    state.slip = note.body.text;
    const key = "mourama_glitter_" + note.id;
    if (localStorage.getItem(key)) return;
    const g = $("#glitter");
    $("#glitter-msg").textContent = note.body.text;
    g.hidden = false;
    $("#glitter-ok").onclick = () => {
      g.hidden = true;
      localStorage.setItem(key, "1");
    };
  } catch {
    /* silent */
  }
}

async function actUpgrade(cid, building) {
  try {
    await api("/api/upgrade", {
      method: "POST",
      body: JSON.stringify({ castro_id: Number(cid), building }),
    });
    await refresh();
  } catch (e) {
    alert(e.message);
  }
}

async function actTrain(cid, unit, count) {
  try {
    await api("/api/train", {
      method: "POST",
      body: JSON.stringify({
        castro_id: Number(cid),
        unit,
        count: Number(count) || 1,
      }),
    });
    await refresh();
  } catch (e) {
    alert(e.message);
  }
}

async function actEncanto(cid) {
  try {
    await api("/api/encanto", {
      method: "POST",
      body: JSON.stringify({ castro_id: Number(cid) }),
    });
    await refresh();
  } catch (e) {
    alert(e.message);
  }
}

boot();
setInterval(() => {
  if (state.token && !$("#play").hidden) refresh().catch(() => {});
}, 4000);
