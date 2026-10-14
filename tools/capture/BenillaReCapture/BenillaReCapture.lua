--[[ BenillaReCapture: what the running client measures where the emulator cannot reach.

  An addon for the 1.12.1 client. Type /brecapture in the world; it runs a queue of jobs, one per
  frame, and writes what it measured to the SavedVariable BenillaReCaptureDB, which the client
  flushes to WTF/Account/<ACCOUNT>/SavedVariables/BenillaReCapture.lua at /reload or logout:

    glyphs, kerning, pangrams   FontString:GetStringWidth for every probe string, face, size and
                                outline flag, the width of every printable ASCII pair less its two
                                glyphs (stored where it is not 0), and five whole sentences. The
                                glyph metrics come from FreeType, which the bit-exact tests replace
                                with a stub; crates/font/tests/glyph_oracle.rs reads this file from
                                BENILLA_RE_GLYPH_CAPTURE.
    random                      math.random() and math.random(1, 1000000), 64 draws each after
                                math.randomseed(N) for six seeds; crates/random/tests/
                                math_random_oracle.rs reads this file from
                                BENILLA_RE_MATH_RANDOM_CAPTURE.
    globals                     the name and type of every global of the world's Lua state.
    meta, probes                the run's settings, and each probe's code point and encoding.

  It needs a world login only because addons load there; nothing it measures depends on the
  server. The file comes from a run of the real client and is never committed (METHOD.md, The
  harness). The client keeps SavedVariables in memory until it writes them, so the queue is capped
  (CFG.max_sizes, CFG.pair_max_sizes): an unbounded run exhausts the 32-bit client, and a file cut
  off mid-write stops the next start until it is deleted.

  The client's Lua is 5.0: no `%` or `#` operators (math.mod, table.getn), no hex literals.
--]]

-- ---------------------------------------------------------------------------------------------
-- CONFIG (trim if a run is too heavy; defaults are the exhaustive-but-bounded set)
-- ---------------------------------------------------------------------------------------------
local CFG = {
  -- bundled faces to probe in addition to whatever the live font objects use. SetFont auto-skips
  -- any that fail to load, so unknown/locale-absent faces cost nothing.
  candidate_faces = {
    "Fonts\\FRIZQT__.TTF", "Fonts\\ARIALN.TTF", "Fonts\\MORPHEUS.TTF", "Fonts\\SKURRI.TTF",
    "Fonts\\2002.TTF", "Fonts\\2002B.TTF", "Fonts\\NIM_____.TTF",
    -- locale (zhCN/koKR/ruRU) faces — capped to UI ranges below if huge:
    "Fonts\\FZLBJW.TTF", "Fonts\\FZXHJW.TTF", "Fonts\\FZXHLJW.TTF", "Fonts\\ARKai_T.ttf",
    "Fonts\\bHEI00M.ttf", "Fonts\\bHEI01B.ttf", "Fonts\\bKAI00M.ttf", "Fonts\\bLEI00D.ttf",
  },
  flags_singles = { "", "OUTLINE" },  -- normal and outline (an outline adds 1.0 to the step base, 0x5ca2b0)
  flags_pairs   = { "" },             -- kerning delta captured on the normal config
  do_pairs      = true,
  do_pangrams   = true,
  do_random     = true,
  do_globals    = true,
  random_seeds  = { 1, 0, 42, 12345, 5875, 2147483647 },
  random_count  = 64,
  -- a face with more probed glyphs than this (i.e. a huge CJK face) is capped to the Latin/UI ranges
  -- only. Latin faces are well under this and captured in full.
  cjk_cap_marker = true,
  -- MEMORY BOUND (32-bit 1.12 client; SavedVariables flush only at logout, so the WHOLE dataset must
  -- fit in RAM at once — an unbounded sweep OOMs and crashes). Cap discovered sizes to a representative
  -- spread, and run the full O(95^2) kerning matrix at only a few sizes: kern base is size-independent
  -- and the advance-adjust is linear in size, so a low+high pair pins it. Raise if a run has headroom.
  max_sizes      = 8,
  pair_max_sizes = 2,
}

