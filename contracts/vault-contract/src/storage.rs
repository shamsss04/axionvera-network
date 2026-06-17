use soroban_sdk::{contracttype, Address, Env, Map};

use crate::errors::{ArithmeticError, AuthorizationError, BalanceError, StateError, ValidationError, VaultError};

pub const PRECISION_FACTOR: i128 = 1_000_000_000;
const REWARD_INDEX_SCALE: i128 = PRECISION_FACTOR;

const INSTANCE_TTL_THRESHOLD: u32 = 518_400;
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400;

const PERSISTENT_TTL_THRESHOLD: u32 = 518_400;
const PERSISTENT_TTL_EXTEND_TO: u32 = 518_400;

/// Keys used to store data in the contract's storage.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Flag indicating if the contract has been initialized.
    Initialized,
    /// Admin address
    Admin,
    /// Pending admin address (for two-step transfer)
    PendingAdmin,
    /// Deposit token address (legacy, kept for backwards compatibility)
    DepositToken,
    /// Reward token address
    RewardToken,
    /// Total deposits amount (legacy, kept for backwards compatibility)
    TotalDeposits,
    /// Global reward index (legacy, kept for backwards compatibility)
    RewardIndex,
    /// Vesting period in seconds
    VestingPeriod,
    /// Reentrancy guard flag
    ReentrancyGuard,
    /// Pause flag
    IsPaused,
    /// User balance (legacy, kept for backwards compatibility)
    UserBalance(Address),
    /// User's last synced reward index (legacy, kept for backwards compatibility)
    UserRewardIndex(Address),
    /// User's accrued but unvested rewards (legacy, kept for backwards compatibility)
    UserAccruedRewards(Address),
    /// User's last reward distribution timestamp (legacy, kept for backwards compatibility)
    UserLastRewardTimestamp(Address),
    /// Map of supported asset addresses
    SupportedAssets,
    /// Total deposits per asset
    AssetTotalDeposits(Address),
    /// Global reward index per asset
    AssetRewardIndex(Address),
    /// User balance per asset
    UserAssetBalance(Address, Address), // (user, asset)
    /// User's last synced reward index per asset
    UserAssetRewardIndex(Address, Address), // (user, asset)
    /// User's accrued but unvested rewards per asset
    UserAssetAccruedRewards(Address, Address), // (user, asset)
    /// User's last reward distribution timestamp per asset
    UserAssetLastRewardTimestamp(Address, Address), // (user, asset)
}

/// The global state of the vault contract.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultState {
    /// The address allowed to perform administrative actions like reward distribution.
    pub admin: Address,
    /// The address of the token that users deposit into the vault.
    pub deposit_token: Address,
    /// The address of the token distributed as rewards.
    pub reward_token: Address,
    /// The total amount of deposit tokens currently held by the vault.
    pub total_deposits: i128,
    /// The global reward index that tracks cumulative rewards per unit of deposit.
    pub reward_index: i128,
    /// The vesting period in seconds.
    pub vesting_period: u64,
}

/// Snapshot of a user's position in the vault for a specific asset.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserPosition {
    /// The amount of deposit tokens the user has currently staked.
    pub balance: i128,
    /// The value of the global reward index at the time of the user's last interaction.
    pub reward_index: i128,
    /// The amount of rewards the user has earned but not yet vested/claimed.
    pub accrued_rewards: i128,
    /// The timestamp of the last reward distribution affecting this user.
    pub last_reward_timestamp: u64,
}

/// Snapshot of a user's position across multiple assets.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiAssetPosition {
    /// Map of asset address to user position
    pub positions: Map<Address, UserPosition>,
}

/// A helper struct for returning reward information in view functions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserRewardSnapshot {
    /// The current reward index applied to the snapshot.
    pub reward_index: i128,
    /// The total rewards (accrued + pending) for the user.
    pub rewards: i128,
    /// The amount of vested rewards available to claim.
    pub vested_rewards: i128,
}

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------

pub fn is_initialized(e: &Env) -> bool {
    e.storage()
        .instance()
        .get::<_, bool>(&DataKey::Initialized)
        .unwrap_or(false)
}

