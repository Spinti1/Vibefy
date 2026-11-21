# SideStore update errors

`SideSign.ServerError 3` alone is not a specific Vibefy package error. SideSign's ServerError enum includes an underlying Apple server rejection; the full message and SideStore diagnostic log are needed to identify the rejection.

The published Vibefy 0.2.8 IPA was checked for archive integrity, arm64 executable, unchanged bundle identity, matching source privacy permissions, absence of app extensions and absence of an embedded provisioning profile or preexisting code signature. No paid-only signing entitlement is bundled.

1. Keep Vibefy installed. Do not delete it as a troubleshooting step: local playlist data may be lost.
2. Update SideStore from its official releases if your build is outdated. Retry after signing into SideStore with the same Apple account.
3. If the source update still fails, download the existing Vibefy IPA and try a manual update through SideStore or iLoader, retaining the same Apple account and bundle identity. Do not choose a separate install or rename its bundle identifier. This does not guarantee that an Apple account/server rejection will disappear.

- IPA: https://github.com/Spinti1/Vibefy/releases/download/mobile-v0.2.8/Vibefy-0.2.8.ipa
- Official SideStore guidance: https://docs.sidestore.io/docs/troubleshooting
- Similar SideSign error report: https://github.com/SideStore/SideStore/issues/1455
- Error definitions: https://github.com/SideStore/SideSign/blob/main/Sources/Models/Errors.swift
