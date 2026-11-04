# Vibefy Windows signing

The Inno Setup (.iss) and WiX (.wxs) files describe installers. They do not contain a reusable Authenticode signature or permission to sign as their original publisher.

The upstream Spotifast documentation describes Azure Artifact Signing under PlentyLabs. Its GitHub OIDC trust is restricted to the upstream repository and environment. Vibefy currently has no authorized access to that signing account. Do not publish Vibefy with a claimed PlentyLabs signature or copy a signature from another executable.

Current status: Vibefy application and installer are unsigned. A SmartScreen reputation warning may appear. Do not disable Windows protection or advise users that a hash replaces code signing.

Before enabling signing:

1. Obtain a signing account/certificate with the verified publisher's authorization (or have its administrator explicitly authorize the Vibefy repository).
2. Sign the final application binary with SHA-256 and a trusted RFC3161 timestamp.
3. Package that exact signed binary, then sign the final installer.
4. Verify both Authenticode signatures, the expected publisher and timestamp. Fail the release if verification fails.
5. Only then generate the published SHA-256 digest and size; upload those exact bytes.

Valid code signing identifies the publisher but does not guarantee that a new release immediately has sufficient SmartScreen reputation.

References:
- https://learn.microsoft.com/windows/apps/package-and-deploy/smartscreen-reputation
- https://learn.microsoft.com/azure/artifact-signing/overview