pub fn require_initialized(e: &Env) -> Result<(), VaultError> {
    if is_initialized(e) {
        Ok(())
    } else {
        Err(StateError::NotInitialized.into())
    }
}

pub fn require_not_paused(e: &Env) -> Result<(), VaultError> {
    if e.storage().instance().get::<_, bool>(&DataKey::IsPaused).unwrap_or(false) {
        Err(AuthorizationError::Unauthorized.into())
    } else {
        Ok(())
    }
}

pub fn initialize_state(
    e: &Env,
    admin: &Address,
    deposit_token: &Address,
    reward_token: &Address,
    vesting_period: u64,
) {
    e.storage().instance().set(&DataKey::Initialized, &true);
    e.storage().instance().set(&DataKey::Admin, admin);
    e.storage().instance().remove(&DataKey::PendingAdmin);
    e.storage().instance().set(&DataKey::DepositToken, deposit_token);
    e.storage().instance().set(&DataKey::RewardToken, reward_token);
    e.storage().instance().set(&DataKey::VestingPeriod, &vesting_period);
    e.storage().instance().set(&DataKey::TotalDeposits, &0_i128);
    e.storage().instance().set(&DataKey::RewardIndex, &0_i128);
    e.storage().instance().set(&DataKey::ReentrancyGuard, &false);
    e.storage().instance().set(&DataKey::IsPaused, &false);
    bump_instance_ttl(e);
}

// ---------------------------------------------------------------------------
// State (global)
// ---------------------------------------------------------------------------

pub fn get_state(e: &Env) -> Result<VaultState, VaultError> {
    require_initialized(e)?;
    let admin = e
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(StateError::InvalidState)?;
    let deposit_token = e
        .storage()
        .instance()
        .get(&DataKey::DepositToken)
        .ok_or(StateError::InvalidState)?;
    let reward_token = e
        .storage()
        .instance()
        .get(&DataKey::RewardToken)
        .ok_or(StateError::InvalidState)?;
    let total_deposits = e
        .storage()
        .instance()
        .get(&DataKey::TotalDeposits)
        .unwrap_or(0_i128);
    let reward_index = e
        .storage()
        .instance()
        .get(&DataKey::RewardIndex)
        .unwrap_or(0_i128);
    let vesting_period = e
        .storage()
        .instance()
        .get(&DataKey::VestingPeriod)
        .unwrap_or(0_u64);
    bump_instance_ttl(e);
    Ok(VaultState {
        admin,
        deposit_token,
        reward_token,
        total_deposits,
        reward_index,
        vesting_period,
    })
}

pub fn get_admin(e: &Env) -> Result<Address, VaultError> {
    Ok(get_state(e)?.admin)
}

pub fn set_admin(e: &Env, admin: &Address) {
    e.storage().instance().set(&DataKey::Admin, admin);
    bump_instance_ttl(e);
}

pub fn get_pending_admin(e: &Env) -> Result<Option<Address>, VaultError> {
    require_initialized(e)?;
    let pending = e.storage().instance().get(&DataKey::PendingAdmin);
    bump_instance_ttl(e);
    Ok(pending)
}

pub fn set_pending_admin(e: &Env, pending_admin: &Address) {
    e.storage().instance().set(&DataKey::PendingAdmin, pending_admin);
    bump_instance_ttl(e);
}

pub fn clear_pending_admin(e: &Env) {
    e.storage().instance().remove(&DataKey::PendingAdmin);
    bump_instance_ttl(e);
}

pub fn get_deposit_token(e: &Env) -> Result<Address, VaultError> {
    Ok(get_state(e)?.deposit_token)
}

pub fn get_reward_token(e: &Env) -> Result<Address, VaultError> {
    Ok(get_state(e)?.reward_token)
}

pub fn get_total_deposits(e: &Env) -> Result<i128, VaultError> {
    Ok(get_state(e)?.total_deposits)
}

pub fn set_total_deposits(e: &Env, total: i128) {
    e.storage().instance().set(&DataKey::TotalDeposits, &total);
    bump_instance_ttl(e);
}

