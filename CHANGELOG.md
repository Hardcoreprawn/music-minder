# Changelog

## [0.1.6](https://github.com/Hardcoreprawn/music-minder/compare/music-minder-v0.1.5...music-minder-v0.1.6) (2025-12-22)


### ✨ Features

* Add application icon ([a77f722](https://github.com/Hardcoreprawn/music-minder/commit/a77f722f9491f7d3c1a056ef93fd9fc5a9784c1a))
* **ui:** Add scan progress indicator with file count ([6d2c7fa](https://github.com/Hardcoreprawn/music-minder/commit/6d2c7faf6d2c74d2a79a3d216184fb76185fd92a))


### 🐛 Bug Fixes

* **ci:** Add manual workflow dispatch for installer builds ([16b9ba0](https://github.com/Hardcoreprawn/music-minder/commit/16b9ba0b7d243d5ef8a577be8908d6a0b454037b))
* **ci:** Remove duplicate Version variable in WiX build ([96af6db](https://github.com/Hardcoreprawn/music-minder/commit/96af6db716f7992966e5859feaf2cf1920db9377))
* **ci:** Use correct release-please component output names ([2cbc016](https://github.com/Hardcoreprawn/music-minder/commit/2cbc0164e462a3e54c6273df392bcee0cf0488c2))
* Display app icon in window title bar ([671aff2](https://github.com/Hardcoreprawn/music-minder/commit/671aff28f48fe1444bef1b542081ed56615bb852))
* **windows:** Hide console window when launching GUI ([69218cc](https://github.com/Hardcoreprawn/music-minder/commit/69218cc9e2ddcaa097d8f2d70949ed1ec15c92f8))
* **wix:** Move shortcuts to same feature as main executable ([0f66bd7](https://github.com/Hardcoreprawn/music-minder/commit/0f66bd751a62532024701a6ce5c4dafb5d219eaf))

## [0.1.5](https://github.com/Hardcoreprawn/music-minder/compare/music-minder-v0.1.4...music-minder-v0.1.5) (2025-12-20)


### ✨ Features

* **audio:** Add SIMD-accelerated audio processing ([a1941ec](https://github.com/Hardcoreprawn/music-minder/commit/a1941ec598e5fbffb19b88585a9a0e9799e19d27))
* **core:** Export new health modules and update main integration ([d3d984e](https://github.com/Hardcoreprawn/music-minder/commit/d3d984eb2dca581927c1a1941fe41749fe956da5))
* **db:** Add track matches and alternative releases storage ([a962b22](https://github.com/Hardcoreprawn/music-minder/commit/a962b223cfef2408479bf32f802081476d3b4d1f))
* **diagnostics:** add SIMD benchmark to system diagnostics ([424f535](https://github.com/Hardcoreprawn/music-minder/commit/424f535d8e2a778ee1761a8487f86feabf0d0b6a))
* **health:** Add Library Gardener for background quality maintenance ([cf14678](https://github.com/Hardcoreprawn/music-minder/commit/cf146788180447dc11e5f221d831788af88bcb86))
* **health:** Add metadata quality assessment system ([af5e822](https://github.com/Hardcoreprawn/music-minder/commit/af5e822bfc775eba334f4de3eb580662f7117af9))
* implement library search, filter, and sort ([3858208](https://github.com/Hardcoreprawn/music-minder/commit/3858208d6bcc69c90ba4fcffd348aecfadf5ee1a))
* Now Playing enhancements - queue position and file info ([2cffc0f](https://github.com/Hardcoreprawn/music-minder/commit/2cffc0f37d0f3ae4b9536c92c51c4e3c7386131e))
* **player:** add play_current() and improve command/event logging ([e3795c8](https://github.com/Hardcoreprawn/music-minder/commit/e3795c8bd944c92c56af0ed157e6fb8d5555f519))
* **queue:** add queue management UI controls ([f49830a](https://github.com/Hardcoreprawn/music-minder/commit/f49830af161f5ee97f6f3e46e501674e63eba37b))
* **scanner:** add background file watcher for incremental scanning ([6e40eae](https://github.com/Hardcoreprawn/music-minder/commit/6e40eaee0edc5bb71875dd556fb2fa94ff56c95b))
* **ui:** Add batch enrichment pane with results view ([21011ef](https://github.com/Hardcoreprawn/music-minder/commit/21011ef17f7b58b04e43a5f9ab3f386dcfde15d9))
* **ui:** Add centralized theme system with design tokens ([d19ef09](https://github.com/Hardcoreprawn/music-minder/commit/d19ef09cd8aba890d5cd20daa32a9f873a65b6e3))
* **ui:** Add GardenerState and quality-related messages ([41b7467](https://github.com/Hardcoreprawn/music-minder/commit/41b7467ef24b6519ac5270b141b64dc41ffccaf6))
* **ui:** add manual refresh button for library rescan ([556b7e2](https://github.com/Hardcoreprawn/music-minder/commit/556b7e2a46b026d299d09296560197d584950eb4))
* **ui:** Add organized Settings pane with sections ([0a05642](https://github.com/Hardcoreprawn/music-minder/commit/0a056426fb29b7ac4dec61d484a513071042bdf8))
* **ui:** improve seek slider with preview and release semantics ([7c00261](https://github.com/Hardcoreprawn/music-minder/commit/7c002610d44bef8abde40f8fc693ba17fa31e7e9))
* **ui:** Integrate gardener and quality updates in UI loop ([860ad30](https://github.com/Hardcoreprawn/music-minder/commit/860ad301cb0ae23a7baad389303a3a049e541159))
* update dependencies with defensive tests ([1ddb170](https://github.com/Hardcoreprawn/music-minder/commit/1ddb170d196b05e86f0d135e5ad7e60f301ddc30))


### 🐛 Bug Fixes

* **scripts:** correct pre-commit hook path and encoding ([507a984](https://github.com/Hardcoreprawn/music-minder/commit/507a984f6dbaae82784d97ffa49567b20aa0218d))
* **tests:** Fix test assertions for quality assessment ([346c9aa](https://github.com/Hardcoreprawn/music-minder/commit/346c9aa8aba9e560b5fc75155cf2c4c53f5f054b))
* **ui:** prevent player button layout shift ([2f722c0](https://github.com/Hardcoreprawn/music-minder/commit/2f722c03ef4c89c699594f2517ce1829157bda13))
* use -C flag to pass WiX preprocessor variable ([a3a2476](https://github.com/Hardcoreprawn/music-minder/commit/a3a247643ad479bd87b614c2692c4fedb8334288))
* **watcher:** migrate GUI subscription to async tokio::sync::mpsc ([15a2828](https://github.com/Hardcoreprawn/music-minder/commit/15a28287e03f79052c78b95fe9d47539b3ae3e3d))


### ⚡ Performance

* **ci:** optimize pipeline for faster runs ([ba7fda1](https://github.com/Hardcoreprawn/music-minder/commit/ba7fda18f6ed2fa772ac924a2d53d18166b05230))


### ♻️ Refactoring

* **cli:** Split commands module into focused submodules ([17aa4cd](https://github.com/Hardcoreprawn/music-minder/commit/17aa4cd86efbbc39fa9705ae73940a6a3e356b0e))
* **player:** event-driven state synchronization ([b450863](https://github.com/Hardcoreprawn/music-minder/commit/b450863bbc50b897ad7b28ea76b5f9a014881174))
* **ui:** consolidate subscriptions and improve player state sync ([747eb69](https://github.com/Hardcoreprawn/music-minder/commit/747eb691abc415d67185db3cad8c942f1c2c5548))
* **ui:** Split library pane into focused modules ([2c79ac8](https://github.com/Hardcoreprawn/music-minder/commit/2c79ac87ce2f85ad0df52bb4e610865a61b3c622))
* **ui:** Update views to use new theme system and modules ([c491fb3](https://github.com/Hardcoreprawn/music-minder/commit/c491fb3cdd9eeb5faa45536480c7685898c85dc5))


### 📚 Documentation

* add Winamp-inspired vision and CLI-first philosophy ([858c1f9](https://github.com/Hardcoreprawn/music-minder/commit/858c1f9f1fa9a59c7fa6f8435806e1ee81d4806a))
* clarify iced 0.14 Windows build issue in Cargo.toml comment ([81fee91](https://github.com/Hardcoreprawn/music-minder/commit/81fee91c5a75273e661d8cade470e6b1073130cf))
* update roadmap - 7.2 and 7.3 mostly complete ([9c646be](https://github.com/Hardcoreprawn/music-minder/commit/9c646be888c01a41d273291bb64341226144eac7))
* Update roadmap and add enrichment UI design document ([4d2083e](https://github.com/Hardcoreprawn/music-minder/commit/4d2083ee1494f9915477f40d267a65b18d2439a0))

## [0.1.4](https://github.com/Hardcoreprawn/music-minder/compare/music-minder-v0.1.3...music-minder-v0.1.4) (2025-12-09)


### ✨ Features

* add cargo-audit security scanning to CI ([0ebc610](https://github.com/Hardcoreprawn/music-minder/commit/0ebc610962c7cb0805c105f991d72b1f430d8348))


### 🐛 Bug Fixes

* quote WiX version argument for PowerShell ([bb75b94](https://github.com/Hardcoreprawn/music-minder/commit/bb75b94cd74f39b70ce8ec381b12b961765e0cd3))


### 📚 Documentation

* add code signing policy for SignPath ([2b0a0ae](https://github.com/Hardcoreprawn/music-minder/commit/2b0a0ae86d9c58294507a785336aed3415cdc126))

## [0.1.3](https://github.com/Hardcoreprawn/music-minder/compare/music-minder-v0.1.2...music-minder-v0.1.3) (2025-12-09)


### 🐛 Bug Fixes

* pass version to WiX installer from release-please ([68304f9](https://github.com/Hardcoreprawn/music-minder/commit/68304f9f7fde8e811806be8a2e67ad90e7f21f97))

## [0.1.2](https://github.com/Hardcoreprawn/music-minder/compare/music-minder-v0.1.1...music-minder-v0.1.2) (2025-12-09)


### 🐛 Bug Fixes

* allow unused imports in platform-specific test modules ([cf780f7](https://github.com/Hardcoreprawn/music-minder/commit/cf780f77c0f12d6cd2e2fb07cea977d394aca0d1))
* platform-specific PlatformConfig for souvlaki ([21e24df](https://github.com/Hardcoreprawn/music-minder/commit/21e24dfcb15ad6af7cddd9bc2178640ba3446d4d))

## [0.1.1](https://github.com/Hardcoreprawn/music-minder/compare/music-minder-v0.1.0...music-minder-v0.1.1) (2025-12-09)


### 🐛 Bug Fixes

* resolve formatting and clippy warnings for CI ([cfce9f0](https://github.com/Hardcoreprawn/music-minder/commit/cfce9f0b40f81d07ba44debaa59d8ae9dfd3a292))


### ♻️ Refactoring

* unify playback initiation with load_and_play_current() ([9830d06](https://github.com/Hardcoreprawn/music-minder/commit/9830d064fef848bb97cf519b69baa96bea731d68))


### 📚 Documentation

* add GitHub Pages site, README, LICENSE, and release workflow ([a329353](https://github.com/Hardcoreprawn/music-minder/commit/a329353346bb2082c0964f121b42476e0f1ad8b7))
