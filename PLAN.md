# vcikit

> **Status (2026-11-22): implemented — the J2534 backend is built; the native-VCX backend is the
> blocked second.** The owner redirected `vcikit` from a skeleton to a working transport abstraction
> with two backends. **Built:** the `RawBackend` trait and `BackendKind` selector, the `LinkConfig` /
> `CanId` types (the raw-mode ComParam subset), the 4-byte CAN-id raw-mode PDU codec, the
> platform-independent J2534 message model, and the **J2534 pass-through backend** (`vcikit-j2534`
> member) — registry discovery of a DLL under `PassThruSupport.04.04`, loading it, and driving the
> J2534-1 ISO 15765 API (`Open → Connect → flow-control StartMsgFilter → WriteMsgs/ReadMsgs`). The
> crate is now a workspace: the abstraction crate keeps `unsafe` forbidden, and the unsafe DLL FFI is
> isolated to one audited module in the member. **Still blocked:** the **native-VCX** backend over
> `vcxtoolkit`'s codecs — `BackendKind::NativeVcx` is exposed, but the impl waits on `vcxtoolkit`
> growing the raw-mode channel bring-up (see "Dependencies and wave"). The design below is unchanged;
> the "Trigger for code" and "Boundary" sections record what the owner's redirect built.
>
> **Architecture (owner, 2026-11-22):** `vcikit` is the VCI abstraction `gmdiag` consumes.
> `vcxtoolkit` owns the native VCX protocol and will separately host our own J2534 pass-through DLL;
> `vcikit`'s J2534 backend loads whatever pass-through is *registered* (VXDIAG's today, ours later).
> `gmdiag` selects the backend. SAE J2534-1 is not in `primarysource`, so the J2534 API is grounded in
> public J2534-1 knowledge, the recovered native `VCXPT32.dll`, and ISO 22900-2 (`docs/j2534.md`).