pub fn get_reward_index(e: &Env) -> Result<i128, VaultError> {
    Ok(get_state(e)?.reward_index)
}

pub fn set_reward_index(e: &Env, index: i128) {
    e.storage().instance().set(&DataKey::RewardIndex, &index);
    bump_instance_ttl(e);
}

pub fn get_vesting_period(e: &Env) -> Result<u64, VaultError> {
    Ok(get_state(e)?.vesting_period)
}

pub fn set_paused(e: &Env, paused: bool) {
    e.storage().instance().set(&DataKey::IsPaused, &paused);
    bump_instance_ttl(e);
}

// ---------------------------------------------------------------------------
// Reentrancy Guard
// ---------------------------------------------------------------------------

pub fn enter_non_reentrant(e: &Env) -> Result<(), VaultError> {
    if e.storage()
        .instance()
        .get::<_, bool>(&DataKey::ReentrancyGuard)
        .unwrap_or(false)
    {
        return Err(AuthorizationError::ReentrancyDetected.into());
    }
    e.storage().instance().set(&DataKey::ReentrancyGuard, &true);
    bump_instance_ttl(e);
    Ok(())
}

pub fn exit_non_reentrant(e: &Env) {
    e.storage().instance().set(&DataKey::ReentrancyGuard, &false);
    bump_instance_ttl(e);
}

// ---------------------------------------------------------------------------
// User Position
// ---------------------------------------------------------------------------

pub fn get_user_position(e: &Env, user: &Address) -> Result<UserPosition, VaultError> {
    require_initialized(e)?;
    Ok(get_user_position_unchecked(e, user))
}

pub fn get_user_position_unchecked(e: &Env, user: &Address) -> UserPosition {
    let balance_key = DataKey::UserBalance(user.clone());
    let reward_index_key = DataKey::UserRewardIndex(user.clone());
    let accrued_rewards_key = DataKey::UserAccruedRewards(user.clone());
    let last_reward_timestamp_key = DataKey::UserLastRewardTimestamp(user.clone());

    let balance = e.storage().persistent().get(&balance_key).unwrap_or(0_i128);
    let reward_index = e
        .storage()
        .persistent()
        .get(&reward_index_key)
        .unwrap_or(0_i128);
    let accrued_rewards = e
        .storage()
        .persistent()
        .get(&accrued_rewards_key)
        .unwrap_or(0_i128);
    let last_reward_timestamp = e
        .storage()
        .persistent()
        .get(&last_reward_timestamp_key)
        .unwrap_or(0_u64);

    if balance != 0 {
        bump_persistent_ttl(e, &balance_key);
    }
    if reward_index != 0 {
        bump_persistent_ttl(e, &reward_index_key);
    }
    if accrued_rewards != 0 {
        bump_persistent_ttl(e, &accrued_rewards_key);
    }
    if last_reward_timestamp != 0 {
        bump_persistent_ttl(e, &last_reward_timestamp_key);
    }

    UserPosition {
        balance,
        reward_index,
        accrued_rewards,
        last_reward_timestamp,
    }
}

pub fn set_user_position(e: &Env, user: &Address, position: &UserPosition) {
    let balance_key = DataKey::UserBalance(user.clone());
    let reward_index_key = DataKey::UserRewardIndex(user.clone());
    let accrued_rewards_key = DataKey::UserAccruedRewards(user.clone());
    let last_reward_timestamp_key = DataKey::UserLastRewardTimestamp(user.clone());

    if position.balance == 0 {
        e.storage().persistent().remove(&balance_key);
    } else {
        e.storage().persistent().set(&balance_key, &position.balance);
        bump_persistent_ttl(e, &balance_key);
    }

    if position.reward_index == 0 {
        e.storage().persistent().remove(&reward_index_key);
    } else {
        e.storage()
            .persistent()
            .set(&reward_index_key, &position.reward_index);
        bump_persistent_ttl(e, &reward_index_key);
    }

    if position.accrued_rewards == 0 {
        e.storage().persistent().remove(&accrued_rewards_key);
    } else {
        e.storage()
            .persistent()
            .set(&accrued_rewards_key, &position.accrued_rewards);
        bump_persistent_ttl(e, &accrued_rewards_key);
    }

    e.storage()
        .persistent()
        .set(&last_reward_timestamp_key, &position.last_reward_timestamp);
    bump_persistent_ttl(e, &last_reward_timestamp_key);
}

