# Spike: `_WKUserStyleSheet` on the shipped WebKit

Date: 2026-10-31. macOS 27.2 (Darwin 27.2.0), system WebKit.

The spike was a throwaway Swift program (not committed) that drove one
offscreen `WKWebView` with a `rice://` scheme handler serving two pages:
`rice://styled.test/` (`<body style="background: rgb(255,0,0)">`) and
`rice://plain.test/` (no styles). Sheets were created through the private
initialiser below and added with `_addUserStyleSheet:`; results were read
with `getComputedStyle`. A Swift program was used instead of a Rust example
because it needs no wry window plumbing and leaves nothing in the tree.

## Runtime selectors

| Selector | Present |
| --- | --- |
| class `_WKUserStyleSheet` | yes |
| `initWithSource:forWKWebView:forMainFrameOnly:includeMatchPatternStrings:excludeMatchPatternStrings:baseURL:level:contentWorld:` | yes |
| `initWithSource:forMainFrameOnly:` | yes |
| `WKUserContentController._addUserStyleSheet:` | yes |
| `WKUserContentController._removeUserStyleSheet:` | yes |
| `WKUserContentController._removeAllUserStyleSheets` | yes |

`level`: `0` user, `1` author.

## Behaviour

| Case | Result |
| --- | --- |
| Author sheet, `forWKWebView: nil`, pattern `rice://styled.test/*`, added in `decidePolicyForNavigationAction` for that navigation | applies to the new document |
| Same sheet on `rice://plain.test/` | does not apply |
| `_removeUserStyleSheet:` then reload | the page's own style returns |
| Author sheet added to a loaded page (no reload) | applies live |
| Pattern `*://styled.test/*` on a `rice:` page | does not apply: `*` is http/https only |
| Sheet created with `forWKWebView: <the view>` and added before `load` | does **not** apply to later documents |
| User-level `:root { background-color: Canvas; }` vs a page that styles only `body` | applies (root white, body red) |
| Author `!important` vs that user sheet | author wins |

## Verdict

**Use `_WKUserStyleSheet`**, created with `forWKWebView: nil` (page-scoped
sheets do not survive into the next document), author level, main frame
only, include pattern `*://<host>/*`, added from the engine's main-frame
navigation attempt hook (wry's `decidePolicyForNavigationAction`, which
WebKit also consults for server redirects). Removal and live changes work,
so changed switches can update open pages without a reload.

## Change from the spec: no Canvas backstop

The user-level `:root { background-color: Canvas; }` backstop of spec §2
paints the root on pages that style only `body`. Such a page's body
background then no longer propagates to the canvas, so a short dark page
shows a white band beneath its content. Instead, a page view keeps WebKit's
default background and drops it (`drawsBackground` off, clear under-page
colour) only while its main frame is on a host whose applied style includes
transparency (a site's transparency feature, or forced transparency). Pages
on other hosts are exactly as before.
