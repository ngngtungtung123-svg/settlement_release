#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol};

// ---------------------------------------------------------------------------
// Status constants (kept as raw `u32` to remain `no_std` friendly and to make
// on-chain inspection by Stellar explorers and CLI tools straightforward).
// ---------------------------------------------------------------------------
const STATUS_OPEN: u32 = 0;
const STATUS_CLOSED: u32 = 1;
const STATUS_CANCELLED: u32 = 2;

const MILESTONE_PENDING: u32 = 0;
const MILESTONE_COMPLETED: u32 = 1;
const MILESTONE_DISPUTED: u32 = 2;

// ---------------------------------------------------------------------------
// On-chain record of a single lawsuit settlement agreement.
// The full, human-readable terms (payment schedule, deadlines, actions, etc.)
// are stored off-chain (e.g. IPFS or a legal document vault). Only a 64-bit
// fingerprint (`terms_hash`) of those terms is anchored on-chain so that both
// parties and any arbitrator can later verify the document has not been
// tampered with.
// ---------------------------------------------------------------------------
#[contracttype]
#[derive(Clone)]
pub struct SettlementData {
    pub plaintiff: Address,
    pub defendant: Address,
    pub terms_hash: u64,
    pub total_milestones: u32,
    pub completed_milestones: u32,
    pub status: u32,
    pub cancel_reason: Option<Symbol>,
}

// ---------------------------------------------------------------------------
// On-chain record of a single milestone inside a settlement. A milestone can
// represent a payment, a deliverable action, an injunction, or a dated
// obligation. It moves through pending -> completed (or disputed).
// ---------------------------------------------------------------------------
#[contracttype]
#[derive(Clone)]
pub struct MilestoneData {
    pub status: u32,
    pub marked_by: Option<Address>,
    pub disputed_by: Option<Address>,
    pub reason: Option<Symbol>,
}

/// `SettlementRelease` is a Soroban smart contract that anchors the
/// execution of a lawsuit settlement agreement between a plaintiff and a
/// defendant. It tracks an ordered list of milestones (payments, actions,
/// deadlines) and lets either party record completion, raise a dispute, or
/// cancel the agreement. Once every milestone is completed the plaintiff can
/// close the settlement, producing an immutable on-chain audit trail.
#[contract]
pub struct SettlementRelease;

#[contractimpl]
impl SettlementRelease {
    /// Open a new lawsuit settlement between `plaintiff` and `defendant`.
    ///
    /// `settlement_id` is a unique 64-bit identifier chosen by the caller.
    /// `terms_hash` is a 64-bit fingerprint of the off-chain settlement
    /// document (e.g. a hash of an IPFS CID or a SHA-256 prefix). It lets
    /// auditors later prove the document was not modified after signing.
    /// `total_milestones` declares how many obligations both parties agreed to.
    ///
    /// Authorization: the plaintiff must authorize the call because they
    /// are the party filing / agreeing to the settlement. The defendant is
    /// recorded as the counter-party; they do not need to sign on chain for
    /// the agreement to be opened (signing is captured by `terms_hash`).
    pub fn open_settlement(
        env: Env,
        plaintiff: Address,
        defendant: Address,
        settlement_id: u64,
        terms_hash: u64,
        total_milestones: u32,
    ) {
        plaintiff.require_auth();

        if plaintiff == defendant {
            panic!("plaintiff and defendant must differ");
        }
        if total_milestones == 0 {
            panic!("total_milestones must be greater than zero");
        }
        if env.storage().instance().has(&settlement_id) {
            panic!("settlement id already in use");
        }

        let settlement = SettlementData {
            plaintiff: plaintiff.clone(),
            defendant: defendant.clone(),
            terms_hash,
            total_milestones,
            completed_milestones: 0,
            status: STATUS_OPEN,
            cancel_reason: None,
        };
        env.storage()
            .instance()
            .set(&settlement_id, &settlement);
    }

    /// Mark a milestone as completed. Either the plaintiff or the defendant
    /// may call this; the caller becomes the witness of completion. The
    /// settlement's running `completed_milestones` counter is incremented.
    ///
    /// Calling on an already-completed milestone is rejected. Calling on a
    /// disputed milestone is also rejected (the dispute must be resolved
    /// first or the settlement cancelled and re-opened).
    pub fn mark_milestone(
        env: Env,
        party: Address,
        settlement_id: u64,
        milestone_id: u32,
    ) {
        party.require_auth();

        let mut settlement: SettlementData = env
            .storage()
            .instance()
            .get(&settlement_id)
            .expect("settlement not found");

        Self::assert_party(&settlement, &party);
        Self::assert_open(&settlement);
        Self::assert_milestone_in_range(&settlement, milestone_id);

        let key = (settlement_id, milestone_id);
        let mut milestone: MilestoneData = env
            .storage()
            .instance()
            .get(&key)
            .unwrap_or(MilestoneData {
                status: MILESTONE_PENDING,
                marked_by: None,
                disputed_by: None,
                reason: None,
            });

        if milestone.status == MILESTONE_COMPLETED {
            panic!("milestone already completed");
        }
        if milestone.status == MILESTONE_DISPUTED {
            panic!("milestone is under dispute");
        }

        milestone.status = MILESTONE_COMPLETED;
        milestone.marked_by = Some(party.clone());
        milestone.disputed_by = None;
        milestone.reason = None;

        settlement.completed_milestones += 1;

        env.storage().instance().set(&key, &milestone);
        env.storage()
            .instance()
            .set(&settlement_id, &settlement);
    }