local PANGRAMS = {
  "The quick brown fox jumps over the lazy dog.",
  "Sphinx of black quartz, judge my vow.",
  "Pack my box with five dozen liquor jugs.",
  "AVAWAYToTaVaWeYoff fi fl 1234567890 !@#$%^&*()",  -- kern-heavy + ligature-ish + digits/symbols
  "WAVE战 Ту По",  -- mixed (ASCII + any locale glyphs the face has)
}

-- ---------------------------------------------------------------------------------------------
-- codepoint probe set: capture (input-bytes -> width), encoding-agnostic ground truth.
-- ASCII single bytes are unambiguous; 0x80-0xFF captured as raw bytes (codepage hypothesis) AND
-- U+00A0..U+017F + curated punctuation captured as UTF-8 (the UTF-8 hypothesis). The client's own
-- font path resolves whichever; we record both so the fixture is complete regardless.
-- ---------------------------------------------------------------------------------------------
local floor = math.floor
local function imod(a, b) return a - floor(a / b) * b end

-- pick a representative spread of at most maxn values from a sorted list (always keeps first + last)
local function cap_spread(list, maxn)
  local n = table.getn(list)
  if n <= maxn then return list end
  if maxn <= 1 then return { list[1] } end
  local seen, out = {}, {}
  local k = 1
  while k <= maxn do
    local idx = floor((k - 1) * (n - 1) / (maxn - 1) + 0.5) + 1
    local v = list[idx]
    if not seen[v] then seen[v] = true; table.insert(out, v) end
    k = k + 1
  end
  return out
end

local function utf8(cp)
  if cp < 128 then
    return string.char(cp)
  elseif cp < 2048 then
    return string.char(192 + floor(cp / 64), 128 + imod(cp, 64))
  else
    return string.char(224 + floor(cp / 4096), 128 + imod(floor(cp / 64), 64), 128 + imod(cp, 64))
  end
end

-- build the probe list: each entry = { label, bytes, cp, enc }
local PROBES = {}
local function addprobe(label, bytes, cp, enc)
  table.insert(PROBES, { label = label, bytes = bytes, cp = cp, enc = enc })
end
do
  -- 0x20..0x7E ASCII (skip control; '|' 0x7c is escaped as "||" so GetStringWidth sees the glyph)
  local b = 32
  while b <= 126 do
    if b == 124 then
      addprobe("U+007C", "||", b, "ascii-esc")
    else
      addprobe(string.format("U+%04X", b), string.char(b), b, "ascii")
    end
    b = b + 1
  end
  -- 0x80..0xFF raw single bytes (codepage hypothesis)
  b = 128
  while b <= 255 do
    addprobe(string.format("byte-%02X", b), string.char(b), b, "byte")
    b = b + 1
  end
  -- U+00A0..U+017F as UTF-8 (Latin-1 Supplement + Latin Extended-A; UTF-8 hypothesis)
  local cp = 160
  while cp <= 383 do
    addprobe(string.format("U+%04X", cp), utf8(cp), cp, "utf8")
    cp = cp + 1
  end
  -- curated higher punctuation WoW UIs actually use
  local CUR = { 8211, 8212, 8216, 8217, 8220, 8221, 8226, 8230, 8482, 8364,
                169, 174, 8249, 8250 }
  for _, c in ipairs(CUR) do addprobe(string.format("U+%04X", c), utf8(c), c, "utf8") end
end

-- ASCII printable kerning-pair alphabet (0x20..0x7E, '|' excluded to avoid escape ambiguity)
local KCHARS = {}
do
  local b = 32
  while b <= 126 do
    if b ~= 124 then table.insert(KCHARS, b) end
    b = b + 1
  end
end

-- ---------------------------------------------------------------------------------------------
-- measuring rig
-- ---------------------------------------------------------------------------------------------
local holder = CreateFrame("Frame", "BenillaReCaptureHolder", UIParent)
holder:SetWidth(1); holder:SetHeight(1); holder:SetPoint("CENTER", UIParent, "CENTER", 0, 0)
local fs = holder:CreateFontString(nil, "BACKGROUND")

