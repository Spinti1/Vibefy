# Vibefy Jam relay

Cloudflare Worker and SQLite Durable Object per room. Desktop and browser use the same TLS WebSocket protocol. No peer-to-peer sockets, audio files, account tokens or participant IP addresses are sent to other participants.

Rooms expire after six hours, hold at most 25 connections and a 100-song queue. A random 12-character invitation authorizes joining; a separate 256-bit credential authorizes host control. Share invitations only with intended listeners. Host controls playback; participants may suggest songs. Audio availability and browser autoplay/background restrictions still apply.

Cloudflare processes connection IP addresses. Application observability is disabled; rooms are removed on expiration or explicit host end. This is not end-to-end encryption: the relay processes song metadata and timing.

Deploy with `npx wrangler deploy` from this directory after `node --test tests/*.test.mjs`. No paid plan is automatically enabled. Monitor account quotas before a public launch. WebSocket hibernation avoids idle-duration billing; messages and storage still have quotas.
