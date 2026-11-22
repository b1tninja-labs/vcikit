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

## SAE J2534 — now cited directly

SAE J2534 (2002) is now imported into `primarysource` (`sae:j2534-2002`, terms acknowledged
2026-11-22). The J2534 pass-through backend's constants and the `PASSTHRU_MSG` layout are therefore
cited **directly** against SAE J2534, clause by clause, in `docs/j2534.md` — the SAE document is the
pass-through source. ISO 22900-2:2009 stays as the **cross-reference** that independently confirms the
raw-mode CAN-id convention (§9.2.6.3.6, §11.1.4.18.5) and maps the D-PDU API to SAE J2534-1 one-to-one
(Annex A, Table A.1); both are kept. The SAE source is a scanned PDF with OCR errors, so each value is
confirmed by its figure and cross-checked against the recovered native pass-through (`docs/j2534.md`).

## Transport and link, for reference

- ISO 15765-2:2016 (`iso:15765-2@2016`) — the transport the dongle firmware runs in raw/PassThru
  mode; segmentation is `isotpkit`'s, not this crate's.
- ISO 11898 (CAN) — the link layer.
