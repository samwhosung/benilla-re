# demo

A small system for the check's own control.

The object file runs from `0x402000` through `0x402100`, and its state is one `CDemo`.

Roots: `0x402000`,
`0x402040`

## State advance

`0x402000` adds the step to the counter and returns the sum.

| field | offset |
|---|---|
| counter | 4 |

Addresses: `0x402000`, `0x402040`, `0x500000`, `0x500014`
Confidence: verified
Proof: one
Test: `demo::demo_advance`

The add is at `0x402010`, inside `0x402000`. The constructor `0x402040` stores the vtable
`0x500000`; the step is converted with `__ftol` (`0x401100`).

```
add eax, [ecx+4]
ret
```

## The constructor

`0x402040` stores the vtable and clears the counter.

Addresses: `0x402040`
Confidence: verified
Proof: two
Test: none

It stores `0x500000` at `+0x00` and zero at `+0x04`.

## Unreached functions

Nothing reaches `0x402100`.

Addresses: `0x402100`
Confidence: inferred
Proof: none
Test: none

A census of calls and stored addresses finds none; the control finds the calls to `0x402000`.
