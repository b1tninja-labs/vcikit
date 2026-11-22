# Standards spine — clause map

The authoritative primary source for the raw-mode contract this crate will model is **ISO
22900-2:2009** (Road vehicles — Modular vehicle communication interface (MVCI) — Part 2: D-PDU API).
Every citation here was confirmed through the `primarysource` `iso_*` tools against
`iso:22900-2@2009`; none is from memory (workspace rule). Confirm again before quoting in code.

| Clause | What it fixes | Status |
|---|---|---|
| §9.2.6.3.6 *ComPrimitives in raw mode (PassThru)* | The raw-mode **request** convention. "For ISO 15765, SAE J1939, and ISO 11898, the first 4 bytes of the pCopData shall be the CAN ID (11 bit or 29 bit)", with the extended-address byte following when enabled. The clause also assigns the raw-mode client ownership of "protocol header byte configuration, CAN IDs, extended addressing, negative responses, … tester present handling, and the binding of response(s) to the request", and states raw mode "has been added … to support pass-through programs such as SAE J2534-1 and RP1210a". | Confirmed present |
| §11.1.4.18.5 *Expected response structure (RawMode/NonRawMode)* | The raw-mode **response** convention: "In RawMode, the expected response shall include the header bytes", and "For ISO 15765, ISO_11898_RAW and SAE J1939, the first 4 bytes will always contain the CAN ID" (Table 70). This is the mirror of §9.2.6.3.6 on the receive side — the part the recovered VCX exchange parsed from byte 0 instead of byte 4. | Confirmed present |
| Annex A, Table A.1 *Mapping of D-PDU, SAE J2534-1 and RP1210a API functions* | Ties the D-PDU `PDUConnect` / `PDUSetComParam` / `PDUStartComPrimitive` lifecycle to the SAE J2534-1 `PassThruConnect` / `PassThruWriteMsgs` / `PassThruReadMsgs` calls and the RP1210a equivalents. This is why ISO 22900-2 is a faithful stand-in for J2534-1 here. | Confirmed present |

## Why SAE J2534-1 is not cited directly

The raw-mode contract is also SAE J2534-1's, but that document is **not available** through the
`primarysource` `sae_*` tools on this machine: the SAE local collection sits behind the owner's
`PRIMARYSOURCE_SAE_ACCEPT_TERMS` gate on an unmerged branch (confirmed 2026-11-22). So J2534-1 must
not be cited from memory. ISO 22900-2:2009, which names J2534-1 as the raw mode it "has been added …
to support" (§9.2.6.3.6) and maps its functions one-to-one (Annex A, Table A.1), is the citable
stand-in. When the SAE collection is enabled, swap the ISO stand-in for direct `sae:j2534@2002`
citations.

## Transport and link, for reference

- ISO 15765-2:2016 (`iso:15765-2@2016`) — the transport the dongle firmware runs in raw/PassThru
  mode; segmentation is `isotpkit`'s, not this crate's.
- ISO 11898 (CAN) — the link layer.