local function setfont(path, size, flagstr)
  -- returns true if the face loaded at this size/flags
  return fs:SetFont(path, size, flagstr)
end

local function width(s)
  fs:SetText(s)
  return fs:GetStringWidth()
end

local function fnum(x)
  if x == nil then return "nil" end
  return string.format("%.17g", x)
end

-- discover (face, size, flags) actually used by live font objects + the distinct size set
local function discover()
  local G = getfenv(0)
  local sizeset, faceset, used = {}, {}, {}
  for name, obj in pairs(G) do
    if type(obj) == "table" and type(obj.GetFont) == "function" then
      local ok, p, sz, fl = pcall(obj.GetFont, obj)
      if ok and type(sz) == "number" and sz > 0 then
        sizeset[floor(sz + 0.5)] = true
        if type(p) == "string" then
          faceset[p] = true
          table.insert(used, { name = name, path = p, size = sz, flags = fl or "" })
        end
      end
    end
  end
  local sizes = {}
  for s in pairs(sizeset) do table.insert(sizes, s) end
  table.sort(sizes)
  return sizes, faceset, used
end

-- ---------------------------------------------------------------------------------------------
-- capture phases (each runs inside one OnUpdate tick via the job queue)
-- ---------------------------------------------------------------------------------------------
local DB           -- alias to BenillaReCaptureDB
local faces = {}   -- list of loadable face paths (key)
local sizes = {}   -- discovered real in-game sizes

local function facekey(p) return p end

local function capture_singles(facep, size, flagstr)
  local tbl = {}
  if not setfont(facep, size, flagstr) then return nil end
  local n = table.getn(PROBES)
  local i = 1
  while i <= n do
    tbl[i] = fnum(width(PROBES[i].bytes))
    i = i + 1
  end
  return tbl
end

-- pairs: probe the full ASCII matrix, STORE ONLY nonzero kern deltas (delta = w(LR)-w(L)-w(R)).
-- also store the singles-on-this-config (w(L)) needed to interpret deltas, plus the probe counts
-- (so completeness is provable: "N probed, M nonzero, all others additive"). Chunked ONE LEFT-CHAR
-- per tick (~95 measurements) so it stays smooth on a slow client; accumulators are
-- keyed by face:size, finalized into DB.kerning on the last row.
local pairacc = {}  -- [facekey..":"..size] = { single={}, deltas={}, probed, nonzero }

local function capture_pairs_row(facep, size, flagstr, leftIdx)
  if not setfont(facep, size, flagstr) then return end
  local key = facekey(facep) .. ":" .. size
  local acc = pairacc[key]
  if not acc then
    acc = { single = {}, deltas = {}, probed = 0, nonzero = 0 }
    local m = table.getn(KCHARS)
    local i = 1
    while i <= m do acc.single[KCHARS[i]] = width(string.char(KCHARS[i])); i = i + 1 end
    pairacc[key] = acc
  end
  local L = KCHARS[leftIdx]
  local wl = acc.single[L]
  local m = table.getn(KCHARS)
  local j = 1
  while j <= m do
    local R = KCHARS[j]
    local d = width(string.char(L, R)) - wl - acc.single[R]
    acc.probed = acc.probed + 1
    if d > 0.00001 or d < -0.00001 then
      acc.nonzero = acc.nonzero + 1
      table.insert(acc.deltas, { l = L, r = R, d = fnum(d) })
    end
    j = j + 1
  end
  if leftIdx >= m then
    DB.kerning[facekey(facep)] = DB.kerning[facekey(facep)] or {}
    DB.kerning[facekey(facep)][size] = { probed = acc.probed, nonzero = acc.nonzero, deltas = acc.deltas }
    pairacc[key] = nil  -- release
  end
end

local function capture_pangrams(facep, size, flagstr)
  if not setfont(facep, size, flagstr) then return nil end
  local out = {}
  for k, txt in ipairs(PANGRAMS) do
    out[k] = { text = txt, w = fnum(width(txt)) }
  end
  return out
end