pub fn get_user_balance(e: &Env, user: &Address) -> Result<i128, VaultError> {
    Ok(get_user_position(e, user)?.balance)
}

// ---------------------------------------------------------------------------
// Deposit/Withdraw Logic
// ---------------------------------------------------------------------------

pub fn store_deposit(
    e: &Env,
    user: &Address,
    amount: i128,
) -> Result<(VaultState, UserPosition), VaultError> {
    let state = get_state(e)?;
    let mut position = get_user_position_unchecked(e, user);
    
    // Accrue rewards earned up to this point using the old balance.
    accrue_position_rewards(e, &state, &mut position)?;

    // Update balance and total deposits.
    position.balance = position
        .balance
        .checked_add(amount)
        .ok_or(ArithmeticError::Overflow)?;
    let next_total = state
        .total_deposits
        .checked_add(amount)
        .ok_or(ArithmeticError::Overflow)?;

    // Persist changes.
    set_total_deposits(e, next_total);
    set_user_position(e, user, &position);

    Ok((
        VaultState {
            total_deposits: next_total,
            ..state
        },
        position,
    ))
}

pub fn store_withdraw(
    e: &Env,
    user: &Address,
    amount: i128,
) -> Result<(VaultState, UserPosition), VaultError> {
    let state = get_state(e)?;
    let mut position = get_user_position_unchecked(e, user);
    
    // Accrue rewards earned up to this point using the old balance.
    accrue_position_rewards(e, &state, &mut position)?;

    if position.balance < amount {
        return Err(BalanceError::InsufficientBalance.into());
    }
    
    // Update balance and total deposits.
    position.balance = position
        .balance
        .checked_sub(amount)
        .ok_or(ArithmeticError::Overflow)?;
    let next_total = state
        .total_deposits
        .checked_sub(amount)
        .ok_or(ArithmeticError::Overflow)?;

    // Persist changes.
    set_total_deposits(e, next_total);
    set_user_position(e, user, &position);

    Ok((
        VaultState {
            total_deposits: next_total,
            ..state
        },
        position,
    ))
}

// ---------------------------------------------------------------------------
// Reward Distribution
// ---------------------------------------------------------------------------

pub fn store_reward_distribution(e: &Env, amount: i128) -> Result<VaultState, VaultError> {
    let state = get_state(e)?;
    let increment = checked_reward_index_increment(amount, state.total_deposits)?;

    let next_reward_index = state
        .reward_index
        .checked_add(increment)
        .ok_or(ArithmeticError::Overflow)?;

    set_reward_index(e, next_reward_index);

    Ok(VaultState {
        reward_index: next_reward_index,
        ..state
    })
}

// ---------------------------------------------------------------------------
// Claim Rewards
// ---------------------------------------------------------------------------

pub fn calculate_vested_rewards(
    current_timestamp: u64,
    position: &UserPosition,
    vesting_period: u64,
) -> Result<i128, VaultError> {
    if vesting_period == 0 {
        // No vesting period, all rewards are immediately vested
        return Ok(position.accrued_rewards);
    }

    if position.last_reward_timestamp == 0 {
        return Ok(0);
    }

    let time_elapsed = current_timestamp
        .checked_sub(position.last_reward_timestamp)
        .unwrap_or(0);

    if time_elapsed >= vesting_period {
        // All rewards are vested
        Ok(position.accrued_rewards)
    } else {
        // Calculate partial vesting
        let vested = (position.accrued_rewards as u128)
            .checked_mul(time_elapsed as u128)
            .ok_or(ArithmeticError::Overflow)?
            .checked_div(vesting_period as u128)
            .ok_or(ArithmeticError::RewardCalculationFailed)? as i128;
        Ok(vested)
    }
}

