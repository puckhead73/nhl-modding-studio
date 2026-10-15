# chunklzx regression vectors (regenerated 2026-09-18 R1)

No game bytes are stored here (EA data policy): only hashes, offsets, lengths and framing counts. The
authoritative record is `manifest.json` (this table is a digest of it). Decoder identity (SHA-256 of the
probe files that produced the manifest):

- `chunklzx.py` `32b946066d275f43ec59ac082b748baf8559c5017565b4a0a61c577ec5d3f4f4`
- `xex_image.py` `59bebd1dde7ccaeec225d292a49d2b0d3dfb70831777a127dc6ee7aae344a552`
- `eb_list.py` `feb25edea28ce6f7ebcf6f880d4baafbd2c93aa4bee1dee0f319c9be839b655b`
- `chunklzx_vectors.py` `3b24d8ac5bf1016fe30207f8eae8f9c272687d35647270e332e2a20070ad308e`

LZX window bits: 17 (fixed). Generated: 2026-09-18T12:28:44.

## Archives (full SHA-256)

| title/archive | EB version | entries | size | sha256 |
|---|---|---|---|---|
| `nhl11/boot.big` | `0x00000400` | 266 | 8770576 | `1ff3c46a53c62e34eb8565b17da3d82c82fc478653db22f2f02788b056ffe51b` |
| `nhl11/rendercache.big` | `0x00000400` | 8401 | 921066528 | `400cc051b4572c777c248419f6722856481b555f78042665072f24828d089c7a` |
| `nhl12/boot.big` | `0x00000400` | 552 | 9791728 | `bdc313e0521bd4584a5f960a5c8f222d518269a577471c3300a4b6c065326541` |
| `nhl12/cacherender.big` | `0x00000400` | 12160 | 1246859632 | `1448b63dab5a4bf49b2aed0293de07315c271117094b3de9400ecf52655af0bc` |
| `nhl13/boot.big` | `0x00000400` | 592 | 10435888 | `120305ee1ed6ca1b0157466d749b16553860cb225d08fcac8050a92bf614ed9f` |
| `nhl13/cacherender.big` | `0x00000400` | 10969 | 1181076176 | `ed056dc3d7d7974b92e306ddcfa5a45f704edf23ff7073a3d22318f2cbf39439` |
| `nhl14/boot.big` | `0x00000400` | 599 | 11117888 | `5e212f5b9ca7a30b15e085937d41aeedc1a4b58b4ca135729a5493704f96f99f` |
| `nhl14/cacherender.big` | `0x00000400` | 11763 | 1282754880 | `8da1fb74b736ab212e33a9cf90b77237676a94e7ccce04739348b6d873c83052` |
| `nhl15/boot.big` | `0x00000400` | 599 | 11267408 | `b4bb1e6319f1ac7522b6e610d9cc63d8a448b7709928917e2dd59468dee6d681` |
| `nhl15/cacherender.big` | `0x00000400` | 11746 | 1284140496 | `859ae3b4738183fdc8d22abd835c1199804acc24c278b4b99c42d7aca3af90c1` |
| `nhllegacy/boot.big` | `0x00000400` | 599 | 11184432 | `6b4b88db2de4014d07875a9995abfc69a18ad4a2a80d6d96b981243a3d70a3dd` |
| `nhllegacy/cacherender.big` | `0x00000400` | 12845 | 1458552880 | `e05179835ff2099b01a3085c335f6b8d6cb8de52cd5cf7ecee23e948d586d6d2` |

## Vectors (all 50 decode byte-equal to the loose plaintext in the same title tree)