    /// Dispute a milestone. Either party can raise a dispute, freezing that
    /// obligation's completion status and recording the disputing party plus
    /// a short `reason` symbol (e.g. "late", "short", "missing"). A disputed
    /// milestone cannot be marked completed until the dispute is resolved
    /// out-of-band and the settlement is re-initialised, or the settlement
    /// is cancelled entirely.
    pub fn dispute_milestone(
        env: Env,
        party: Address,
        settlement_id: u64,
        milestone_id: u32,
        reason: Symbol,
    ) {
        party.require_auth();

        let settlement: SettlementData = env
            .storage()
            .instance()
            .get(&settlement_id)
            .expect("settlement not found");

        Self::assert_party(&settlement, &party);
        Self::assert_open(&settlement);
        Self::assert_milestone_in_range(&settlement, milestone_id);

        let key = (settlement_id, milestone_id);
        let mut milestone: MilestoneData = env
            .storage()
            .instance()
            .get(&key)
            .unwrap_or(MilestoneData {
                status: MILESTONE_PENDING,
                marked_by: None,
                disputed_by: None,
                reason: None,
            });

        if milestone.status == MILESTONE_COMPLETED {
            panic!("cannot dispute an already completed milestone");
        }
        if milestone.status == MILESTONE_DISPUTED {
            panic!("milestone already disputed");
        }

        milestone.status = MILESTONE_DISPUTED;
        milestone.disputed_by = Some(party.clone());
        milestone.reason = Some(reason);

        env.storage().instance().set(&key, &milestone);
    }

    /// Close a settlement once every single milestone has been marked
    /// completed. Only the plaintiff may close, mirroring the real-world
    /// convention that the moving party in a lawsuit dismisses the case
    /// after the defendant has performed. The settlement's status becomes
    /// `STATUS_CLOSED`, an immutable terminal state.
    pub fn close(env: Env, plaintiff: Address, settlement_id: u64) {
        plaintiff.require_auth();

        let mut settlement: SettlementData = env
            .storage()
            .instance()
            .get(&settlement_id)
            .expect("settlement not found");

        if plaintiff != settlement.plaintiff {
            panic!("only the plaintiff can close the settlement");
        }
        if settlement.status != STATUS_OPEN {
            panic!("settlement is not open");
        }
        if settlement.completed_milestones != settlement.total_milestones {
            panic!("not all milestones are completed");
        }

        settlement.status = STATUS_CLOSED;
        env.storage()
            .instance()
            .set(&settlement_id, &settlement);
    }

    /// Cancel an open settlement. Either party can invoke this with a
    /// symbolic `reason` (e.g. "breach", "mutual", "frivolous"). Cancelling
    /// moves the settlement to the terminal `STATUS_CANCELLED` state and
    /// permanently records the reason on-chain.
    pub fn cancel(env: Env, party: Address, settlement_id: u64, reason: Symbol) {
        party.require_auth();

        let mut settlement: SettlementData = env
            .storage()
            .instance()
            .get(&settlement_id)
            .expect("settlement not found");

        Self::assert_party(&settlement, &party);
        if settlement.status != STATUS_OPEN {
            panic!("settlement is not open");
        }

        settlement.status = STATUS_CANCELLED;
        settlement.cancel_reason = Some(reason);
        env.storage()
            .instance()
            .set(&settlement_id, &settlement);
    }

    /// Read the status of a single milestone. Returns
    /// `0` (pending), `1` (completed) or `2` (disputed). This is the
    /// primary view used by front-ends and arbitrators to display progress.
    pub fn get_milestone_status(env: Env, settlement_id: u64, milestone_id: u32) -> u32 {
        let key = (settlement_id, milestone_id);
        let milestone: MilestoneData = env
            .storage()
            .instance()
            .get(&key)
            .expect("milestone not found");
        milestone.status
    }

    /// Read the aggregate state of a settlement as the tuple
    /// `(status, completed_milestones, total_milestones)`. The `status`
    /// follows the same `0` open / `1` closed / `2` cancelled encoding.
    pub fn get_settlement(env: Env, settlement_id: u64) -> (u32, u32, u32) {
        let settlement: SettlementData = env
            .storage()
            .instance()
            .get(&settlement_id)
            .expect("settlement not found");
        (
            settlement.status,
            settlement.completed_milestones,
            settlement.total_milestones,
        )
    }

    // -----------------------------------------------------------------------
    // Internal helpers (no `pub`, no `self` exposed externally).
    // -----------------------------------------------------------------------

    fn assert_party(settlement: &SettlementData, party: &Address) {
        if party != &settlement.plaintiff && party != &settlement.defendant {
            panic!("caller is not a party to the settlement");
        }
    }

    fn assert_open(settlement: &SettlementData) {
        if settlement.status != STATUS_OPEN {
            panic!("settlement is not open");
        }
    }

    fn assert_milestone_in_range(settlement: &SettlementData, milestone_id: u32) {
        if milestone_id == 0 || milestone_id > settlement.total_milestones {
            panic!("milestone id out of range");
        }
    }
}