local function capture_random()
  local out = {}
  for _, seed in ipairs(CFG.random_seeds) do
    local rec = { seed = seed, unit = {}, mn = {} }
    math.randomseed(seed)
    for k = 1, CFG.random_count do rec.unit[k] = fnum(math.random()) end     -- no-arg: (rand()%32767)/32767
    math.randomseed(seed)
    for k = 1, CFG.random_count do rec.mn[k] = math.random(1, 1000000) end   -- m..n integer path
    table.insert(out, rec)
  end
  return out
end

local function capture_globals()
  local G = getfenv(0)
  local keys = {}
  for name, v in pairs(G) do
    table.insert(keys, { name = tostring(name), t = type(v) })
  end
  table.sort(keys, function(a, b) return a.name < b.name end)
  return { count = table.getn(keys), where = "in-world", keys = keys }
end

-- ---------------------------------------------------------------------------------------------
-- job queue / driver
-- ---------------------------------------------------------------------------------------------
local jobs = {}
local jobi = 0
local status = holder:CreateFontString(nil, "OVERLAY")
status:SetFont("Fonts\\FRIZQT__.TTF", 16, "OUTLINE")  -- explicit font: 1.12-safe (no inherit-template dependency)
status:SetPoint("TOP", UIParent, "TOP", 0, -120)

local function enqueue(desc, fn) table.insert(jobs, { desc = desc, fn = fn }) end

local function build_jobs()
  -- phase 0: meta + discovery already done before queueing
  -- singles
  for _, facep in ipairs(faces) do
    for _, size in ipairs(sizes) do
      for _, fl in ipairs(CFG.flags_singles) do
        local fp, sz, flg = facep, size, fl
        enqueue("singles " .. fp .. " @" .. sz .. " [" .. flg .. "]", function()
          local r = capture_singles(fp, sz, flg)
          if r then
            DB.glyphs[facekey(fp)] = DB.glyphs[facekey(fp)] or {}
            DB.glyphs[facekey(fp)][sz] = DB.glyphs[facekey(fp)][sz] or {}
            DB.glyphs[facekey(fp)][sz][flg == "" and "normal" or flg] = r
          end
        end)
      end
    end
  end
  -- pairs (chunked one left-char per job) — full O(n^2) matrix only at a few sizes (CFG.pair_max_sizes)
  if CFG.do_pairs then
    local nL = table.getn(KCHARS)
    local pair_sizes = cap_spread(sizes, CFG.pair_max_sizes)
    for _, facep in ipairs(faces) do
      for _, size in ipairs(pair_sizes) do
        for _, fl in ipairs(CFG.flags_pairs) do
          local li = 1
          while li <= nL do
            local fp, sz, flg, idx = facep, size, fl, li
            enqueue(string.format("pairs %s @%s row %d/%d", fp, sz, idx, nL), function()
              capture_pairs_row(fp, sz, flg, idx)
            end)
            li = li + 1
          end
        end
      end
    end
  end
  -- pangrams
  if CFG.do_pangrams then
    for _, facep in ipairs(faces) do
      for _, size in ipairs(sizes) do
        local fp, sz = facep, size
        enqueue("pangrams " .. fp .. " @" .. sz, function()
          local r = capture_pangrams(fp, sz, "")
          if r then
            DB.pangrams[facekey(fp)] = DB.pangrams[facekey(fp)] or {}
            DB.pangrams[facekey(fp)][sz] = r
          end
        end)
      end
    end
  end
  -- random + globals
  if CFG.do_random then enqueue("math.random", function() DB.random = capture_random() end) end
  if CFG.do_globals then enqueue("in-world _G", function() DB.globals = capture_globals() end) end
end

local running = false
local function tick()
  if not running then return end
  if jobi >= table.getn(jobs) then
    running = false
    holder:SetScript("OnUpdate", nil)
    status:SetText("|cff00ff00BenillaReCapture COMPLETE|r — /console reloadui (or /logout) to write SavedVariables")
    DEFAULT_CHAT_FRAME:AddMessage("|cff00ff00[BenillaReCapture]|r complete. Type /reload (or /logout) to write BenillaReCaptureDB.")
    return
  end
  jobi = jobi + 1
  local job = jobs[jobi]
  status:SetText(string.format("BenillaReCapture %d/%d: %s", jobi, table.getn(jobs), job.desc))
  -- run the job; guard so one bad face/size never aborts the whole sweep
  local ok, err = pcall(job.fn)
  if not ok then
    DEFAULT_CHAT_FRAME:AddMessage("|cffff0000[BenillaReCapture]|r job " .. jobi .. " error: " .. tostring(err))
  end
