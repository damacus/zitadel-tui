# Changelog

## [0.1.6](https://github.com/damacus/zitadel-tui/compare/v0.1.5...v0.1.6) (2026-04-27)


### Bug Fixes

* **auth:** use userinfo for session status ([f2e46d1](https://github.com/damacus/zitadel-tui/commit/f2e46d115839d5ab965524c11c3b1c2c29966916))

## [0.1.5](https://github.com/damacus/zitadel-tui/compare/v0.1.4...v0.1.5) (2026-04-27)


### Bug Fixes

* **auth:** validate device login with userinfo ([a30740f](https://github.com/damacus/zitadel-tui/commit/a30740fee2a592c57fc3da2ee7f23967599c6fd4))

## [0.1.4](https://github.com/damacus/zitadel-tui/compare/v0.1.3...v0.1.4) (2026-04-27)


### Bug Fixes

* allow subcommands without once ([3625aba](https://github.com/damacus/zitadel-tui/commit/3625abab84e6b0bcc3e4f64e1954c25b8cefce7f))
* classify API app records ([#91](https://github.com/damacus/zitadel-tui/issues/91)) ([e10ac65](https://github.com/damacus/zitadel-tui/commit/e10ac6587e0d37852b99139bbc8fe61eade98e85))
* classify oidc apps without auth method ([444ac75](https://github.com/damacus/zitadel-tui/commit/444ac75fc8ec5992df76c0c2b3321e9d6283bc3e)), closes [#88](https://github.com/damacus/zitadel-tui/issues/88)
* handle machine user records ([#92](https://github.com/damacus/zitadel-tui/issues/92)) ([dbbfa8e](https://github.com/damacus/zitadel-tui/commit/dbbfa8e5289b759723838d8192a6e59febdf4257))
* hide deprecated once flag ([898cd85](https://github.com/damacus/zitadel-tui/commit/898cd8516cd430a98a299ca661f2de8a800af200))
* read service account auth status id ([#95](https://github.com/damacus/zitadel-tui/issues/95)) ([dd87e13](https://github.com/damacus/zitadel-tui/commit/dd87e131d18785e22157ecd43edb0ad1d34170f2))
* remember device login client id ([#85](https://github.com/damacus/zitadel-tui/issues/85)) ([dcb4972](https://github.com/damacus/zitadel-tui/commit/dcb497273ade344fa44566c5378329d3767611ec))
* show empty list feedback ([#93](https://github.com/damacus/zitadel-tui/issues/93)) ([06ec567](https://github.com/damacus/zitadel-tui/commit/06ec56746066ec7edc01cfa0efe497a95bfe952e))

## [0.1.3](https://github.com/damacus/zitadel-tui/compare/v0.1.2...v0.1.3) (2026-04-27)


### Features

* add responsive TUI shell ([#80](https://github.com/damacus/zitadel-tui/issues/80)) ([da6a905](https://github.com/damacus/zitadel-tui/commit/da6a905c6270fff11a71b08d817f43eac9a4efc6))
* OAuth2 Device Flow login/logout (auth login, auth logout) ([#63](https://github.com/damacus/zitadel-tui/issues/63)) ([4a974f9](https://github.com/damacus/zitadel-tui/commit/4a974f9639d951414593856ab1b3091513a870fa))

## [0.1.2](https://github.com/damacus/zitadel-tui/compare/v0.1.1...v0.1.2) (2026-04-02)


### Features

* add token cache and OIDC device flow modules ([#66](https://github.com/damacus/zitadel-tui/issues/66)) ([ec893af](https://github.com/damacus/zitadel-tui/commit/ec893af10b02259c0cbf619c372731318229e139))


### Bug Fixes

* **docs:** document all CLI options and correct config guidance ([#61](https://github.com/damacus/zitadel-tui/issues/61)) ([3db4ac9](https://github.com/damacus/zitadel-tui/commit/3db4ac9f90c99c33e0d57dd3ed7c7d96ce97f129))

## [0.1.1](https://github.com/damacus/zitadel-tui/compare/v0.1.0...v0.1.1) (2026-03-26)


### Features

* add GitHub workflows and make app public-ready ([9b90e12](https://github.com/damacus/zitadel-tui/commit/9b90e122fb6a46d925063dfeabf8367f991d28f3))
* add interactive command atelier workflows ([3efa7b0](https://github.com/damacus/zitadel-tui/commit/3efa7b0a9d602a875bddf8c1db6b94c9feaa633d))
* add YAML-based predefined users configuration ([b60dc8e](https://github.com/damacus/zitadel-tui/commit/b60dc8e23f83c28e2e9a66e6b81691f73a961617))
* bootstrap rust zitadel tui migration ([3ae1dbb](https://github.com/damacus/zitadel-tui/commit/3ae1dbb98cd47b1abe61f91c5804b3fe266fa7f3))


### Bug Fixes

* correct Ruby version requirement in README ([#7](https://github.com/damacus/zitadel-tui/issues/7)) ([3166e22](https://github.com/damacus/zitadel-tui/commit/3166e22e7a7a270729094e291cb1190f1529bd93))
* harden config and auth handling ([5bc0ecc](https://github.com/damacus/zitadel-tui/commit/5bc0ecc14af92d50ec37853065bbe018e2617cb0))


### Performance Improvements

* parallelize conductor refreshes ([75f4bbc](https://github.com/damacus/zitadel-tui/commit/75f4bbc1bfe86659c624ef054a709bcb8dd2a2e4))

## [1.0.1](https://github.com/damacus/zitadel-tui/compare/v1.0.0...v1.0.1) (2026-01-22)


### Bug Fixes

* correct Ruby version requirement in README ([#7](https://github.com/damacus/zitadel-tui/issues/7)) ([3166e22](https://github.com/damacus/zitadel-tui/commit/3166e22e7a7a270729094e291cb1190f1529bd93))

## 1.0.0 (2026-01-22)


### Features

* add GitHub workflows and make app public-ready ([9b90e12](https://github.com/damacus/zitadel-tui/commit/9b90e122fb6a46d925063dfeabf8367f991d28f3))
* add YAML-based predefined users configuration ([b60dc8e](https://github.com/damacus/zitadel-tui/commit/b60dc8e23f83c28e2e9a66e6b81691f73a961617))
