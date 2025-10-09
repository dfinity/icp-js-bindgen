## Unreleased

## 0.1.1 (2025-10-09)

## 0.1.0 (2025-10-08)

### Feat

- remove `processError` and redundant `private actor` class property (#70)
- remove additional features (#67)
- overwrite output only with `force` (#38)
- output package version in generated bindings (#33)
- merge the service wrapper file and the index file (#20)
- `output` option (#16)
- `interfaceDeclaration` flag (#11)
- port bindgen code from candid_parser (#1)
- use constants from vite build
- additional features
- log during watch
- cli command
- vite plugin
- initial lib files

### Fix

- remove unneeded unions on typed arrays (#74)
- add space at the end of the doc comment (#63)
- doc comment above the variant field (#61)
- improve logs (#55)
- use interface in type declarations (#51)
- ignore inline comments or separated by newlines  (#46)
- print error message in cli (#36)
- use constants directly in files
- pass additional features from plugin

### Refactor

- explain code porting
- codestyle
- format with biome
- out dir param