pub fn store_claimable_rewards(e: &Env, user: &Address) -> Result<i128, VaultError> {
    let state = get_state(e)?;
    let mut position = get_user_position_unchecked(e, user);
    
    // Accrue all rewards earned up to the current global index.
    accrue_position_rewards(e, &state, &mut position)?;

    // Calculate vested rewards
    let current_timestamp = e.ledger().timestamp();
    let vested = calculate_vested_rewards(current_timestamp, &position, state.vesting_period)?;

    // Update position with remaining accrued rewards
    position.accrued_rewards = position
        .accrued_rewards
        .checked_sub(vested)
        .ok_or(ArithmeticError::Overflow)?;

    set_user_position(e, user, &position);

    Ok(vested)
}

// ---------------------------------------------------------------------------
// Read-only reward preview
// ---------------------------------------------------------------------------

pub fn preview_user_rewards(e: &Env, user: &Address) -> Result<UserRewardSnapshot, VaultError> {
    require_initialized(e)?;
    let state = get_state(e)?;
    let mut position = get_user_position_unchecked(e, user);
    
    // Calculate accrued rewards without modifying state
    accrue_position_rewards(e, &state, &mut position)?;

    let current_timestamp = e.ledger().timestamp();
    let vested = calculate_vested_rewards(current_timestamp, &position, state.vesting_period)?;

    Ok(UserRewardSnapshot {
        reward_index: position.reward_index,
        rewards: position.accrued_rewards,
        vested_rewards: vested,
    })
}

pub fn pending_user_rewards_view(e: &Env, user: &Address) -> Result<i128, VaultError> {
    Ok(preview_user_rewards(e, user)?.rewards)
}

pub fn vested_user_rewards_view(e: &Env, user: &Address) -> Result<i128, VaultError> {
    Ok(preview_user_rewards(e, user)?.vested_rewards)
}

// ---------------------------------------------------------------------------
// Helper Functions
// ---------------------------------------------------------------------------

pub(crate) fn checked_reward_index_increment(
    amount: i128,
    total_deposits: i128,
) -> Result<i128, VaultError> {
    if total_deposits <= 0 {
        return Err(BalanceError::NoDeposits.into());
    }

    let scaled = amount
        .checked_mul(REWARD_INDEX_SCALE)
        .ok_or(ArithmeticError::Overflow)?;
    let increment = scaled
        .checked_div(total_deposits)
        .ok_or(ArithmeticError::RewardCalculationFailed)?;

    if increment <= 0 {
        return Err(ArithmeticError::ZeroRewardIncrement.into());
    }

    Ok(increment)
}

pub(crate) fn checked_accrued_rewards(balance: i128, delta: i128) -> Result<i128, VaultError> {
    balance
        .checked_mul(delta)
        .ok_or(ArithmeticError::Overflow)?
        .checked_div(REWARD_INDEX_SCALE)
        .ok_or(ArithmeticError::RewardCalculationFailed.into())
}

fn accrue_position_rewards(
    e: &Env,
    state: &VaultState,
    position: &mut UserPosition,
) -> Result<(), VaultError> {
    if state.reward_index == position.reward_index || position.balance == 0 {
        position.reward_index = state.reward_index;
        return Ok(());
    }

    if position.balance > 0 {
        let delta = state
            .reward_index
            .checked_sub(position.reward_index)
            .ok_or(ArithmeticError::Overflow)?;
        let accrued = checked_accrued_rewards(position.balance, delta)?;

        if accrued > 0 {
            position.accrued_rewards = position
                .accrued_rewards
                .checked_add(accrued)
                .ok_or(ArithmeticError::Overflow)?;
            // Update last reward timestamp whenever new rewards are accrued
            position.last_reward_timestamp = e.ledger().timestamp();
        }
    }

    position.reward_index = state.reward_index;
    Ok(())
}

fn bump_instance_ttl(e: &Env) {
    e.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}