A transport-agnostic model of a vehicle communication interface in **raw mode** — the
seam between a diagnostic driver (`gmdiag`) and whatever hardware actually reaches the
bus (`vcxtoolkit`'s VCX-NANO today, a SAE J2534 pass-through or RP1210 tomorrow). It
owns exactly the part of the stack that is a published standard and is the same across
every interface: the ComParam set that configures a link, and the raw-mode request /
response PDU convention. It is **not** a dongle driver, **not** ISO-TP, **not** UDS, and
**not** GM-specific.

## Why this exists

`gmdiag`'s first real read of the owner's Volt returned a constant `0x9E` for every
identifier. The cause, recovered statically from the registered J2534 pass-through
`VCXPT32.dll` and written up in `vcx-open-row-recovered-from-vcxpt32.md`, is that the
VCX exchange (a) never configured or started the channel — no `SetComParam` (CAN baud),
no `SetFilter`, no `Start` — and (b) dropped the ECU's CAN id: it put `1A C1` on the
wire where the real request is `00 00 07 E0 1A C1`, and parsed the reply from byte 0
where the real reply is `00 00 07 E8 5A C1 …`.

The id-prefix is not a VXDIAG quirk. ISO 22900-2:2009 §9.2.6.3.6 ("ComPrimitives in raw
mode (PassThru)") is normative: raw mode "has been added … to support pass-through
programs such as SAE J2534-1 and RP1210a", and "for ISO 15765, SAE J1939, and ISO 11898,
the first 4 bytes of the pCopData shall be the CAN ID (11 bit or 29 bit)", with an
extended-address byte following when enabled. The same clause assigns the raw-mode client
ownership of "protocol header byte configuration, CAN IDs, extended addressing, negative
responses, … tester present handling, and the binding of response(s) to the request."

That paragraph describes a layer, and it is a layer that neither sibling owns cleanly.
`vcxtoolkit` is the VCX *dongle* (its serial frame codec, its `VCX_PRO_*` commands, its
firmware). `gmdiag` is the GM *driver* (which DIDs, which modules, the read-first gating).
The raw-mode contract — id-prefixed PDUs, the ComParam set, the connect → configure →
start → send/recv → stop lifecycle, raw vs non-raw — sits between them, is identical for a
J2534 backend and the VCX backend, and is written down in ISO 22900-2 and SAE J2534-1.
Putting it in `vcxtoolkit` ties a standard contract to one dongle; putting it in `gmdiag`
ties it to one OEM. It wants its own crate.

## What it owns

- **The raw-mode PDU codec.** Prepend / strip the 4-byte CAN id (11- or 29-bit, big-endian)
  and the optional extended-address byte, per ISO 22900-2 §9.2.6.3.6. This is the exact
  fix `gmdiag` needs on send and recv, expressed once, for any backend.
- **The ComParam set.** A typed catalog of the link parameters a raw-mode client must set:
  CAN physical/functional request id, response id, baud, P2/P2* and STmin timing, flow-
  control (block size / separation time). ISO 22900-2 §11.1.4.10 names them
  (`CP_CanPhysReqId`, `CP_CanPhysReqFormat`, …); the *values* map onto each backend's own
  config call (for VCX, the recovered `SetComParam{key=3, value=baud}` + the flow-control
  `AddFilter`).
- **The ComLogicalLink lifecycle.** connect → set ComParams → start → send/recv → stop,
  as a state machine a backend drives. This is the missing bring-up that made `gmdiag`'s
  read return the dongle's status instead of the ECU's.
- **A `Backend` trait.** The one seam each interface implements: open the link, apply a
  ComParam set, start, exchange one raw-mode PDU, stop. `vcxtoolkit`'s `Channel` becomes
  one impl; a future `j2534kit` (ctypes-free FFI or a service) is another.

## What it does NOT own (and must not absorb)

- The VCX serial frame codec, `VCX_PRO_*` commands, licensing, firmware — all `vcxtoolkit`.
- UDS / GMLAN service PDUs (`$1A`, `$22`, `$3E`, negative-response decode) — all `udskit`.
- ISO-TP segmentation — in raw/PassThru mode the dongle firmware does it; for a raw-CAN
  backend it is `isotpkit`. vcikit carries whole application PDUs, not CAN frames.
- Non-raw / ODX ComParam databases, the D-Server API (ISO 22900-3), MVCI object model
  beyond the raw-mode subset our stack uses. Scope is raw mode only. Growing an ODX engine
  here is the shell-ahead-of-its-wave failure PORTFOLIO.md §7 warns about.
- GM module ids, DID catalogs, the read-first / owner gates — all `gmdiag`.

## Standards spine

The authoritative primary source for the raw-mode contract is **ISO 22900-2:2009** (D-PDU
API): §9.2.6.3.6 (raw mode / PassThru — the 4-byte CAN-id prefix) and §11.1.4.x (ComParams).
It is in `primarysource` (`iso:22900-2@2009`); confirm every citation with the `iso_*` tools,
never from memory (workspace rule). ISO 15765-2:2016 (`iso:15765-2@2016`) is the transport the
firmware runs; ISO 11898 (CAN) the link.

**SAE J2534 exists in `primarysource` but is GATED** (confirmed by the primarysource repo
owner session 2026-11-22): the document is recorded as `j2534@2002` in the SAE home
(`D:\reference\sae`, from `D:\Research\volt\sae.j2534.2002.pdf`, sha256 `867444e9…`), but the
`sae_*` tools are not on `main` — the SAE local collection (commit `44db669`) is on the
unmerged branch `claude/bt-obd-reference-candidates-cbc1b5`, and it sits behind a terms gate
that needs the owner's own `PRIMARYSOURCE_SAE_ACCEPT_TERMS`. So J2534 is not citable from the
`sae_*` tools until the owner decides to merge that branch and accept the SAE terms — the
owner's call, not a thing to work around. Until then, cite ISO 22900-2:2009 §9.2.6.3.6 (which
names J2534-1 as the raw mode it was "added to support") and §11.1.4.18.5 ("identical to
protocol ID CAN in SAE J2534-1"), confirmed present as `iso:22900-2@2009`; it is a faithful
stand-in. When the SAE collection is enabled, swap the ISO stand-in for direct `sae:j2534@2002`
citations. No J2534-2 PDF is known to be in the owner's files. Record the clause map in
`docs/iso-map.md`; never paraphrase either from memory.

## Dependencies and wave

- **Blocked on `vcxtoolkit` growing the raw-mode bring-up first.** The `SetComParam{key3,
  baud}` + `SetFilter(1)` + `Start(0)` + flow-control `AddFilter` sequence (recovered, and
  buildable from already-public `forward::{baud_record, set_com_param, start_payload,
  add_filter}` + `Command::ProSetComParam/ProSetFilter/ProStart`) is the first backend. Until
  one real backend can connect-configure-start, vcikit has nothing to abstract over and would
  be an empty trait — exactly the premature shell to avoid.
- **Light coupling to `udskit`** for the application PDU types it carries (opaque bytes are
  enough; no hard dep required).
- **Consumer: `gmdiag`.** Its `VcxTransport` / `vcx_device` stop calling `vcxtoolkit`
  directly and drive a `vcikit` link; the 4-byte id prepend/strip moves out of `gmdiag` into
  vcikit where it is backend-independent.

Tier/wave for PORTFOLIO.md: a refactor-seam crate, sequenced **after** the vcxtoolkit
channel-bring-up lands and **before** a second (J2534) backend is added — its whole
justification is the second backend, so it may stay a plan until that is actually wanted.

## Trigger for code — met (2026-11-22)

Two independent triggers were named; either was enough:
1. A **second backend** is genuinely on the table — the owner's redirect makes the J2534 pass-through
   a real, usable interface (a registered DLL, VXDIAG's today and our own later), so the id-prefix +
   link-config logic would otherwise be copied into two dongles; and
2. `vcxtoolkit`'s raw-mode bring-up and `gmdiag`'s id-prefix fix are the duplication this crate
   extracts.

Trigger 1 is now real, so the crate is **implemented** (not a skeleton): the `RawBackend` trait, the
`CanId` / `LinkConfig` types, the 4-byte CAN-id raw-mode codec, the J2534 message model, and the
**J2534 pass-through backend** are written and tested (`ROADMAP.md`). The second backend justifies the
abstraction exactly as this section required. The **native-VCX** backend remains the one piece still
blocked on `vcxtoolkit`'s channel bring-up; it is an exposed `BackendKind` with no impl yet.

## Name

`vcikit` (VCI = vehicle communication interface, ISO 22900-1's term). Vetted 2026-11-22 with
`projecthub shells names --index` against the 936,266-name snapshot:
- **crates.io: FREE**, no one-keystroke live neighbour. As a Rust crate this is the namespace
  that governs, so the name is clear to publish as a crate.
- PyPI: canonical `vcikit` is 404/404 (free) but flagged one keystroke from the live packages
  `bcikit` and `cikit`. Irrelevant for a Rust crate, but noted per the portfolio rule.
Directory keeps the name; the crate publish is clear on crates.io. (Contrast `itrust`, which is
one keystroke from live *crate/PyPI* names and so cannot release under that spelling.)

## Boundary

The abstraction and the J2534 backend are built; the native-VCX backend is not. This document, drafted
from the VCXPT32.dll recovery and the ISO 22900-2 confirmation and extended by the owner's 2026-11-22
redirect, defines the scope the crate fills. `vcxtoolkit` is not edited from here, and
`READ_LAYOUT_RECOVERED` is not flipped from here; implementing the native-VCX backend, which needs
`vcxtoolkit`'s raw-mode channel bring-up, remains the owner's decision. The J2534 backend touches a
registered pass-through DLL only on a real Windows host; off Windows (the CI) its device paths are
`cfg`-gated and discovery is empty. The transport stays dumb — the read-first / gated ethos lives in
`gmdiag`, not here.
