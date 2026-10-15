//! The check's control: a clean fixture passes, and each rule, planted alone in a copy of it,
//! yields exactly the problems expected of it and nothing else.

use super::run;
use crate::tree::{load, Memory};
use std::collections::BTreeMap;
use std::path::Path;

type Files = BTreeMap<String, String>;

fn clean() -> Files {
    load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/clean"))
}

fn problems(files: Files) -> Vec<String> {
    let checked = run(&Memory(files)).expect("an in-memory tree reads");
    let mut lines: Vec<String> = checked.problems.iter().map(|p| p.to_string()).collect();
    lines.sort();
    lines
}

/// Replaces the one occurrence of `from` in `file`.
fn swap(files: &mut Files, file: &str, from: &str, to: &str) {
    let text = files
        .get_mut(file)
        .unwrap_or_else(|| panic!("{file} is in the fixture"));
    assert_eq!(
        text.matches(from).count(),
        1,
        "{from:?} occurs once in {file}"
    );
    *text = text.replacen(from, to, 1);
}

fn more(n: usize) -> String {
    " more".repeat(n)
}

/// One planted violation: what it breaks, how, and the exact problems it must yield.
struct Plant {
    rule: &'static str,
    plant: Box<dyn Fn(&mut Files)>,
    expect: Vec<String>,
}

fn plant(rule: &'static str, plant: impl Fn(&mut Files) + 'static, expect: &[&str]) -> Plant {
    Plant {
        rule,
        plant: Box::new(plant),
        expect: expect.iter().map(|s| s.to_string()).collect(),
    }
}

fn edit(file: &'static str, from: &'static str, to: &'static str) -> impl Fn(&mut Files) {
    move |files| swap(files, file, from, to)
}

const F: &str = "symbols/functions.tsv";
const G: &str = "symbols/globals.tsv";
const S: &str = "symbols/structs.tsv";
const D: &str = "symbols/fields.tsv";
const DOC: &str = "systems/demo.md";

const STEP: &str = "0x500030\tfloat\ts_step\t\tdemo\tHolds 8.0, the step\tinferred\tnone\t\n";
const HELPER: &str =
    "0x402100\t8\tUnusedHelper\t\tdemo\tunreached\t\tReturns zero\tinferred\tnone\tdemo#unreached-functions\t\n";
const SPARE: &str = "CSpare\t4\t\tdemo\tHolds a spare dword\tinferred\tnone\t\n";
const COUNTER: &str =
    "CDemo\t0x0004\t4\tuint32_t\tm_counter\tdemo\tHolds the counter\tverified\tone\tdemo#state-advance\n";
const FUNC: &str =
    "luaL_reg\t0x0004\t4\tvoid *\tfunc\tlua\tHolds the C function\tinferred\tnone\t\n";
const OVERVIEW: &str = "A small system for the check's own control.";
const CTOR_SHAPE: &str = "Addresses: `0x402040`\nConfidence: verified\nProof: two\nTest: none\n";