| title | archive | member | entry | offset | zsize | in sha256 (16) | chunks / sub-frames (full+last usize) | out len | out sha256 (16) | plaintext |
|---|---|---|---|---|---|---|---|---|---|---|
| nhl11 | boot.big | `attribdb/renddb.bin` | 116 | 5876192 | 21346 | `3c968c451a71d59c` | 1 / 7+14176 | 243552 | `b126f8a12eee7258` | EQUAL |
| nhl11 | boot.big | `attribdb/feattribdb.vlt` | 107 | 5856544 | 11372 | `635d2d5e918a738d` | 1 / 0+23360 | 23360 | `701da75f889778ca` | EQUAL |
| nhl11 | boot.big | `attribdb/crowds.vlt` | 105 | 5795008 | 59136 | `b36aa1cf272eca45` | 2 / 7+32768 + 2+29328 | 357008 | `f3bf83a944989698` | EQUAL |
| nhl11 | boot.big | `attribdb/crowds.bin` | 104 | 5789616 | 5378 | `69022acbfe42e475` | 1 / 1+10592 | 43360 | `4b5cbaed45b198f4` | EQUAL |
| nhl11 | boot.big | `attribdb/audioevents.vlt` | 103 | 5787712 | 1898 | `a33682779a9ac8d4` | 1 / 0+2784 | 2784 | `7c1ea129e45563d1` | EQUAL |
| nhl11 | rendercache.big | `rendering/logo/texlib_0_0_0.rx2` | 2009 | 210278944 | 7308 | `b70ac7953704c2a0` | 1 / 6+4096 | 200704 | `a146d626b70e2da8` | EQUAL |
| nhl11 | rendercache.big | `rendering/logo/texlib_27_0_3.rx2` | 2628 | 211776416 | 960 | `6cc6baf8a9e8c7c6` | 1 / 6+4096 | 200704 | `9e8b5373812bc625` | EQUAL |
| nhl11 | rendercache.big | `rendering/logo/texlib_27_0_0.rx2` | 2625 | 211763408 | 4136 | `eb5d99b3eab924d0` | 1 / 6+4096 | 200704 | `a85a31fa0e8f4f92` | EQUAL |
| nhl12 | boot.big | `attribdb/renddb.bin` | 134 | 4144592 | 25514 | `8e91a9bbae764496` | 2 / 7+32768 + 2+23920 | 351600 | `5a054e24d342a603` | EQUAL |
| nhl12 | boot.big | `attribdb/feattribdb.vlt` | 124 | 4125024 | 11302 | `fe1f5e4ddd63c936` | 1 / 0+23376 | 23376 | `fba3f42d918e4904` | EQUAL |
| nhl12 | boot.big | `attribdb/crowds.vlt` | 121 | 4063472 | 59134 | `003def493ff7b38d` | 2 / 7+32768 + 2+29344 | 357024 | `8fb5732ea947c793` | EQUAL |
| nhl12 | boot.big | `attribdb/crowds.bin` | 120 | 4058080 | 5392 | `4a626c7f79de9250` | 1 / 1+10608 | 43376 | `0cd3facc7ba3cf51` | EQUAL |
| nhl12 | boot.big | `attribdb/audioevents.vlt` | 119 | 4055888 | 2182 | `82fd24262dfcca45` | 1 / 0+3696 | 3696 | `a626d284e0f51584` | EQUAL |
| nhl12 | cacherender.big | `rendering/logo/texlib_0_0_0.rx2` | 4154 | 469575776 | 7290 | `7822ffd3d5c4b08a` | 1 / 6+4096 | 200704 | `7d8f3992363cad06` | EQUAL |
| nhl12 | cacherender.big | `rendering/logo/texlib_27_0_3.rx2` | 4911 | 471373760 | 958 | `a63f80b80442a203` | 1 / 6+4096 | 200704 | `dd4b7d4a29c2baf6` | EQUAL |
| nhl12 | cacherender.big | `rendering/logo/texlib_27_0_0.rx2` | 4908 | 471360672 | 4188 | `77eb0755ae047dca` | 1 / 6+4096 | 200704 | `0ffc95f812fbd018` | EQUAL |
| nhl13 | boot.big | `attribdb/renddb.bin` | 143 | 6311904 | 23434 | `b3746729dde4928a` | 1 / 2+28128 | 93664 | `adccab5db62d212f` | EQUAL |
| nhl13 | boot.big | `attribdb/feattribdb.vlt` | 135 | 6292080 | 10870 | `ffd3943a620f07f7` | 1 / 0+22336 | 22336 | `1418b4f23c7d2eaf` | EQUAL |
| nhl13 | boot.big | `attribdb/crowds.vlt` | 132 | 6230512 | 59270 | `4a3bfb12eb532a9b` | 2 / 7+32768 + 2+29824 | 357504 | `5b5c182ef19bc4ef` | EQUAL |
| nhl13 | boot.big | `attribdb/crowds.bin` | 131 | 6225104 | 5394 | `4eab16a4bcca01b6` | 1 / 1+10608 | 43376 | `43a920d0619af8f1` | EQUAL |
| nhl13 | boot.big | `attribdb/audioevents.vlt` | 130 | 6222848 | 2242 | `4d50bdbb377c6d5e` | 1 / 0+4064 | 4064 | `ea58f6e631f2b7e4` | EQUAL |
| nhl13 | cacherender.big | `rendering/logo/texlib_0_0_0.rx2` | 4483 | 596609488 | 8886 | `18917a0a033255ac` | 1 / 6+4096 | 200704 | `7b5f27ea479aa4b9` | EQUAL |
| nhl13 | cacherender.big | `rendering/logo/texlib_27_0_3.rx2` | 5243 | 598777936 | 960 | `f543a0a49c858ff1` | 1 / 6+4096 | 200704 | `c7ba030d47e1ed71` | EQUAL |
| nhl13 | cacherender.big | `rendering/logo/texlib_27_0_0.rx2` | 5240 | 598761136 | 5050 | `35a39293f78df8f4` | 1 / 6+4096 | 200704 | `e26d4fc6508d3fef` | EQUAL |
| nhl14 | boot.big | `attribdb/renddb.bin` | 145 | 6595344 | 24048 | `0380661d64596d15` | 1 / 3+5488 | 103792 | `dc01966c28daf3bb` | EQUAL |
| nhl14 | boot.big | `attribdb/feattribdb.vlt` | 137 | 6575520 | 10870 | `fee3bad00658b887` | 1 / 0+22336 | 22336 | `5bc374bf6c620ec7` | EQUAL |
| nhl14 | boot.big | `attribdb/crowds.vlt` | 134 | 6512464 | 60768 | `721290d2c0105061` | 2 / 7+32768 + 3+704 | 361152 | `faafb52a02f83395` | EQUAL |
| nhl14 | boot.big | `attribdb/crowds.bin` | 133 | 6507024 | 5440 | `635420e44767fcd5` | 1 / 1+10896 | 43664 | `24064c5c2af8d93f` | EQUAL |
| nhl14 | boot.big | `attribdb/audioevents.vlt` | 132 | 6504768 | 2242 | `4d50bdbb377c6d5e` | 1 / 0+4064 | 4064 | `ea58f6e631f2b7e4` | EQUAL |
| nhl14 | cacherender.big | `rendering/logo/texlib_0_0_0.rx2` | 5397 | 670910608 | 8890 | `e1ebb6097dcb5b67` | 1 / 6+4096 | 200704 | `f4d91640a2d240d6` | EQUAL |
| nhl14 | cacherender.big | `rendering/logo/texlib_27_0_3.rx2` | 6166 | 673127328 | 960 | `f543a0a49c858ff1` | 1 / 6+4096 | 200704 | `c7ba030d47e1ed71` | EQUAL |
| nhl14 | cacherender.big | `rendering/logo/texlib_27_0_0.rx2` | 6163 | 673110528 | 5050 | `35a39293f78df8f4` | 1 / 6+4096 | 200704 | `e26d4fc6508d3fef` | EQUAL |
| nhl15 | boot.big | `attribdb/renddb.bin` | 145 | 6580704 | 24050 | `eda9e1df4eddfdfb` | 1 / 3+5488 | 103792 | `f64330f34f6b71c3` | EQUAL |
| nhl15 | boot.big | `attribdb/feattribdb.vlt` | 137 | 6560880 | 10868 | `5f1c9660764bc2a9` | 1 / 0+22336 | 22336 | `e1ad72be02cd1ee0` | EQUAL |
| nhl15 | boot.big | `attribdb/crowds.vlt` | 134 | 6497824 | 60768 | `721290d2c0105061` | 2 / 7+32768 + 3+704 | 361152 | `faafb52a02f83395` | EQUAL |
| nhl15 | boot.big | `attribdb/crowds.bin` | 133 | 6492384 | 5440 | `635420e44767fcd5` | 1 / 1+10896 | 43664 | `24064c5c2af8d93f` | EQUAL |
| nhl15 | boot.big | `attribdb/audioevents.vlt` | 132 | 6490128 | 2254 | `935bb5a1f042a332` | 1 / 0+4080 | 4080 | `9ce1200bf16e7d0a` | EQUAL |
| nhl15 | cacherender.big | `rendering/logo/texlib_0_0_0.rx2` | 5917 | 711238768 | 8890 | `e1ebb6097dcb5b67` | 1 / 6+4096 | 200704 | `f4d91640a2d240d6` | EQUAL |
| nhl15 | cacherender.big | `rendering/logo/texlib_27_0_3.rx2` | 6704 | 713631984 | 960 | `f543a0a49c858ff1` | 1 / 6+4096 | 200704 | `c7ba030d47e1ed71` | EQUAL |
| nhl15 | cacherender.big | `rendering/logo/texlib_27_0_0.rx2` | 6701 | 713615184 | 5050 | `35a39293f78df8f4` | 1 / 6+4096 | 200704 | `e26d4fc6508d3fef` | EQUAL |
| nhl15 | cacherender.big | `rendering/logo/texlib_27_1_6.rx2` | 6713 | 713668192 | 5908 | `054861e09591d73b` | 1 / 6+4096 | 200704 | `e6e012074e5f86ac` | EQUAL |
| nhllegacy | boot.big | `attribdb/renddb.bin` | 145 | 6580704 | 24050 | `eda9e1df4eddfdfb` | 1 / 3+5488 | 103792 | `f64330f34f6b71c3` | EQUAL |
| nhllegacy | boot.big | `attribdb/feattribdb.vlt` | 137 | 6560880 | 10868 | `5f1c9660764bc2a9` | 1 / 0+22336 | 22336 | `e1ad72be02cd1ee0` | EQUAL |
| nhllegacy | boot.big | `attribdb/crowds.vlt` | 134 | 6497824 | 60768 | `721290d2c0105061` | 2 / 7+32768 + 3+704 | 361152 | `faafb52a02f83395` | EQUAL |
| nhllegacy | boot.big | `attribdb/crowds.bin` | 133 | 6492384 | 5440 | `635420e44767fcd5` | 1 / 1+10896 | 43664 | `24064c5c2af8d93f` | EQUAL |
| nhllegacy | boot.big | `attribdb/audioevents.vlt` | 132 | 6490128 | 2254 | `935bb5a1f042a332` | 1 / 0+4080 | 4080 | `9ce1200bf16e7d0a` | EQUAL |
| nhllegacy | cacherender.big | `rendering/logo/texlib_0_0_0.rx2` | 6237 | 740334112 | 8886 | `18917a0a033255ac` | 1 / 6+4096 | 200704 | `7b5f27ea479aa4b9` | EQUAL |
| nhllegacy | cacherender.big | `rendering/logo/texlib_27_0_3.rx2` | 7095 | 743005232 | 960 | `f543a0a49c858ff1` | 1 / 6+4096 | 200704 | `c7ba030d47e1ed71` | EQUAL |
| nhllegacy | cacherender.big | `rendering/logo/texlib_27_0_0.rx2` | 7092 | 742988432 | 5050 | `35a39293f78df8f4` | 1 / 6+4096 | 200704 | `e26d4fc6508d3fef` | EQUAL |
| nhllegacy | cacherender.big | `rendering/logo/texlib_27_1_6.rx2` | 7104 | 743041440 | 5902 | `bdedaabb1cc58ce8` | 1 / 6+4096 | 200704 | `e570ade818f5223d` | EQUAL |

Reproduce: `python probes/eb_list.py "<archive>" --extract <member> m.bin && python probes/chunklzx.py m.bin out.bin` → compare
`sha256(out.bin)` with `output_sha256`; or `python probes/chunklzx_vectors.py probes/chunklzx_vectors.spec.json out.json --archive-hashes`
to regenerate everything (~3 min, read-only).
