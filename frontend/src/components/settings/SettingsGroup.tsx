// =============================================================================
// SettingsGroup — one collapsed block, and what it looks like open
// =============================================================================
//
// States 2 and 3 of the mockup. Closed, a block is one line with its count.
// Open, it is its rows — and past forty of them it grows a filter box pinned to
// the top of the list, because by then scrolling has taken the heading off
// screen and there is no way back to the top except scrolling.
//
// ## The count on the header is the SERVER's, the count beside the filter is not
//
// They answer different questions. `block.count` is how many stored rows this
// block holds, and it never changes as you type. "12 of 50 shown" is what the
// filter has left, and it is arithmetic over the rows in hand. Conflating them
// would make a filtered group look like a group that had lost rows.

import React, { useState } from "react";

import type { BlockDto, CoupledGroupDto, SettingDto } from "../../services/settings";
import CoupledListEditor from "./CoupledListEditor";
import SettingRow from "./SettingRow";
import { filterWithin, needsGroupFilter } from "./settingsSearch";
import {
  blockButtonStyle,
  blockCountStyle,
  ellipsisStyle,
  groupFilterInputStyle,
  groupFilterStyle,
} from "./settingsPageStyles";

export const SettingsGroup: React.FC<{
  block: BlockDto;
  /** Every stored row of this block, in the order the server sent them. */
  settings: readonly SettingDto[];
  /**
   * The coupled groups whose rows live in THIS block.
   *
   * Each renders as one editor in place of its rows — never alongside them, or
   * the page would offer two controls for one value and one of them would be
   * the single-row field the backend now refuses.
   */
  groups?: readonly CoupledGroupDto[];
  open: boolean;
  onToggle: () => void;
  onSaved: (message: string) => void;
}> = ({ block, settings, groups = [], open, onToggle, onSaved }) => {
  const [filter, setFilter] = useState("");

  if (!open) {
    return (
      <button
        type="button"
        style={blockButtonStyle}
        onClick={onToggle}
        data-block={block.id}
        data-count={block.count}
        aria-expanded={false}
      >
        <span>▸ {block.label}</span>
        <span style={blockCountStyle}>{block.count}</span>
      </button>
    );
  }

  const pinned = needsGroupFilter(settings.length);
  const shown = pinned ? filterWithin(settings, filter) : settings;

  return (
    <div data-block={block.id} data-count={block.count} data-open="yes">
      <button
        type="button"
        style={blockButtonStyle}
        onClick={onToggle}
        aria-expanded
      >
        <span>▾ {block.label}</span>
        <span style={blockCountStyle}>{block.count}</span>
      </button>

      {pinned && (
        <div style={groupFilterStyle} data-group-filter="yes">
          <span>Narrow this group:</span>
          <input
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder={`type to filter these ${settings.length}…`}
            aria-label={`Filter ${block.label}`}
            style={groupFilterInputStyle}
          />
          <span data-filter-count>
            {shown.length === settings.length
              ? `${settings.length} shown`
              : `${shown.length} of ${settings.length} shown`}
          </span>
        </div>
      )}

      {/* The editors come first: they are the block's coupled rows, and a
          control that writes several values belongs above the ones that write
          one. The in-group filter narrows the single rows only — an editor is
          one control, not a list of rows to search. */}
      {groups.map((group) => (
        <CoupledListEditor key={group.id} group={group} onSaved={onSaved} />
      ))}

      {shown.map((setting) => (
        <SettingRow
          key={setting.key}
          setting={setting}
          query={pinned ? filter : ""}
          onSaved={onSaved}
        />
      ))}

      {/* A filter that matched nothing says so. An empty list under a heading
          that still reads "50" is the page appearing to have lost the rows. */}
      {shown.length === 0 && (
        <div style={ellipsisStyle}>
          Nothing in {block.label} matches “{filter}”.
        </div>
      )}
    </div>
  );
};

export default SettingsGroup;
