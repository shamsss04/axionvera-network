#![no_std]

use soroban_sdk::{contracterror, contracttype, Address, BytesN, Env, Symbol, Val, Vec};

/// Trait that all event emitters must implement.
/// Ensures each action emits a well-formed event with the standard two-topic pattern.
pub trait VaultEventEmitter {
    fn emit_initialize(e: &Env, admin: Address, deposit_token: Address, reward_token: Address);
    fn emit_deposit(e: &Env, user: Address, amount: i128);
    fn emit_withdraw(e: &Env, user: Address, amount: i128, remaining_balance: i128);
    fn emit_distribute(e: &Env, caller: Address, amount: i128);
    fn emit_claim_rewards(e: &Env, user: Address, amount: i128);
    fn emit_lock(e: &Env, user: Address, amount: i128, unlock_timestamp: u64);
    fn emit_unlock(e: &Env, user: Address, amount: i128);
    fn emit_admin_transfer_proposed(e: &Env, current_admin: Address, pending_admin: Address);
    fn emit_admin_transfer_accepted(e: &Env, previous_admin: Address, new_admin: Address);
    fn emit_upgrade(e: &Env, admin: Address, new_wasm_hash: BytesN<32>);
    fn emit_pause(e: &Env, admin: Address);
    fn emit_unpause(e: &Env, admin: Address);
    fn emit_asset_added(e: &Env, asset: Address);
    fn emit_asset_deposit(e: &Env, user: Address, asset: Address, amount: i128);
    fn emit_asset_withdraw(
        e: &Env,
        user: Address,
        asset: Address,
        amount: i128,
        remaining_balance: i128,
    );
    fn emit_asset_distribute(e: &Env, caller: Address, asset: Address, amount: i128);
    fn emit_asset_claim_rewards(e: &Env, user: Address, asset: Address, amount: i128);
}

/// A single operation inside a cross-contract execution plan.
///
/// `depends_on` lists operation ids that must appear earlier in the same plan.
/// `rollback` contains zero or one compensating calls that are scheduled if
/// this operation completed before a later step failed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrchestrationOperation {
    pub id: u32,
    pub target: Address,
    pub function: Symbol,
    pub args: Vec<Val>,
    pub depends_on: Vec<u32>,
    pub rollback: Vec<RollbackOperation>,
}

/// A compensating call for an executed operation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RollbackOperation {
    pub target: Address,
    pub function: Symbol,
    pub args: Vec<Val>,
}

/// A deterministic execution plan for coordinating multiple contract calls.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionPlan {
    pub id: BytesN<32>,
    pub caller: Address,
    pub operations: Vec<OrchestrationOperation>,
}

/// State recorded for a single operation in an execution receipt.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationStatus {
    Pending,
    Executed,
    RolledBack,
}

/// Final state of an execution plan.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutionStatus {
    Succeeded,
    Failed,
    RolledBack,
}

/// Per-operation receipt data persisted by the orchestrator.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationReceipt {
    pub operation_id: u32,
    pub status: OperationStatus,
}

/// Receipt persisted after every attempted orchestration run.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionReceipt {
    pub plan_id: BytesN<32>,
    pub caller: Address,
    pub status: ExecutionStatus,
    pub executed: Vec<OperationReceipt>,
    pub rollback: Vec<OperationReceipt>,
    pub failed_operation: Option<u32>,
    pub timestamp: u64,
}

/// Errors returned by orchestration validation and execution.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum OrchestrationError {
    EmptyPlan = 1,
    TooManyOperations = 2,
    DuplicateOperationId = 3,
    InvalidTarget = 4,
    InvalidDependency = 5,
    DependencyNotOrdered = 6,
    OperationFailed = 7,
    RollbackFailed = 8,
}

/// Interface implemented by contracts that coordinate execution plans.
pub trait TransactionOrchestrator {
    fn validate_plan(e: Env, plan: ExecutionPlan) -> Result<(), OrchestrationError>;
    fn execute_plan(e: Env, plan: ExecutionPlan) -> Result<ExecutionReceipt, OrchestrationError>;
    fn execution_receipt(e: Env, plan_id: BytesN<32>) -> Option<ExecutionReceipt>;
}

