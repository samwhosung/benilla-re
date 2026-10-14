# ui: the skills list and its verbs

The skills window's list (`SkillInfo.cpp`, `0x4d2ab0` to `0x4d4090`) and its Lua functions are documented in `tradeskill/skills`: the skill slots, the list state, the rebuild, order and visibility, rows, selection, collapse, abandon and skill points. The skill field block and the events that rebuild the list are `objectlayer/playerfields#skill-changes`; the point counters are `ui/talents`.

Roots: `0x4d2cb0`, `0x4d2ed0`, `0x4d3070`, `0x4d3610`, `0x4d32d0`, `0x4d3480`, `0x4d3d40`, `0x4d3e50`, `0x4d3ab0`
