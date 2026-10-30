/** rice fork: whether the window-controls group (hover mode) is down. The
 *  sidebar's head makes room for it only while it is, so the column's
 *  content starts at the top otherwise and slides down with the group. */
let down = $state(false);

export const windowGroupDown = () => down;
export function setWindowGroupDown(value: boolean) {
  down = value;
}