// ---------------------------------------------------------------------------
// Policy Engine Types and Interface
// ---------------------------------------------------------------------------

/// The type of policy.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, Copy)]
#[repr(u32)]
pub enum PolicyType {
    /// A policy that allows or denies operations based on conditions.
    AllowDeny = 0,
    /// A policy that validates operation parameters.
    Validation = 1,
}

/// The status of a policy.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, Copy)]
#[repr(u32)]
pub enum PolicyStatus {
    /// Policy is active and enforced.
    Active = 0,
    /// Policy is inactive and not enforced.
    Inactive = 1,
}

/// A policy rule that can be evaluated against an execution request.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Policy {
    /// Unique identifier for the policy.
    pub id: BytesN<32>,
    /// Human-readable name for the policy.
    pub name: Bytes,
    /// Type of the policy.
    pub policy_type: PolicyType,
    /// Status of the policy.
    pub status: PolicyStatus,
    /// The contract address this policy applies to (None for all).
    pub target_contract: Option<Address>,
    /// The function symbol this policy applies to (None for all functions).
    pub target_function: Option<Symbol>,
    /// The priority of the policy (higher = evaluated first).
    pub priority: u32,
    /// Timestamp when the policy was created.
    pub created_at: u64,
}

/// A request to evaluate policies for an operation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyEvaluationRequest {
    /// The contract address being called.
    pub target_contract: Address,
    /// The function symbol being called.
    pub target_function: Symbol,
    /// The arguments passed to the function.
    pub args: Vec<Val>,
    /// The caller of the operation.
    pub caller: Address,
}

/// The result of a policy evaluation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyEvaluationResult {
    /// Whether the evaluation passed.
    pub passed: bool,
    /// The ID of the policy that failed (if any).
    pub failed_policy_id: Option<BytesN<32>>,
    /// A message explaining the result.
    pub message: Bytes,
}

/// Errors returned by policy engine operations.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum PolicyError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    PolicyAlreadyExists = 4,
    PolicyNotFound = 5,
    InvalidPolicyName = 6,
    InvalidPolicyPriority = 7,
    PolicyEvaluationFailed = 8,
    Paused = 9,
    NoPendingAdmin = 10,
}

/// Interface implemented by policy engine contracts.
pub trait PolicyEngine {
    /// Initializes the policy engine with an admin.
    fn initialize(e: Env, admin: Address) -> Result<(), PolicyError>;
    
    /// Adds a new policy.
    fn add_policy(e: Env, policy: Policy) -> Result<(), PolicyError>;
    
    /// Updates an existing policy.
    fn update_policy(e: Env, policy: Policy) -> Result<(), PolicyError>;
    
    /// Deletes a policy.
    fn delete_policy(e: Env, policy_id: BytesN<32>) -> Result<(), PolicyError>;
    
    /// Gets a policy by ID.
    fn get_policy(e: Env, policy_id: BytesN<32>) -> Result<Policy, PolicyError>;
    
    /// Lists all policies.
    fn list_policies(e: Env) -> Result<Vec<Policy>, PolicyError>;
    
    /// Evaluates all active policies against a request.
    fn evaluate(e: Env, request: PolicyEvaluationRequest) -> Result<PolicyEvaluationResult, PolicyError>;
    
    /// Gets the current admin.
    fn admin(e: Env) -> Result<Address, PolicyError>;
    
    /// Proposes a new admin.
    fn propose_new_admin(e: Env, new_admin: Address) -> Result<(), PolicyError>;
    
    /// Accepts the admin role.
    fn accept_admin(e: Env, new_admin: Address) -> Result<(), PolicyError>;
    
    /// Pauses the policy engine (no evaluations or policy changes).
    fn pause_contract(e: Env) -> Result<(), PolicyError>;
    
    /// Unpauses the policy engine.
    fn unpause_contract(e: Env) -> Result<(), PolicyError>;
    
    /// Checks if the contract is paused.
    fn is_paused(e: Env) -> bool;
}
