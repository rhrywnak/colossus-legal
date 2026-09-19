// =============================================================================
// SettingsAreaRail — the left-hand list of areas, with the counts as served
// =============================================================================
//
// Every label and every number in this component comes off the wire. It sums
// nothing, orders nothing and names nothing: `areas` arrives in the order the
// page shows it, each entry already carrying how many stored rows it holds.
//
// The mockup's ten area counts were three out of date by the time this was
// built. That is not a mistake anyone made — the store moved — and it is the
// whole argument for a rail that cannot hold a number of its own.

import React from "react";

import type { AreaDto } from "../../services/settings";
import { railCountStyle, railItemStyle, railStyle } from "./settingsPageStyles";

/** The id of the landing view: no area selected. */
export const OVERVIEW_ID = "";

export const SettingsAreaRail: React.FC<{
  areas: readonly AreaDto[];
  selectedAreaId: string;
  onSelect: (areaId: string) => void;
}> = ({ areas, selectedAreaId, onSelect }) => (
  <nav style={railStyle} aria-label="Settings areas">
    <button
      type="button"
      style={railItemStyle(selectedAreaId === OVERVIEW_ID)}
      onClick={() => onSelect(OVERVIEW_ID)}
      data-area="overview"
      aria-current={selectedAreaId === OVERVIEW_ID ? "page" : undefined}
    >
      <span>Overview</span>
    </button>
    {areas.map((area) => (
      <button
        key={area.id}
        type="button"
        style={railItemStyle(selectedAreaId === area.id)}
        onClick={() => onSelect(area.id)}
        data-area={area.id}
        data-count={area.count}
        aria-current={selectedAreaId === area.id ? "page" : undefined}
      >
        <span>{area.label}</span>
        <span style={railCountStyle}>{area.count}</span>
      </button>
    ))}
  </nav>
);

export default SettingsAreaRail;
