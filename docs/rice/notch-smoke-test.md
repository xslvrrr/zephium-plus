# Notch look smoke test (owner checklist)

Needs eyes and a pointer, so it is not automated. Run it on the dev build
(`pnpm dev` from `desktop/`) after any change to the notch, the reach zones,
the chrome regions or the window controls. Default settings: `ui.toolbar=top`,
`ui.window-controls=hover`, macOS.

## Core checks

1. New tab: the notch is down with the address field in it, there is no
   wallpaper, and the texture shows behind the page.
2. Web page: the notch is hidden. Move to the top-centre edge and it slides
   down. Move 30 px below it and it stays. Move away and it slides up.
3. With the notch down, click and scroll the page to the notch's left and
   right: the page responds.
4. Cmd+L on a page: the notch comes down with the field focused. Escape, or
   clicking the page, puts it away.
5. The address field's site menu opens below the notch and stays usable.
6. Hover mode: move to the top-left corner. The traffic lights and the sidebar
   button slide down together. The button hides and shows the sidebar. Moving
   away slides them up. With them up, clicking where they were reaches the
   page.
7. Hidden sidebar: the left-edge peek still works and the page to its right
   stays clickable.
8. Reduce Motion on: every slide is instant.
9. Sidebar mode (`ui.toolbar=sidebar`): upstream's look, with the address in
   the sidebar.

## Runtime-only checks (flagged during review)

10. History menu hold: with the notch down, press and hold the back or
    forward button until the history menu opens. The notch stays down while
    the menu is open. (The native menu gives no close signal, so also check
    the notch goes up once you dismiss it.) Do the same for the "…" sidebar
    menu: expect the notch may not be held while it is open.
11. Traffic lights after a resize: in hover mode with the lights up, resize
    the window by dragging an edge. The lights must not flash or pop into
    view. Also resize while they are sliding up.
12. Peek and lights: in hover mode, peek the hidden sidebar. The lights stay
    where the group put them (not 8 pt left), and the group's button still
    toggles the sidebar. Switching from shown to hover mid-peek may leave an
    8 pt offset until the next resize: note it if you see it.
13. Texture corners and opacity: the texture reaches the window's rounded
    corners with no square edge or gap, and the strength follows Settings →
    Appearance → texture strength (default 15) in light and dark.
14. Reduce Transparency: turn it on, then off again. The texture backdrop
    returns without needing a window refresh.
15. New-tab text contrast: on a new tab, the clock and search text read
    clearly over the texture in light and dark, and on both a dark and a light
    texture.
16. Solid tier: with the Solid material, the gutter beside the page keeps its
    ground (it is not left transparent) while the notch is down.
17. Find in page (Cmd+F) with the notch up: the find bar works. In notch mode
    it may briefly widen a compact sidebar.
18. Peek mid-resize: resize the window during a peek of the hidden sidebar.
    The card should follow the window; report it if it does not.
