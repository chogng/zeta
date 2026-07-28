# Third-party notices

Zeta Desktop directly distributes the following third-party components.
Release packaging must include this file and the referenced license and notice
texts.

| Component | Version | License used by Zeta | License text |
| --- | --- | --- | --- |
| [DOMPurify](https://github.com/cure53/DOMPurify) | 3.4.12 | Apache-2.0, selected from `MPL-2.0 OR Apache-2.0` | [`licenses/DOMPurify.txt`](licenses/DOMPurify.txt) |
| [Marked](https://github.com/markedjs/marked) | 18.0.7 | MIT and bundled Markdown notice | [`licenses/Marked.txt`](licenses/Marked.txt) |
| [markdown-it](https://github.com/markdown-it/markdown-it) | 14.3.0 | MIT | [`licenses/markdown-it.txt`](licenses/markdown-it.txt) |
| `@chogng/lxicons` | 1.0.21 | MIT | [`licenses/lxicons.txt`](licenses/lxicons.txt) |
| [Monaco Editor](https://github.com/microsoft/monaco-editor) | 0.56.0 | MIT and bundled third-party notices | [`licenses/Monaco-Editor.txt`](licenses/Monaco-Editor.txt), [`licenses/Monaco-Editor-ThirdPartyNotices.txt`](licenses/Monaco-Editor-ThirdPartyNotices.txt) |
| [ProseMirror](https://prosemirror.net/) (`commands` 1.7.1, `history` 1.5.0, `keymap` 1.2.3, `model` 1.25.11, `schema-basic` 1.2.4, `schema-list` 1.5.1, `state` 1.4.4, `view` 1.42.2) | package versions listed at left | MIT | [`licenses/ProseMirror.txt`](licenses/ProseMirror.txt) |
| [Typst](https://github.com/typst/typst) | 0.15.1 | Apache-2.0; includes separately attributed third-party material | [`licenses/Typst.txt`](licenses/Typst.txt), [`licenses/Typst-NOTICE.txt`](licenses/Typst-NOTICE.txt) |
| [typst-assets bundled fonts](https://github.com/typst/typst-assets) | 0.15.1 | Apache-2.0 plus the font and asset licenses enumerated upstream | [`licenses/Typst.txt`](licenses/Typst.txt), [`licenses/Typst-Assets-NOTICE.txt`](licenses/Typst-Assets-NOTICE.txt) |

Transitive dependencies and third-party material distributed by Electron or
native runtimes remain governed by their own accompanying notices. Before a
release, the packaging pipeline must retain those upstream notices and verify
that this direct-dependency list still matches `desktop/package.json` and the
Rust components linked into the shipped `zeta` executable.
