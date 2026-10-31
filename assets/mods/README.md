# Original mod artwork

Nyan Cat: https://github.com/NoahClemmensen/spicetify-nyan-bar-synced
Inspected Git tree SHA: 45b55071b392462f8a528b5b31514403c4a9f40a
src/resources/images/nyan_1.png through nyan_6.png are preserved byte for byte.
Original six-frame cycle runs once per beat (upstream default 116 BPM).

Cat Jam: https://github.com/BlafKing/spicetify-cat-jam-synced
Inspected Git tree SHA: e7bfd49fcc13457bbc98e696294cf5cf43eb6c31
Original asset: src/resources/catjam.webm (VP9 alpha, 325 frames, 9.75 seconds).
Original animation baseline: 135.48 BPM. Native speed is estimated song BPM / baseline.
Converted for the existing native PNG renderer, preserving all original frames and transparency.
Atlas: 16 columns x 21 rows, 112x86 pixels per frame. The final 11 tiles are unused.

Authoring command (FFmpeg is not required or distributed at runtime):
ffmpeg -c:v libvpx-vp9 -i catjam.webm -vf "scale=112:86:flags=lanczos,tile=16x21:color=black@0.0" -frames:v 1 -update 1 catjam-atlas.png

Neither inspected repository supplies an explicit artwork license notice. Their artwork is not covered by the MIT notices for other mods.

SHA256:
nyan_1.png: 533288FF80BE94D7FE10EB308C9A5BB41255BE019567DFBB696F0798A6D44ADF
nyan_2.png: 8AEA5AA2285BB25FF25884C1798796C117C483A23944A64817B936899101D933
nyan_3.png: B44D990E38AD28A5E20737EA302762A06229E259F448148A60E22A1046129E80
nyan_4.png: B61AC0AEB9F047A0A08968D4702397C26AC38A3D33B78DE876145017FE49E53D
nyan_5.png: 5738403070E64A1CE95ABF34F3242FEAE4D1701C0C0FFFFB115FA69800B99E22
nyan_6.png: D8ABF9D88CF90C1B25056FFA98E25923504511A94E176D6847DC4CC2A140AE20
catjam-atlas.png: 2095E280C6E15905C2109CC7C16A54EC19849108D24405B0CF2BD7FB6D200EF6
Original catjam.webm: 1098835930BA97C9998A299385F0566A29CC57941E01888A10AB6FD3F10196B4

Track Peek: native adaptation inspired by Grason Chan / Spotify Spice (MIT).
https://github.com/grasonchan/spotify-spice
Only adjacent-track cards and audio preview; no Vinyl feature or JavaScript runtime.
Original license retained in LICENSE_TRACK_PEEK.txt.
