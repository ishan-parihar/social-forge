# AGENTS.md - Knowledge Base

## Environment & Test Identity
To ensure consistent testing and avoid context loss regarding connected accounts, use the following credentials and identity:

### Database Configuration
- **PostgreSQL Container**: `social-forge-postgres-1`
- **DB Name**: `postiz`
- **User/Password**: `postiz` / `postiz`
- **Internal Connection**: `postgres://postiz:postiz@172.21.0.2:5432/postiz`
- **Host Connection**: `postgres://postiz:postiz@localhost:5432/postiz`

### Golden Test User (The "Primary" Identity)
Use this user for all tool verification as they have the most connected channels.
- **User ID**: `87c12961-11e0-47b9-8788-efe46b2acacc`

#### Connected Social Channels
| Provider | Internal ID / Page ID |
|---|---|
| Facebook | `4372074126446140` |
| Facebook | `604373986102944` |
| Facebook | `338858752654432` |
| Facebook | `106249392449992` |
| Facebook | `102729826251641` |
| Instagram | `17841400680408909` |
| Instagram | `17841401924712730` |
| Instagram | `4372074126446140` |
| Instagram | `17841474734070627` |
| Instagram | `17841461291118404` |

**Verification Rule**: Always verify the `integrations` table for this user before assuming a page is "not connected".
