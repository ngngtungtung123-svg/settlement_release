# settlement_release

## Project Title
settlement_release

## Project Description
Lawsuit settlements frequently collapse not because the parties disagree on the deal, but because they have no shared, tamper-proof ledger of who has done what by when. `settlement_release` is a Soroban smart contract that anchors a settlement agreement between a plaintiff and a defendant on the Stellar network. The full terms of the deal are kept off-chain (for example on IPFS), and a 64-bit hash of those terms is recorded on-chain. Each obligation — a payment, an action, a deadline — is then tracked as a milestone that either party can mark complete, dispute, or use as evidence in follow-up proceedings, giving both sides and any arbitrator a single, immutable timeline of compliance.

## Project Vision
Our long-term vision is to make pre-trial and post-trial settlement execution as trustworthy as the underlying court order. By giving every milestone an on-chain, time-stamped audit trail, we want to (a) reduce the cost of enforcing settlements for small and mid-size claims, (b) give legal-tech platforms a primitive they can compose into mediation and arbitration products, and (c) eventually bridge real on-chain escrow so that funds or tokenised assets are released only when the corresponding milestone is mutually acknowledged. We see this dApp as the trust layer for a future where litigation and crypto-native finance can meet without losing legal rigour.

## Key Features
- **Settlement anchoring** — open an agreement with a plaintiff, defendant, an off-chain `terms_hash`, and a declared number of milestones; the same id can never be re-used.
- **Milestone tracking** — each milestone moves through `pending -> completed` or `pending -> disputed`, with the witness and any dispute reason recorded on-chain.
- **Mutual authorisation** — every state change is signed by one of the two parties using `require_auth()`, and the contract refuses calls from any other address.
- **Two terminal states** — `close()` (plaintiff only, requires every milestone completed) and `cancel()` (either party, records a reason) produce an immutable end to the agreement.
- **Read-only views** — `get_settlement` and `get_milestone_status` expose aggregate progress and per-milestone status for off-chain dashboards, arbitrators, and audit tools.

## Contract

- **Network:** Stellar Testnet (Public)
- **Scope:** legal dApp — see `contracts/settlement_release/src/lib.rs` for the full settlement_release business logic.
- **Functions exposed:** see `Key Features` above and the `pub fn` list in `lib.rs`.
- **Contract ID:** `CBGOG7G5N7NIZ5H2T53EOUA5T7DVNF5N5L3CMJCZQPGXQWZ7NKQSXVVY`
- **Explorer template:** `https://stellar.expert/explorer/testnet/tx/406a615caca23a384df0767849fa974f53d48d6bd00f650686e3f83c1d8d8718`

## Future Scope
- **On-chain escrow integration** — pair each payment milestone with a Stellar asset transfer that releases funds only when `mark_milestone` is invoked by the plaintiff.
- **Mediator / arbitrator role** — add a third authorised `Address` that can resolve disputes (move a milestone from `disputed` back to `pending` or `completed`) without re-opening the whole agreement.
- **Time-locked milestones** — extend `MilestoneData` with a deadline and let the contract auto-mark overdue obligations as `breached`, triggering an event listeners can consume.
- **Rich dispute evidence** — store a hash of supporting documents alongside the `reason` symbol so arbitrators can pull the full file from IPFS.
- **Mainnet release and DAO governance** — once audited, deploy to Stellar mainnet and let a legal DAO manage a registry of approved mediator templates.

## Profile

- **Name:** <!-- Fill github name -->
- **Project:** `settlement_release` (legal)
- **Built with:** Soroban SDK 25, Rust, Stellar Testnet
