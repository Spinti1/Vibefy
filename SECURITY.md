# Security

Do not put passwords, refresh tokens, cookies, private playlists or personal logs in public issues. Report vulnerabilities through GitHub's private vulnerability reporting for this repository when enabled. If it is unavailable, open a minimal issue requesting a private reporting channel without exploit details or secrets.

## Desktop updater

Only `Spinti1/Vibefy` releases over GitHub HTTPS are accepted. Downloads require GitHub's published asset size and SHA-256 digest, plus the exact checksum in `checksums.txt`. Missing digests, changed bytes, truncated/oversized assets and untrusted redirect hosts fail closed. The original Spotifast release signing key is not trusted by Vibefy. Installers are **not code-signed**. This protects download integrity, not a compromised GitHub publisher account.

The existing fastframe helper waits for the running application to close, launches Setup in the current installation directory, and supports rollback receipts. Normal shutdown saves state and flushes pending writes before stopping the backend. Settings and credentials are not shipped in release files. The first setup installation adds the Vibefy installer marker required for future in-place updates.

## Web beta

No Spotify password, desktop token, browser cookie or arbitrary source URL is accepted by the API. Public recording IDs and playlist IDs are validated. Audio extraction can only resolve YouTube IDs, then proxy HTTPS `*.googlevideo.com` ranges. Redirects are not followed. Each response is bounded to 2 MiB; invalid/multiple ranges are rejected. Music-video and unclassified playlist entries are omitted. Metadata is inserted with DOM text APIs, not HTML. Backups are size-limited and validated. API responses do not disclose stack traces or extractor URLs.

The public Vercel deployment uses the official visible YouTube iframe because YouTube refused server extraction. It does not bypass provider challenges or use personal cookies. The direct audio proxy is disabled by default on Vercel. The deployment adds a restrictive CSP, frame denial, MIME-sniffing protection and disabled microphone/camera/geolocation access. Service workers cache only the app shell, never API responses, audio or credentials. Saved libraries are local to each browser. Browser audio / background playback remains subject to OS policy.

Per-instance request throttles, a four-worker bound and request deadlines limit bursts. They are **not a distributed abuse-prevention system**. Before broad promotion, configure Vercel Firewall rate limits and monitor usage. The Hobby deployment has no durable shared session store or mobile Jam relay. No paid service is provisioned by this source tree.

## Validation scope

Focused tests exercise update corruption, size mismatch, digest absence, unsafe redirects, invalid source IDs, SSRF-shaped inputs, video filtering, metadata rendering and API limits. This is a scoped hardening review, not a guarantee that all exploits have been eliminated. Legacy upstream test harness failures are documented separately; do not confuse them with a fully passing cross-platform audit.
