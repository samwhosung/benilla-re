# ui: the profession window

The profession window (`TradeSkillFrame.cpp`, `0x4fc870` to `0x50031e`) is documented in `tradeskill/`. Where each part lives:

| part | doc |
|---|---|
| state, records, build, difficulty, available count, slot bits, order, filters, open and close | `tradeskill/recipes` |
| the Lua functions that read a row, set the filters and collapse groups | `tradeskill/bindings` |
| `DoTradeSkill` and the repeat latch | `tradeskill/cast` |
| `GetTradeSkillItemStats` | `tradeskill/itemstats` |
| the recipe icon | `ui/spellbook#recipe-icons` |

Roots: `0x4fc910`, `0x4fca20`, `0x4fd180`, `0x4fd380`, `0x4fd540`, `0x4fd820`, `0x4fdc50`, `0x4fdd40`, `0x4ff5c0`, `0x500280`