fn plants() -> Vec<Plant> {
    vec![
        // The tables' formats.
        plant(
            "a table is missing",
            |f: &mut Files| {
                f.remove(D);
            },
            &["symbols/fields.tsv:1: missing"],
        ),
        plant(
            "docs/METHOD.md is missing",
            |f: &mut Files| {
                f.remove("docs/METHOD.md");
            },
            &[
                "docs/METHOD.md:1: missing",
                "symbols/functions.tsv:2: finding METHOD#the-harness does not resolve to an entry",
            ],
        ),
        plant(
            "the header",
            edit(D, "struct\toffset\t", "struct\tplace\t"),
            &["symbols/fields.tsv:1: header is not struct offset size type name system meaning confidence proof finding"],
        ),
        plant(
            "the final newline",
            edit(D, "\tHolds the C function\tinferred\tnone\t\n", "\tHolds the C function\tinferred\tnone\t"),
            &["symbols/fields.tsv:5: does not end with a newline"],
        ),
        plant(
            "the column count",
            edit(D, FUNC, "luaL_reg\t0x0004\t4\tvoid *\tfunc\tlua\tHolds the C function\tinferred\tnone\t\textra\n"),
            &["symbols/fields.tsv:5: 11 columns, not 10"],
        ),
        plant(
            "sorting",
            |f: &mut Files| {
                let name = "luaL_reg\t0x0000\t4\tconst char *\tname\tlua\tHolds the function's name\tinferred\tnone\t\n";
                swap(f, D, &format!("{name}{FUNC}"), &format!("{FUNC}{name}"));
            },
            &["symbols/fields.tsv:5: out of order or duplicate key"],
        ),
        plant(
            "a struct appears twice",
            edit(S, SPARE, "CSpare\t4\t\tdemo\tHolds a spare dword\tinferred\tnone\t\nCSpare\t4\t\tdemo\tHolds a spare dword\tinferred\tnone\t\n"),
            &[
                "symbols/structs.tsv:4: out of order or duplicate key",
                "symbols/structs.tsv:4: struct CSpare appears twice",
            ],
        ),
        // Functions and globals.
        plant(
            "the address format",
            edit(G, "0x500030\tfloat", "0x50003G\tfloat"),
            &["symbols/globals.tsv:4: address '0x50003G' is not 0x and six lowercase hex digits"],
        ),
        plant(
            "name_from",
            edit(G, "s_step\t\tdemo", "s_step\tvmangos\tdemo"),
            &["symbols/globals.tsv:4: name_from is idb or empty"],
        ),
        plant(
            "a name used twice",
            edit(G, "\ts_step\t", "\ts_table\t"),
            &["symbols/globals.tsv:4: name s_table is also used at 0x500014"],
        ),
        plant(
            "an address with two rows",
            edit(G, "proof\tfinding\n", "proof\tfinding\n0x401000\tfloat\ts_copy\t\tdemo\tHolds a copy\tinferred\tnone\t\n"),
            &["symbols/globals.tsv:2: address 0x401000 also has a row at symbols/functions.tsv:2"],
        ),
        plant(
            "a global inside another",
            edit(G, "uint32_t[4]\ts_table", "uint32_t[8]\ts_table"),
            &["symbols/globals.tsv:4: 0x500030 lies inside 0x500014"],
        ),
        plant(
            "functions inside a long one, the second past a short one",
            edit(F, "0x402000\t64\t", "0x402000\t512\t"),
            &[
                "symbols/functions.tsv:5: 0x402040 lies inside 0x402000",
                "symbols/functions.tsv:6: 0x402100 lies inside 0x402000",
            ],
        ),
        // The common columns.
        plant(
            "an empty meaning",
            edit(G, "Holds 8.0, the step", ""),
            &["symbols/globals.tsv:4: meaning is empty"],
        ),
        plant(
            "the meaning's length",
            |f: &mut Files| swap(f, G, "Holds 8.0, the step", &format!("Holds 8.0, the step{}", more(21))),
            &["symbols/globals.tsv:4: meaning is 124 characters, over 120"],
        ),
        plant(
            "the meaning's final period",
            edit(G, "Holds 8.0, the step", "Holds 8.0, the step."),
            &["symbols/globals.tsv:4: meaning ends with a period"],
        ),
        plant(
            "the meaning's verb",
            edit(G, "Holds 8.0, the step", "Hold 8.0, the step"),
            &["symbols/globals.tsv:4: meaning does not start with a verb"],
        ),
        plant(
            "a banned word in a meaning",
            edit(G, "Holds 8.0, the step", "Holds 8.0, maybe the step"),
            &["symbols/globals.tsv:4: meaning has a banned word: maybe"],
        ),
        plant(
            "a question mark in a meaning",
            edit(G, "Holds 8.0, the step", "Holds 8.0, the step?"),
            &["symbols/globals.tsv:4: meaning has a banned word: ?"],
        ),
        plant(
            "the system",
            edit(G, "s_step\t\tdemo", "s_step\t\tDemo"),
            &["symbols/globals.tsv:4: system 'Demo' is not a lowercase word"],
        ),
        plant(
            "a system with no doc, no library and no listing",
            edit(G, "s_step\t\tdemo", "s_step\t\tdemos"),
            &["symbols/globals.tsv:4: system 'demos' has no doc, is no library and is not listed in symbols/README.md"],
        ),
        plant(
            "a listed system that has a doc",
            edit("symbols/README.md", "- `client`:", "- `demo`: the demo\n- `client`:"),
            &["symbols/README.md:7: demo is listed but has a doc"],
        ),
        plant(
            "the list of systems without a doc is missing",
            edit("symbols/README.md", "## Systems without a doc", "## Other systems"),
            &[
                "symbols/README.md:1: has no \"## Systems without a doc\" section",
                "symbols/functions.tsv:2: system 'client' has no doc, is no library and is not listed in symbols/README.md",
            ],
        ),
        plant(
            "the confidence",
            edit(G, "the step\tinferred\tnone", "the step\tcertain\tnone"),
            &["symbols/globals.tsv:4: confidence 'certain' is not verified or inferred"],
        ),
        plant(
            "inferred with a proof",
            edit(G, "the step\tinferred\tnone", "the step\tinferred\tone"),
            &["symbols/globals.tsv:4: inferred takes proof none"],
        ),
        plant(
            "verified without a proof",
            edit(F, "verified\tone\tMETHOD#the-harness", "verified\tnone\tMETHOD#the-harness"),
            &["symbols/functions.tsv:2: verified takes proof one or two"],
        ),
        plant(
            "verified without a finding",
            edit(G, STEP, "0x500030\tfloat\ts_step\t\tdemo\tHolds 8.0, the step\tverified\tone\t\n"),
            &["symbols/globals.tsv:4: verified needs a finding"],
        ),
        plant(
            "a finding that resolves to no entry",
            edit(G, STEP, "0x500030\tfloat\ts_step\t\tdemo\tHolds 8.0, the step\tinferred\tnone\tdemo#nowhere\n"),
            &["symbols/globals.tsv:4: finding demo#nowhere does not resolve to an entry"],
        ),
        plant(
            "a finding that does not mention its row",
            edit(G, STEP, "0x500030\tfloat\ts_step\t\tdemo\tHolds 8.0, the step\tinferred\tnone\tdemo#the-constructor\n"),
            &["symbols/globals.tsv:4: finding demo#the-constructor does not mention 0x500030"],
        ),
        plant(
            "a struct's common columns",
            edit(S, SPARE, "CSpare\t4\t\tdemo\tHolds a spare dword\tverified\tone\t\n"),
            &["symbols/structs.tsv:3: verified needs a finding"],
        ),
        plant(
            "a field's common columns",
            edit(D, FUNC, "luaL_reg\t0x0004\t4\tvoid *\tfunc\tlua\tHolds the C function\tinferred\tnone\tlua#nowhere\n"),
            &["symbols/fields.tsv:5: finding lua#nowhere does not resolve to an entry"],
        ),
        // Names.
        plant(
            "a name's length",
            edit(F, "\tUnusedHelper\t", "\tUnusedHelperThatReturnsZeroWhereverItIsCalledFrom\t"),
            &["symbols/functions.tsv:6: name is 49 characters, over 48"],
        ),
        plant(
            "a library name",
            edit(F, "\t__ftol\t", "\t__ftol!\t"),
            &["symbols/functions.tsv:3: name '__ftol!' is not an identifier"],
        ),
        plant(
            "a function's name",
            edit(F, "\tUnusedHelper\t", "\tunusedHelper\t"),
            &["symbols/functions.tsv:6: name 'unusedHelper' breaks the naming rule for functions"],
        ),
        plant(
            "a global's name",
            edit(G, "\ts_step\t", "\tstep\t"),
            &["symbols/globals.tsv:4: name 'step' breaks the naming rule for globals"],
        ),
        plant(
            "a struct's name",
            edit(S, "CSpare\t4\t", "CSpare_\t4\t"),
            &["symbols/structs.tsv:3: struct name 'CSpare_' breaks the naming rule"],
        ),
        plant(
            "a field's name",
            edit(D, "\tm_counter\t", "\tcounter\t"),
            &["symbols/fields.tsv:3: field name 'counter' breaks the naming rule"],
        ),
        // Function columns.
        plant(
            "a function's size",
            edit(F, "0x401100\t20\t", "0x401100\t0x14\t"),
            &[
                "symbols/functions.tsv:3: size '0x14' is not a positive decimal",
                "systems/demo.md:24: 0x401100 resolves to no row",
            ],
        ),
        plant(
            "a class",
            edit(F, "\tdemo\tunreached\t", "\tdemo\tdead\t"),
            &["symbols/functions.tsv:6: class 'dead' is not one of boundary, host, orchestration, primitive, unreached"],
        ),
        plant(
            "a class in a system with a doc",
            edit(F, "\tdemo\torchestration\t", "\tdemo\t\t"),
            &["symbols/functions.tsv:5: class is required: demo has a doc"],
        ),
        plant(
            "an unreached function's test",
            edit(F, HELPER, "0x402100\t8\tUnusedHelper\t\tdemo\tunreached\t\tReturns zero\tinferred\tnone\tdemo#unreached-functions\tdemo::demo_unused\n"),
            &["symbols/functions.tsv:6: an unreached function has no test"],
        ),
        plant(
            "a test's format",
            edit(F, "\tdemo::demo_advance\n", "\tdemo_advance\n"),
            &["symbols/functions.tsv:4: test 'demo_advance' is not crate::test_fn"],
        ),
        plant(
            "a signature's length",
            |f: &mut Files| {
                let long = format!("uint32_t __thiscall (CDemo *this{})", ", int x".repeat(19));
                swap(f, F, "uint32_t __thiscall (CDemo *this)", &long);
            },
            &["symbols/functions.tsv:4: signature is over 160 characters"],
        ),
        // Types, structs and fields.
        plant(
            "a global's type",
            edit(G, "0x500030\tfloat", "0x500030\tvec3"),
            &["symbols/globals.tsv:4: type 'vec3' has no known size"],
        ),
        plant(
            "a struct's size",
            edit(S, "CSpare\t4\t", "CSpare\tfour\t"),
            &["symbols/structs.tsv:3: size 'four' is not a positive decimal"],
        ),
        plant(
            "a struct's vtable",
            edit(S, "CSpare\t4\t\t", "CSpare\t4\t0x500100\t"),
            &["symbols/structs.tsv:3: vtable 0x500100 is not a row of globals.tsv"],
        ),
        plant(
            "a field of no struct",
            edit(D, "luaL_reg\t0x0000", "CGone\t0x0000\t4\tfloat\tm_x\tdemo\tHolds x\tinferred\tnone\t\nluaL_reg\t0x0000"),
            &["symbols/fields.tsv:4: struct CGone is not a row of structs.tsv"],
        ),
        plant(
            "a field's offset",
            edit(D, "CDemo\t0x0004\t", "CDemo\t0x04\t"),
            &["symbols/fields.tsv:3: offset is 0x and four hex digits, size a positive decimal"],
        ),
        plant(
            "a field past its struct",
            edit(D, COUNTER, "CDemo\t0x0004\t8\tuint64_t\tm_counter\tdemo\tHolds the counter\tverified\tone\tdemo#state-advance\n"),
            &["symbols/fields.tsv:3: field ends past the 8-byte struct"],
        ),
        plant(
            "a field's type",
            edit(D, "\tuint32_t\tm_counter", "\tcounter_t\tm_counter"),
            &["symbols/fields.tsv:3: type 'counter_t' has no known size"],
        ),
        plant(
            "a field's size against its type",
            edit(D, "CDemo\t0x0004\t4\t", "CDemo\t0x0004\t2\t"),
            &["symbols/fields.tsv:3: size 2 differs from the size of uint32_t, 4"],
        ),
        plant(
            "a field name twice in a struct",
            edit(D, "\tvoid *\tfunc\t", "\tvoid *\tname\t"),
            &["symbols/fields.tsv:5: field name appears twice in luaL_reg"],
        ),
        plant(
            "overlapping fields",
            edit(D, "CDemo\t0x0004\t", "CDemo\t0x0002\t"),
            &["symbols/fields.tsv:3: field overlaps another field of CDemo"],
        ),
        // A doc as a whole.
        plant(
            "a doc's length",
            |f: &mut Files| {
                let row = format!("| counter |{} |\n", " word".repeat(340));
                swap(f, DOC, "| counter | 4 |\n", &row.repeat(12));
            },
            &["systems/demo.md:1: 4271 words, over 4000; split by topic"],
        ),
        plant(
            "a doc's title line",
            edit(DOC, "# demo\n", "demo\n"),
            &["systems/demo.md:1: the doc starts with \"# <system>\""],
        ),
        plant(
            "the overview's length",
            |f: &mut Files| swap(f, DOC, OVERVIEW, &format!("{OVERVIEW}{}", more(179))),
            &["systems/demo.md:1: overview is 201 words, over 200"],
        ),
        plant(
            "the Roots line",
            edit(DOC, "Roots: `0x402000`,\n`0x402040`\n\n", ""),
            &["systems/demo.md:1: no Roots line before the first entry"],
        ),
        plant(
            "a root the system does not own",
            edit(DOC, "Roots: `0x402000`,", "Roots: `0x401000`, `0x402000`,"),
            &["systems/demo.md:7: root 0x401000 is not a function row of demo"],
        ),
        plant(
            "a code block's length",
            edit(DOC, "add eax, [ecx+4]\nret\n", "nop\nnop\nnop\nnop\nnop\nnop\nnop\nnop\nret\n"),
            &["systems/demo.md:26: code block over 8 lines"],
        ),
        plant(
            "a hedge",
            edit(DOC, OVERVIEW, "A small system, probably for the check's own control."),
            &["systems/demo.md:3: hedge: 'probably'"],
        ),
        plant(
            "shouting",
            edit(DOC, OVERVIEW, "NOTE: a small system for the check's own control."),
            &["systems/demo.md:3: shouting: 'NOTE:'"],
        ),
        plant(
            "history",
            edit(DOC, OVERVIEW, "A small system that previously served the check's control."),
            &["systems/demo.md:3: history: 'previously'"],
        ),
        plant(
            "the first person",
            edit(DOC, OVERVIEW, "Our small system for the check's own control."),
            &["systems/demo.md:3: first person: 'Our'"],
        ),
        plant(
            "prose rules skip inline code and code blocks",
            |f: &mut Files| {
                swap(f, DOC, OVERVIEW, "A small system for the check's own control, `probably`.");
                swap(f, DOC, "ret\n```", "ret ; we previously did this\n```");
            },
            &[],
        ),
        // A doc's entries, planted in an entry no row links to.
        plant(
            "an entry title's length",
            edit(DOC, "## The constructor\n", "## The constructor of the demo state class\n"),
            &["systems/demo.md:31: title is 7 words, over 6"],
        ),
        plant(
            "an entry's shape",
            edit(DOC, CTOR_SHAPE, "Addresses: `0x402040`\nConfidence: verified\nProof: two\n"),
            &["systems/demo.md:31: no Addresses, Confidence, Proof and Test lines in that order"],
        ),
        plant(
            "an entry's Addresses line",
            edit(DOC, CTOR_SHAPE, "Addresses: 0x402040\nConfidence: verified\nProof: two\nTest: none\n"),
            &["systems/demo.md:35: Addresses is a comma-separated list of `0x……` addresses"],
        ),
        plant(
            "an entry's Confidence line",
            edit(DOC, CTOR_SHAPE, "Addresses: `0x402040`\nConfidence: sure\nProof: two\nTest: none\n"),
            &["systems/demo.md:36: Confidence 'sure' is not verified or inferred"],
        ),
        plant(
            "an entry's Proof line",
            edit(DOC, CTOR_SHAPE, "Addresses: `0x402040`\nConfidence: inferred\nProof: two\nTest: none\n"),
            &["systems/demo.md:37: inferred takes Proof none"],
        ),
        plant(
            "an entry's Test line",
            edit(DOC, CTOR_SHAPE, "Addresses: `0x402040`\nConfidence: verified\nProof: two\nTest: demo::demo_ctor\n"),
            &["systems/demo.md:38: Test is none or a list of `crate::test_fn`"],
        ),
        plant(
            "a claim's length",
            |f: &mut Files| {
                let claim = "`0x402040` stores the vtable and clears the counter.";
                swap(f, DOC, claim, &format!("{claim}{}", more(113)));
            },
            &["systems/demo.md:31: claim is 121 words, over 120"],
        ),
        plant(
            "a claim's table",
            |f: &mut Files| {
                let claim = "`0x402040` stores the vtable and clears the counter.\n";
                let table = format!("{claim}\n| a | b |\n|---|---|\n{}", "| 1 | 2 |\n".repeat(13));
                swap(f, DOC, claim, &table);
            },
            &["systems/demo.md:35: table has 13 rows, over 12"],
        ),
        plant(
            "a derivation's length",
            |f: &mut Files| {
                let derivation = "It stores `0x500000` at `+0x00` and zero at `+0x04`.";
                swap(f, DOC, derivation, &format!("{derivation}{}", more(192)));
            },
            &["systems/demo.md:31: derivation is 201 words, over 200"],
        ),
        // Addresses in any Markdown file.
        plant(
            "a system doc's address without a row",
            edit(DOC, "Nothing reaches `0x402100`.", "Nothing reaches `0x402100` or `0x600000`."),
            &["systems/demo.md:44: 0x600000 resolves to no row"],
        ),
        plant(
            "a system doc's address in the padding after a function",
            edit(DOC, "Nothing reaches `0x402100`.", "Nothing reaches `0x402100`; it ends at `0x402108` and `0x40210f` pads."),
            &[],
        ),
        plant(
            "a system doc's address 16 bytes past a function",
            edit(DOC, "Nothing reaches `0x402100`.", "Nothing reaches `0x402100` or `0x402118`."),
            &["systems/demo.md:44: 0x402118 resolves to no row"],
        ),
        plant(
            "a meaning's address without a row",
            edit(G, "Holds 8.0, the step", "Holds 8.0, the step 0x600000 reads"),
            &["symbols/globals.tsv:4: meaning cites 0x600000, which resolves to no row"],
        ),
        plant(
            "a spec doc's address needs no row",
            edit("README.md", "`0x500018`", "`0x600000`"),
            &[],
        ),
        plant(
            "a spec doc's address outside the image",
            edit("README.md", "`0x500018`", "`0xe00000`"),
            &["README.md:4: 0xe00000 is not an address inside the image"],
        ),
        // A test is not a proof, and anchors.
        plant(
            "a row with proof test",
            edit(F, "\tverified\tone\tdemo#state-advance\tdemo::demo_advance\n", "\tverified\ttest\tdemo#state-advance\tdemo::demo_advance\n"),
            &["symbols/functions.tsv:4: verified takes proof one or two"],
        ),
        plant(
            "an entry with Proof test",
            edit(DOC, "Proof: one\nTest: `demo::demo_advance`", "Proof: test\nTest: `demo::demo_advance`"),
            &["systems/demo.md:20: verified takes Proof one or two"],
        ),
        plant(
            "two headings with one anchor",
            edit(DOC, "## Unreached functions\n", "## The Constructor\n\nNothing else.\n\n## Unreached functions\n"),
            &[
                "systems/demo.md:42: heading's anchor #the-constructor is also line 31's",
                "systems/demo.md:42: no Addresses, Confidence, Proof and Test lines in that order",
            ],
        ),
        plant(
            "a heading-like line in a code block is no heading",
            edit(DOC, "add eax, [ecx+4]\n", "# demo\n"),
            &[],
        ),
        // Citations in prose.
        plant(
            "a cited entry that exists, by bare doc name and by link",
            edit(DOC, "returns the sum.\n", "returns the sum (`demo#state-advance`, [here](demo.md#state-advance)).\n"),
            &[],
        ),
        plant(
            "a cited entry that does not exist",
            edit(DOC, "returns the sum.\n", "returns the sum (`demo#state-step`).\n"),
            &["systems/demo.md:12: demo#state-step cites no entry"],
        ),
        plant(
            "a link to an entry that does not exist",
            edit(DOC, "returns the sum.\n", "returns the sum ([there](../other/demo.md#state-advance)).\n"),
            &["systems/demo.md:12: ../other/demo.md#state-advance links to no entry"],
        ),
        // The tables' headers.
        plant(
            "a table's header",
            edit("tables/demo.tsv", "id\tname\tname_address\n", "id\tname\n"),
            &["tables/demo.tsv:1: header is not id name name_address, as tables/README.md defines it"],
        ),
        plant(
            "the header of a table whose columns are another's",
            edit("tables/copy.tsv", "id\tname\tname_address\n", "id\tname_address\tname\n"),
            &["tables/copy.tsv:1: header is not id name name_address, as tables/README.md defines it"],
        ),
        plant(
            "a table the README does not define",
            |f: &mut Files| {
                f.insert("tables/extra.tsv".to_string(), "id\n".to_string());
            },
            &["tables/extra.tsv:1: tables/README.md defines no columns for it"],
        ),
        plant(
            "a table the README defines that is missing",
            |f: &mut Files| {
                f.remove("tables/copy.tsv");
            },
            &["tables/README.md:12: copy.tsv is defined here but missing"],
        ),
        plant(
            "the rules' own README is a spec doc",
            edit(
                "systems/README.md",
                "The shape of a doc.",
                "The shape of a doc, as at `0x3fffff`.",
            ),
            &["systems/README.md:3: 0x3fffff is not an address inside the image"],
        ),
    ]
}