fn bump_persistent_ttl(e: &Env, key: &DataKey) {
    e.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

// ---------------------------------------------------------------------------
// Multi-Asset Support Functions
// ---------------------------------------------------------------------------

/// Add a new supported asset to the vault
pub fn add_supported_asset(e: &Env, asset: &Address) -> Result<(), VaultError> {
    let mut assets = get_supported_assets(e);
    if !assets.contains_key(asset.clone()) {
        assets.set(asset.clone(), true);
        e.storage().instance().set(&DataKey::SupportedAssets, &assets);
        
        // Initialize asset-specific state
        e.storage().instance().set(&DataKey::AssetTotalDeposits(asset.clone()), &0_i128);
        e.storage().instance().set(&DataKey::AssetRewardIndex(asset.clone()), &0_i128);
        
        bump_instance_ttl(e);
    }
    Ok(())
}

/// Get all supported assets
pub fn get_supported_assets(e: &Env) -> Map<Address, bool> {
    e.storage()
        .instance()
        .get(&DataKey::SupportedAssets)
        .unwrap_or(Map::new(e))
}

/// Check if an asset is supported
pub fn is_asset_supported(e: &Env, asset: &Address) -> bool {
    let assets = get_supported_assets(e);
    assets.contains_key(asset.clone())
}

/// Get total deposits for a specific asset
pub fn get_asset_total_deposits(e: &Env, asset: &Address) -> Result<i128, VaultError> {
    if !is_asset_supported(e, asset) {
        return Err(ValidationError::InvalidAddress.into());
    }
    Ok(e.storage()
        .instance()
        .get(&DataKey::AssetTotalDeposits(asset.clone()))
        .unwrap_or(0_i128))
}

/// Set total deposits for a specific asset
pub fn set_asset_total_deposits(e: &Env, asset: &Address, total: i128) {
    e.storage().instance().set(&DataKey::AssetTotalDeposits(asset.clone()), &total);
    bump_instance_ttl(e);
}

/// Get reward index for a specific asset
pub fn get_asset_reward_index(e: &Env, asset: &Address) -> Result<i128, VaultError> {
    if !is_asset_supported(e, asset) {
        return Err(ValidationError::InvalidAddress.into());
    }
    Ok(e.storage()
        .instance()
        .get(&DataKey::AssetRewardIndex(asset.clone()))
        .unwrap_or(0_i128))
}

/// Set reward index for a specific asset
pub fn set_asset_reward_index(e: &Env, asset: &Address, index: i128) {
    e.storage().instance().set(&DataKey::AssetRewardIndex(asset.clone()), &index);
    bump_instance_ttl(e);
}

/// Get user position for a specific asset
pub fn get_user_asset_position(e: &Env, user: &Address, asset: &Address) -> Result<UserPosition, VaultError> {
    require_initialized(e)?;
    if !is_asset_supported(e, asset) {
        return Err(ValidationError::InvalidAddress.into());
    }
    Ok(get_user_asset_position_unchecked(e, user, asset))
}

pub fn get_user_asset_position_unchecked(e: &Env, user: &Address, asset: &Address) -> UserPosition {
    let balance_key = DataKey::UserAssetBalance(user.clone(), asset.clone());
    let reward_index_key = DataKey::UserAssetRewardIndex(user.clone(), asset.clone());
    let accrued_rewards_key = DataKey::UserAssetAccruedRewards(user.clone(), asset.clone());
    let last_reward_timestamp_key = DataKey::UserAssetLastRewardTimestamp(user.clone(), asset.clone());

    let balance = e.storage().persistent().get(&balance_key).unwrap_or(0_i128);
    let reward_index = e
        .storage()
        .persistent()
        .get(&reward_index_key)
        .unwrap_or(0_i128);
    let accrued_rewards = e
        .storage()
        .persistent()
        .get(&accrued_rewards_key)
        .unwrap_or(0_i128);
    let last_reward_timestamp = e
        .storage()
        .persistent()
        .get(&last_reward_timestamp_key)
        .unwrap_or(0_u64);

    if balance != 0 {
        bump_persistent_ttl(e, &balance_key);
    }
    if reward_index != 0 {
        bump_persistent_ttl(e, &reward_index_key);
    }
    if accrued_rewards != 0 {
        bump_persistent_ttl(e, &accrued_rewards_key);
    }
    if last_reward_timestamp != 0 {
        bump_persistent_ttl(e, &last_reward_timestamp_key);
    }

    UserPosition {
        balance,
        reward_index,
        accrued_rewards,
        last_reward_timestamp,
    }
}

/// Set user position for a specific asset
pub fn set_user_asset_position(e: &Env, user: &Address, asset: &Address, position: &UserPosition) {
    let balance_key = DataKey::UserAssetBalance(user.clone(), asset.clone());
    let reward_index_key = DataKey::UserAssetRewardIndex(user.clone(), asset.clone());
    let accrued_rewards_key = DataKey::UserAssetAccruedRewards(user.clone(), asset.clone());
    let last_reward_timestamp_key = DataKey::UserAssetLastRewardTimestamp(user.clone(), asset.clone());

    if position.balance == 0 {
        e.storage().persistent().remove(&balance_key);
    } else {
        e.storage().persistent().set(&balance_key, &position.balance);
        bump_persistent_ttl(e, &balance_key);
    }

    if position.reward_index == 0 {
        e.storage().persistent().remove(&reward_index_key);
    } else {
        e.storage()
            .persistent()
            .set(&reward_index_key, &position.reward_index);
        bump_persistent_ttl(e, &reward_index_key);
    }

    if position.accrued_rewards == 0 {
        e.storage().persistent().remove(&accrued_rewards_key);
    } else {
        e.storage()
            .persistent()
            .set(&accrued_rewards_key, &position.accrued_rewards);
        bump_persistent_ttl(e, &accrued_rewards_key);
    }

    e.storage()
        .persistent()
        .set(&last_reward_timestamp_key, &position.last_reward_timestamp);
    bump_persistent_ttl(e, &last_reward_timestamp_key);
}

/// Get user balance for a specific asset
pub fn get_user_asset_balance(e: &Env, user: &Address, asset: &Address) -> Result<i128, VaultError> {
    Ok(get_user_asset_position(e, user, asset)?.balance)
}

/// Store deposit for a specific asset
pub fn store_asset_deposit(
    e: &Env,
    user: &Address,
    asset: &Address,
    amount: i128,
) -> Result<UserPosition, VaultError> {
    if !is_asset_supported(e, asset) {
        return Err(ValidationError::InvalidAddress.into());
    }
    
    let mut position = get_user_asset_position_unchecked(e, user, asset);
    let asset_reward_index = get_asset_reward_index(e, asset)?;
    let asset_total = get_asset_total_deposits(e, asset)?;
    
    // Accrue rewards earned up to this point using the old balance.
    accrue_asset_position_rewards(e, asset_reward_index, &mut position)?;

    // Update balance and total deposits.
    position.balance = position
        .balance
        .checked_add(amount)
        .ok_or(ArithmeticError::Overflow)?;
    let next_total = asset_total
        .checked_add(amount)
        .ok_or(ArithmeticError::Overflow)?;

    // Persist changes.
    set_asset_total_deposits(e, asset, next_total);
    set_user_asset_position(e, user, asset, &position);

    Ok(position)
}

/// Store withdraw for a specific asset
pub fn store_asset_withdraw(
    e: &Env,
    user: &Address,
    asset: &Address,
    amount: i128,
) -> Result<UserPosition, VaultError> {
    if !is_asset_supported(e, asset) {
        return Err(ValidationError::InvalidAddress.into());
    }
    
    let mut position = get_user_asset_position_unchecked(e, user, asset);
    let asset_reward_index = get_asset_reward_index(e, asset)?;
    let asset_total = get_asset_total_deposits(e, asset)?;
    
    // Accrue rewards earned up to this point using the old balance.
    accrue_asset_position_rewards(e, asset_reward_index, &mut position)?;

    if position.balance < amount {
        return Err(BalanceError::InsufficientBalance.into());
    }
    
    // Update balance and total deposits.
    position.balance = position
        .balance
        .checked_sub(amount)
        .ok_or(ArithmeticError::Overflow)?;
    let next_total = asset_total
        .checked_sub(amount)
        .ok_or(ArithmeticError::Overflow)?;

    // Persist changes.
    set_asset_total_deposits(e, asset, next_total);
    set_user_asset_position(e, user, asset, &position);

    Ok(position)
}

/// Store reward distribution for a specific asset
pub fn store_asset_reward_distribution(e: &Env, asset: &Address, amount: i128) -> Result<i128, VaultError> {
    if !is_asset_supported(e, asset) {
        return Err(ValidationError::InvalidAddress.into());
    }
    
    let asset_total = get_asset_total_deposits(e, asset)?;
    let asset_reward_index = get_asset_reward_index(e, asset)?;
    
    let increment = checked_reward_index_increment(amount, asset_total)?;
    let next_reward_index = asset_reward_index
        .checked_add(increment)
        .ok_or(ArithmeticError::Overflow)?;

    set_asset_reward_index(e, asset, next_reward_index);

    Ok(next_reward_index)
}

/// Claim rewards for a specific asset
pub fn store_asset_claimable_rewards(e: &Env, user: &Address, asset: &Address) -> Result<i128, VaultError> {
    if !is_asset_supported(e, asset) {
        return Err(ValidationError::InvalidAddress.into());
    }
    
    let state = get_state(e)?;
    let mut position = get_user_asset_position_unchecked(e, user, asset);
    let asset_reward_index = get_asset_reward_index(e, asset)?;
    
    // Accrue all rewards earned up to the current global index.
    accrue_asset_position_rewards(e, asset_reward_index, &mut position)?;

    // Calculate vested rewards
    let current_timestamp = e.ledger().timestamp();
    let vested = calculate_vested_rewards(current_timestamp, &position, state.vesting_period)?;

    // Update position with remaining accrued rewards
    position.accrued_rewards = position
        .accrued_rewards
        .checked_sub(vested)
        .ok_or(ArithmeticError::Overflow)?;

    set_user_asset_position(e, user, asset, &position);

    Ok(vested)
}

/// Preview user rewards for a specific asset
pub fn preview_user_asset_rewards(e: &Env, user: &Address, asset: &Address) -> Result<UserRewardSnapshot, VaultError> {
    require_initialized(e)?;
    if !is_asset_supported(e, asset) {
        return Err(ValidationError::InvalidAddress.into());
    }
    
    let state = get_state(e)?;
    let mut position = get_user_asset_position_unchecked(e, user, asset);
    let asset_reward_index = get_asset_reward_index(e, asset)?;
    
    // Calculate accrued rewards without modifying state
    accrue_asset_position_rewards(e, asset_reward_index, &mut position)?;

    let current_timestamp = e.ledger().timestamp();
    let vested = calculate_vested_rewards(current_timestamp, &position, state.vesting_period)?;

    Ok(UserRewardSnapshot {
        reward_index: position.reward_index,
        rewards: position.accrued_rewards,
        vested_rewards: vested,
    })
}

pub fn pending_user_asset_rewards_view(e: &Env, user: &Address, asset: &Address) -> Result<i128, VaultError> {
    Ok(preview_user_asset_rewards(e, user, asset)?.rewards)
}

pub fn vested_user_asset_rewards_view(e: &Env, user: &Address, asset: &Address) -> Result<i128, VaultError> {
    Ok(preview_user_asset_rewards(e, user, asset)?.vested_rewards)
}

fn accrue_asset_position_rewards(
    e: &Env,
    asset_reward_index: i128,
    position: &mut UserPosition,
) -> Result<(), VaultError> {
    if asset_reward_index == position.reward_index || position.balance == 0 {
        position.reward_index = asset_reward_index;
        return Ok(());
    }

    if position.balance > 0 {
        let delta = asset_reward_index
            .checked_sub(position.reward_index)
            .ok_or(ArithmeticError::Overflow)?;
        let accrued = checked_accrued_rewards(position.balance, delta)?;

        if accrued > 0 {
            position.accrued_rewards = position
                .accrued_rewards
                .checked_add(accrued)
                .ok_or(ArithmeticError::Overflow)?;
            // Update last reward timestamp whenever new rewards are accrued
            position.last_reward_timestamp = e.ledger().timestamp();
        }
    }

    position.reward_index = asset_reward_index;
    Ok(())
}
