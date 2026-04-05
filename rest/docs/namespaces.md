# API Namespaces

## Overview

The REST client provides 21 namespaced API surfaces. Each namespace groups related operations.

## Namespace Reference

### Fabric (AI and Communication Platform)

| Namespace | Access | Description |
|-----------|--------|-------------|
| `fabric().ai_agents()` | AI agent management | Create, update, delete, list AI agents |
| `fabric().addresses()` | Address management | SIP, phone, and agent addresses |
| `fabric().subscribers()` | Subscriber management | User and device subscriptions |
| `fabric().sip_endpoints()` | SIP endpoints | Manage SIP registrations |
| `fabric().phone_numbers()` | Number management | Assign numbers to resources |
| `fabric().conversations()` | Conversations | Multi-party messaging |
| `fabric().devices()` | Devices | Device registrations |
| `fabric().tokens()` | Tokens | Auth token generation |
| `fabric().policies()` | Policies | Access policies |
| `fabric().calls()` | Calls | Call history and management |
| `fabric().logs()` | Logs | Activity logs |
| `fabric().features()` | Features | Feature flags |
| `fabric().webhooks()` | Webhooks | Event webhooks |

### Calling

| Method | Description |
|--------|-------------|
| `calling().dial(params)` | Initiate outbound call |
| `calling().update(call_sid, params)` | Modify active call |
| `calling().list(params)` | List calls |
| `calling().get(call_sid)` | Get call details |
| `calling().recordings(call_sid)` | List call recordings |

### Messaging

| Method | Description |
|--------|-------------|
| `messaging().send(params)` | Send SMS/MMS |
| `messaging().list(params)` | List messages |
| `messaging().get(message_sid)` | Get message details |

### Phone Numbers

| Method | Description |
|--------|-------------|
| `phone_numbers().list(params)` | List owned numbers |
| `phone_numbers().search(params)` | Search available numbers |
| `phone_numbers().buy(params)` | Purchase a number |
| `phone_numbers().update(sid, params)` | Update number config |
| `phone_numbers().release(sid)` | Release a number |

### SIP

| Method | Description |
|--------|-------------|
| `sip().endpoints().list(params)` | List SIP endpoints |
| `sip().endpoints().create(params)` | Create SIP endpoint |
| `sip().endpoints().update(sid, params)` | Update endpoint |
| `sip().endpoints().delete(sid)` | Delete endpoint |
| `sip().domains().list(params)` | List SIP domains |

### Video

| Method | Description |
|--------|-------------|
| `video().rooms().list(params)` | List video rooms |
| `video().rooms().create(params)` | Create video room |
| `video().rooms().get(room_id)` | Get room details |
| `video().rooms().delete(room_id)` | Delete room |
| `video().recordings().list(params)` | List recordings |

### Datasphere

| Method | Description |
|--------|-------------|
| `datasphere().documents().search(params)` | Search documents |
| `datasphere().documents().upload(params)` | Upload document |
| `datasphere().documents().list(params)` | List documents |
| `datasphere().documents().delete(id)` | Delete document |

### Queues

| Method | Description |
|--------|-------------|
| `queues().list(params)` | List queues |
| `queues().create(params)` | Create queue |
| `queues().get(sid)` | Get queue details |
| `queues().members(sid)` | List queue members |

### Recordings

| Method | Description |
|--------|-------------|
| `recordings().list(params)` | List recordings |
| `recordings().get(sid)` | Get recording details |
| `recordings().delete(sid)` | Delete recording |

### Compat (Twilio-Compatible)

| Method | Description |
|--------|-------------|
| `compat().calls().create(params)` | Create call |
| `compat().calls().list(params)` | List calls |
| `compat().messages().create(params)` | Send message |
| `compat().messages().list(params)` | List messages |

### Additional Namespaces

| Namespace | Description |
|-----------|-------------|
| `fax()` | Fax operations |
| `conferences()` | Conference management |
| `transcriptions()` | Transcription operations |
| `applications()` | Application management |
| `usage()` | Usage and billing data |