#[test]
fn the_clean_fixture_passes() {
    assert_eq!(problems(clean()), Vec::<String>::new());
}

#[test]
fn a_class_name_keeps_the_client_suffix() {
    let mut files = clean();
    for file in [S, D, F, G] {
        let text = files.get_mut(file).expect("in the fixture");
        *text = text.replace("CDemo", "CDemo_C");
    }
    swap(&mut files, DOC, "`CDemo`", "`CDemo_C`");
    assert_eq!(problems(files), Vec::<String>::new());
}

#[test]
fn a_suffix_belongs_to_the_class_alone() {
    for good in [
        "CDemo_C::Run",
        "CDemo_C::CDemo_C",
        "CDemo_C::~CDemo_C",
        "A::B_C::B_C",
    ] {
        assert!(super::method_name(good), "{good} is refused");
    }
    for bad in [
        "CDemo::Run_Fast",
        "CDemo_C::Run_C",
        "CDemo_C::Other_C",
        "CDemo_C::~Other_C",
    ] {
        assert!(!super::method_name(bad), "{bad} is accepted");
    }
}

#[test]
fn each_planted_violation_is_caught_alone() {
    let mut failures = Vec::new();
    for plant in plants() {
        let mut files = clean();
        (plant.plant)(&mut files);
        let mut expect = plant.expect.clone();
        expect.sort();
        let found = problems(files);
        if found != expect {
            failures.push(format!(
                "{}:\n  expected {expect:#?}\n  found {found:#?}",
                plant.rule
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_named_tests_come_from_the_table_and_the_docs() {
    let checked = run(&Memory(clean())).expect("reads");
    assert_eq!(
        checked.named_tests(),
        [
            (
                "symbols/functions.tsv:4".to_string(),
                "demo::demo_advance".to_string()
            ),
            (
                "systems/demo.md:21".to_string(),
                "demo::demo_advance".to_string()
            ),
        ]
    );
}

#[test]
fn the_summary_counts_what_was_read() {
    let checked = run(&Memory(clean())).expect("reads");
    assert_eq!(
        checked.summary(),
        "5 functions, 3 globals, 3 structs, 4 fields, 1 system docs"
    );
}