end

local function start()
  if running then DEFAULT_CHAT_FRAME:AddMessage("[BenillaReCapture] already running"); return end
  BenillaReCaptureDB = {}
  DB = BenillaReCaptureDB

  -- discovery
  local dsizes, dfaceset, dused = discover()
  sizes = dsizes
  if table.getn(sizes) == 0 then
    -- defensive fallback (should not happen in-world): a representative real-size set
    sizes = { 10, 11, 12, 13, 14, 16, 18, 24, 30 }
  end
  -- bound memory: cap to a representative spread of real sizes (see CFG.max_sizes)
  sizes = cap_spread(sizes, CFG.max_sizes)
  -- faces: candidates ∪ live-used, de-duplicated, keep only those that load at the first size
  local faceseen = {}
  faces = {}
  local function tryface(p)
    if faceseen[p] then return end
    faceseen[p] = true
    if setfont(p, sizes[1], "") then table.insert(faces, p) end
  end
  for _, p in ipairs(CFG.candidate_faces) do tryface(p) end
  for p in pairs(dfaceset) do tryface(p) end

  -- meta header
  local okdate, dt = pcall(function() return date("%Y-%m-%d %H:%M:%S") end)
  DB.meta = {
    note      = "BenillaReCapture: glyph metrics, math.random draws, in-world globals",
    build     = "1.12.1.5875",
    date      = okdate and dt or "n/a",
    locale    = GetLocale(),
    screenW   = GetScreenWidth(), screenH = GetScreenHeight(),
    gxResolution = GetCVar("gxResolution"), gxWindow = GetCVar("gxWindow"),
    uiScale_cvar = GetCVar("uiScale"),
    uiparent_effscale = UIParent:GetEffectiveScale(),
    holder_effscale   = holder:GetEffectiveScale(),
    fs_effscale       = holder:GetEffectiveScale(),  -- fs inherits holder's scale; 1.12 FontString has no GetEffectiveScale
    sizes = sizes, faces = faces,
    used_font_objects = dused,
    probe_count = table.getn(PROBES),
    width_units = "FontString:GetStringWidth() local units; normalize by *_effscale for screen px",
    encoding_note = "probes carry .enc (ascii/byte/utf8); fixture is (input-bytes -> width)",
  }
  -- probe definitions, index-aligned with DB.glyphs[face][size][flags]
  DB.probes = {}
  for i, p in ipairs(PROBES) do
    DB.probes[i] = { label = p.label, cp = p.cp, enc = p.enc, bytes_len = string.len(p.bytes) }
  end
  DB.glyphs, DB.kerning, DB.pangrams = {}, {}, {}

  jobs = {}; jobi = 0
  build_jobs()
  running = true
  holder:SetScript("OnUpdate", tick)
  DEFAULT_CHAT_FRAME:AddMessage(string.format(
    "|cff00ff00[BenillaReCapture]|r started: %d faces x %d sizes, %d probes, %d jobs. Watch the bar; do NOT log out until COMPLETE.",
    table.getn(faces), table.getn(sizes), table.getn(PROBES), table.getn(jobs)))
end

-- slash command
SLASH_BENILLARECAPTURE1 = "/brecapture"
SlashCmdList["BENILLARECAPTURE"] = function() start() end

-- gentle auto-hint on login (does NOT auto-start; the user triggers when ready & at rest)
local boot = CreateFrame("Frame")
boot:RegisterEvent("PLAYER_LOGIN")
boot:SetScript("OnEvent", function()
  DEFAULT_CHAT_FRAME:AddMessage("|cff00ff00[BenillaReCapture]|r loaded. Stand still, then type /brecapture")
end)
