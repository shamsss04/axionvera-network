# AxionVault Events Documentation

This document describes the event system for AxionVault, including event types, schema, and how to consume events for analytics and dashboards.

## Event Types

The contract emits the following event types:

| Event Type | Description | Topics |
|------------|-------------|--------|
| `Initialize` | Emitted when the vault is initialized | (AxionVault, Initialize) |
| `Deposit` | Emitted when a user deposits funds | (AxionVault, Deposit) |
| `Withdraw` | Emitted when a user withdraws funds | (AxionVault, Withdraw) |
| `Distribute` | Emitted when rewards are distributed | (AxionVault, Distribute) |
| `Claim` | Emitted when a user claims rewards | (AxionVault, Claim) |
| `Lock` | Emitted when funds are locked | (AxionVault, Lock) |
| `Unlock` | Emitted when funds are unlocked | (AxionVault, Unlock) |
| `AdminProp` | Emitted when an admin transfer is proposed | (AdminProp) |
| `AdminAcpt` | Emitted when an admin transfer is accepted | (AdminAcpt) |
| `Upgrade` | Emitted when the contract is upgraded | (Upgrade) |
| `AssetAdd` | Emitted when a new asset is added | (AssetAdd) |
| `AssetDep` | Emitted when a user deposits an asset | (AxionVault, AssetDep) |
| `AssetWith` | Emitted when a user withdraws an asset | (AxionVault, AssetWith) |
| `AssetDist` | Emitted when asset rewards are distributed | (AxionVault, AssetDist) |
| `AssetClm` | Emitted when a user claims asset rewards | (AxionVault, AssetClm) |

## Event Schema

All events share a common structure stored in the database:

```json
{
  "id": "event-unique-id",
  "ledger": 123456,
  "contract_id": "contract-address",
  "topic": ["AxionVault", "Deposit"],
  "value": {
    "xdr": "base64-encoded-xdr"
  }
}
```

## Database Schema

Events are stored in the `events` table with the following columns:

| Column | Type | Description |
|--------|------|-------------|
| `id` | SERIAL | Primary key |
| `event_id` | TEXT | Unique event ID |
| `ledger_sequence` | INTEGER | Stellar ledger number |
| `contract_id` | TEXT | Contract address |
| `event_type` | TEXT | Event type |
| `protocol` | TEXT | Protocol identifier (usually "AxionVault") |
| `action` | TEXT | Action type |
| `user_address` | TEXT | User address (if applicable) |
| `asset_address` | TEXT | Asset address (if applicable) |
| `amount` | NUMERIC | Amount (if applicable) |
| `timestamp` | BIGINT | Unix timestamp from the event |
| `data` | JSONB | Full event payload as JSON |
| `created_at` | TIMESTAMPTZ | When the event was indexed |

## Querying Events

### Query All Events

```sql
SELECT * FROM events ORDER BY timestamp DESC LIMIT 100;
```

### Query Events by User

```sql
SELECT * FROM events WHERE user_address = 'GABC...XYZ' ORDER BY timestamp DESC;
```

### Query Events by Type

```sql
SELECT * FROM events WHERE event_type = 'Deposit' ORDER BY timestamp DESC;
```

### Query Events by Date Range

```sql
SELECT * FROM events WHERE timestamp >= 1710000000 AND timestamp <= 1720000000;
```

## Indexer API

The indexer runs in the network node and:
1. Polls the Soroban RPC for new events every 5 seconds
2. Stores events in the PostgreSQL database
3. Tracks progress in the `indexer_state` table
4. Provides idempotent processing (events are not duplicated)

## Future Enhancements

Planned improvements:
- Parse XDR event values into structured JSON
- Add more specific query endpoints (gRPC and HTTP)
- Add WebSocket subscriptions for real-time events
- Add event analytics dashboards
